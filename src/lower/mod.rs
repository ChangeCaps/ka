use std::{
    collections::{HashMap, HashSet, VecDeque},
    mem,
};

use crate::{
    arena::{Arena, Id},
    ast,
    diagnostic::{Diagnostic, Emitter, Span},
    ir,
    lower::ty::Generics,
};

mod def;
mod exhaust;
mod expr;
mod infer;
mod pat;
mod scope;
mod ty;

pub struct Lowerer<'a> {
    emitter: &'a mut dyn Emitter,

    imports: HashMap<&'static str, Id<ir::Scope>>,
    modules: Vec<(Id<ir::Scope>, Box<[ast::Def]>)>,

    externs: Arena<ir::Extern>,
    globals: Arena<ir::Global>,

    aliases: Arena<ir::Alias>,
    scopes: Arena<ir::Scope>,
    vars: Arena<ir::Var>,

    bounds: Arena<ir::Bounds>,
    subst: HashMap<Id<ir::Bounds>, ir::Ty>,
    cache: HashSet<u64>,

    dependencies: HashMap<Id<ir::Global>, Dependencies>,
}

type Dependencies = HashMap<Id<ir::Global>, Vec<(ir::Ty, Span)>>;
type LetDefState<'a> = (Id<ir::Global>, Id<ir::Scope>, ir::Pat, &'a ast::LetDef);

impl<'a> Lowerer<'a> {
    pub fn new(emitter: &'a mut dyn Emitter) -> Self {
        Self {
            emitter,

            imports: HashMap::new(),
            modules: Vec::new(),

            externs: Arena::new(),
            globals: Arena::new(),

            aliases: Arena::new(),
            scopes: Arena::new(),
            vars: Arena::new(),

            bounds: Arena::new(),
            subst: HashMap::new(),
            cache: HashSet::new(),

            dependencies: HashMap::new(),
        }
    }

    pub fn add_module(&mut self, name: &'static str, defs: Box<[ast::Def]>) -> Id<ir::Scope> {
        let scope = self.add_scope(ir::ScopeKind::Module, None);

        self.imports.insert(name, scope);
        self.modules.push((scope, defs));

        scope
    }

    pub fn finish(mut self) -> ir::Program {
        let modules = mem::take(&mut self.modules);

        for (scope, defs) in &modules {
            self.import_defs(*scope, defs);
        }

        for (scope, defs) in &modules {
            self.alias_defs(*scope, defs);
        }

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

        ir::Program {
            externs: self.externs,
            globals: self.globals,
            order,

            scopes: self.scopes,
            vars: self.vars,

            bounds: self.bounds,
            subst: self.subst,
        }
    }

    pub(super) fn import_defs<'b>(
        &mut self,
        scope: Id<ir::Scope>,
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

            self.scopes[scope].imports.push(ir::Import {
                name: def.name,
                scope: import_scope,
            });
        }
    }

    pub(super) fn alias_defs<'b>(
        &mut self,
        scope: Id<ir::Scope>,
        defs: impl IntoIterator<Item = &'b ast::Def>,
    ) {
        let mut alias_defs = Vec::new();

        for def in defs {
            let ast::Def::Alias(def) = def else {
                continue;
            };

            alias_defs.push(def);
        }

        self.aliases(scope, &alias_defs);
    }

    pub(super) fn extern_defs<'b>(
        &mut self,
        scope: Id<ir::Scope>,
        defs: impl IntoIterator<Item = &'b ast::Def>,
    ) {
        for def in defs {
            let ast::Def::Extern(def) = def else {
                continue;
            };

            let ty = self.ty(scope, &mut Generics::dynamic(), &def.ty);

            let r#extern = self.externs.add(ir::Extern {
                id: def.id,
                name: def.name,
                ty: ty.clone(),
            });

            let var = self.vars.add(ir::Var {
                kind: ir::VarKind::Extern(r#extern),
                name: def.name,
                ty,
            });

            self.scopes[scope].vars.push(var);
        }
    }

    fn register_let_defs<'b>(
        &mut self,
        scope: Id<ir::Scope>,
        defs: &'b [ast::Def],
    ) -> Vec<LetDefState<'b>> {
        let mut let_defs = Vec::new();

        for def in defs {
            let ast::Def::Let(def) = def else {
                continue;
            };

            let global = self.globals.reserve();

            let kind = ir::VarKind::Global(global);
            let pat = self.register_let_def(scope, kind, def);

            let_defs.push((global, scope, pat, def));
        }

        let_defs
    }

    fn complete_let_defs(&mut self, defs: Vec<LetDefState<'_>>) {
        for (global, scope, pat, def) in defs {
            let kind = ir::ScopeKind::Global(global);
            let scope = self.add_scope(kind, scope);

            let expr = self.complete_let_def(scope, def);
            self.unify(&pat.ty(), &expr.ty(), def.span);

            let kind = match def.params.is_empty() {
                true => ir::GlobalKind::Lambda,
                false => ir::GlobalKind::Value,
            };

            self.globals.insert(global, ir::Global { kind, pat, expr });
        }
    }

    fn resolve_global_dependencies(&mut self) -> Vec<Id<ir::Global>> {
        let dependencies = mem::take(&mut self.dependencies);

        for (global, deps) in &dependencies {
            for (&dep, tys) in deps {
                let mut ty = self.globals[dep].expr.ty();

                if !Self::depends_on(&dependencies, dep, *global) {
                    ty = self.instantiate(ty);
                }

                for (infer, span) in tys {
                    self.unify(infer, &ty, *span);
                }
            }
        }

        fn recurse(
            global: Id<ir::Global>,
            seen: &mut HashSet<Id<ir::Global>>,
            order: &mut Vec<Id<ir::Global>>,
            dependencies: &HashMap<Id<ir::Global>, Dependencies>,
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
        dependencies: &HashMap<Id<ir::Global>, Dependencies>,
        a: Id<ir::Global>,
        b: Id<ir::Global>,
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
