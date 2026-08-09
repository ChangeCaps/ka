use std::{
    collections::{HashMap, HashSet, VecDeque},
    iter, mem,
};

use crate::{
    arena::{Arena, Id},
    ast,
    diagnostic::{Diagnostic, Emitter, Span},
    ir::{Alias, Bounds, Extern, Global, Import, Pat, Program, Scope, ScopeKind, Ty, Var, VarKind},
    lower::ty::Generics,
};

mod def;
mod exhaust;
mod expr;
mod infer;
mod intrinsic;
mod pat;
mod prelude;
mod scope;
mod ty;

pub struct Lowerer<'a> {
    emitter: &'a mut dyn Emitter,

    imports: HashMap<&'static str, Id<Scope>>,
    modules: Vec<(Id<Scope>, Box<[ast::Def]>)>,

    externs: Arena<Extern>,
    globals: Arena<Global>,

    aliases: Arena<Alias>,
    scopes: Arena<Scope>,
    vars: Arena<Var>,

    bounds: Arena<Bounds>,
    subst: HashMap<Id<Bounds>, Ty>,
    cache: HashSet<u64>,

    prelude: Id<Scope>,
    dependencies: HashMap<Id<Global>, Dependencies>,
}

type Dependencies = HashMap<Id<Global>, Vec<(Ty, Span)>>;
type LetDefState<'a> = (Id<Global>, Id<Scope>, Pat, &'a ast::LetDef);

impl<'a> Lowerer<'a> {
    pub fn new(emitter: &'a mut dyn Emitter) -> Self {
        let mut scopes = Arena::new();

        let prelude = Scope::new(ScopeKind::Module, None);
        let prelude = scopes.add(prelude);

        Self {
            emitter,

            imports: HashMap::new(),
            modules: Vec::new(),

            externs: Arena::new(),
            globals: Arena::new(),

            aliases: Arena::new(),
            vars: Arena::new(),

            bounds: Arena::new(),
            subst: HashMap::new(),
            cache: HashSet::new(),

            dependencies: HashMap::new(),

            scopes,
            prelude,
        }
    }

    pub fn add_module(&mut self, name: &'static str, defs: Box<[ast::Def]>) -> Id<Scope> {
        let scope = self.add_scope(ScopeKind::Module, self.prelude);

        self.imports.insert(name, scope);
        self.modules.push((scope, defs));

        scope
    }

    pub fn finish(mut self, main_module: &str) -> (Program, Option<Id<Var>>) {
        self.add_prelude();
        self.add_intrinsics();

        let modules = mem::take(&mut self.modules);

        for (scope, defs) in &modules {
            self.import_defs(*scope, defs);
        }

        self.aliases(modules.iter().flat_map(|(scope, defs)| {
            iter::repeat(*scope).zip(defs.iter().filter_map(ast::Def::as_alias))
        }));

        for (scope, defs) in &modules {
            self.extern_defs(*scope, defs);
        }

        let mut let_defs = Vec::new();

        for (scope, defs) in &modules {
            let defs = self.register_let_defs(*scope, defs);
            let_defs.extend(defs);
        }

        self.complete_let_defs(let_defs);

        let order = self.resolve_global_dependencies();

        let main = self
            .imports
            .get(main_module)
            .and_then(|scope| self.find_var(*scope, "main"));

        if let Some(main) = main {
            let ty = self.vars[main].ty.clone();
            let span = self.vars[main].span;

            self.unify(&ty, &Ty::Monad(Box::new(Ty::UNIT)), span);
        } else {
            let diagnostic = Diagnostic::error("`main` not defined");
            self.emitter.emit(diagnostic);
        }

        let program = Program {
            externs: self.externs,
            globals: self.globals,
            order,

            scopes: self.scopes,
            vars: self.vars,

            bounds: self.bounds,
            subst: self.subst,
        };

        (program, main)
    }

    pub(super) fn import_defs<'b>(
        &mut self,
        scope: Id<Scope>,
        defs: impl IntoIterator<Item = &'b ast::Def>,
    ) {
        for def in defs {
            let ast::Def::Import(def) = def else {
                continue;
            };

            let Some(import_scope) = self.imports.get(def.path).copied() else {
                let diagnostic = Diagnostic::error(format!("invalid import path `{}`", def.path))
                    .with_label(def.span, "in import found here");

                self.emitter.emit(diagnostic);

                continue;
            };

            self.scopes[scope].imports.push(Import {
                name: def.name,
                scope: import_scope,
            });
        }
    }

    pub(super) fn alias_defs<'b>(
        &mut self,
        scope: Id<Scope>,
        defs: impl IntoIterator<Item = &'b ast::Def>,
    ) {
        let defs = defs.into_iter().filter_map(ast::Def::as_alias);
        self.aliases(iter::repeat(scope).zip(defs));
    }

    pub(super) fn extern_defs<'b>(
        &mut self,
        scope: Id<Scope>,
        defs: impl IntoIterator<Item = &'b ast::Def>,
    ) {
        for def in defs {
            let ast::Def::Extern(def) = def else {
                continue;
            };

            let ty = self.ty(scope, &mut Generics::dynamic(), &def.ty);

            let r#extern = self.externs.add(Extern {
                id: def.id,
                name: def.name,
                ty: ty.clone(),
            });

            let var = self.vars.add(Var {
                kind: VarKind::Extern(r#extern),
                name: def.name,
                span: def.span,
                ty,
            });

            self.scopes[scope].vars.push(var);
        }
    }

    fn register_let_defs<'b>(
        &mut self,
        scope: Id<Scope>,
        defs: &'b [ast::Def],
    ) -> Vec<LetDefState<'b>> {
        let mut let_defs = Vec::new();

        for def in defs {
            let ast::Def::Let(def) = def else {
                continue;
            };

            let global = self.globals.reserve();

            let kind = VarKind::Global(global);
            let pat = self.register_let_def(scope, kind, def);

            if def.is_rec {
                let diagnostic = Diagnostic::warning("redundant `rec` modifier")
                    .with_label(def.span, "in `let` found here")
                    .with_note("module level `let` are always recursive");

                self.emitter.emit(diagnostic);
            }

            let_defs.push((global, scope, pat, def));
        }

        let_defs
    }

    fn complete_let_defs(&mut self, defs: Vec<LetDefState<'_>>) {
        for (global, scope, pat, def) in defs {
            let kind = ScopeKind::Global(global);
            let scope = self.add_scope(kind, scope);

            let expr = self.complete_let_def(scope, def);
            self.unify(&pat.ty(), &expr.ty(), def.span);

            self.globals.insert(global, Global { pat, expr });
        }
    }

    fn resolve_global_dependencies(&mut self) -> Vec<Id<Global>> {
        let dependencies = mem::take(&mut self.dependencies);

        for (global, deps) in &dependencies {
            for (&dep, tys) in deps {
                let mut ty = self.globals[dep].expr.ty();
                let is_recursive = Self::depends_on(&dependencies, dep, *global);

                if !is_recursive {
                    ty = self.instantiate(ty);
                }

                for (infer, span) in tys {
                    self.unify(infer, &ty, *span);
                }
            }
        }

        fn recurse(
            global: Id<Global>,
            seen: &mut HashSet<Id<Global>>,
            order: &mut Vec<Id<Global>>,
            dependencies: &HashMap<Id<Global>, Dependencies>,
        ) {
            if !seen.insert(global) {
                return;
            }

            for (dependency, _) in dependencies.get(&global).into_iter().flatten() {
                recurse(*dependency, seen, order, dependencies);
            }

            order.push(global);
        }

        let mut seen = HashSet::new();
        let mut order = Vec::new();

        for global in self.globals.keys() {
            recurse(global, &mut seen, &mut order, &dependencies);
        }

        order
    }

    fn depends_on(
        dependencies: &HashMap<Id<Global>, Dependencies>,
        a: Id<Global>,
        b: Id<Global>,
    ) -> bool {
        let mut seen = HashSet::new();
        let mut stack = VecDeque::new();
        stack.push_back(a);

        while let Some(x) = stack.pop_front() {
            if !seen.insert(x) {
                continue;
            }

            if x == b {
                return true;
            }

            for dependency in dependencies
                .get(&x)
                .iter()
                .flat_map(|deps| deps.keys())
                .copied()
            {
                stack.push_back(dependency);
            }
        }

        false
    }
}
