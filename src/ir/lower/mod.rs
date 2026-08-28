mod def;
mod exhaust;
mod expr;
mod infer;
mod intrinsic;
mod pat;
mod prelude;
mod scope;
mod ty;

use std::{
    collections::{HashMap, HashSet},
    iter, mem,
};

use crate::{
    arena::{Arena, Id},
    ast,
    diagnostic::{Diagnostic, Emitter, Span},
    ir::{
        Alias, Bound, Extern, Global, Import, Program, Scope, ScopeKind, Ty, Var, VarKind,
        Visibility, Visible, lower::ty::Generics,
    },
};

pub struct Lowerer<'a> {
    emitter: &'a mut dyn Emitter,

    imports: HashMap<&'static str, Id<Scope>>,
    modules: Vec<(Id<Scope>, Box<[ast::ModuleDef]>)>,

    externs: Arena<Extern>,
    globals: Arena<Global>,

    aliases: Arena<Alias>,
    scopes: Arena<Scope>,
    vars: Arena<Var>,

    bounds: Arena<Bound>,
    subst: HashMap<Id<Bound>, Ty>,
    cache: HashSet<u64>,

    prelude: Id<Scope>,
    dependencies: HashMap<Id<Global>, Dependencies>,
}

type Dependencies = HashMap<Id<Global>, Vec<(Ty, Span)>>;

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

    pub fn add_module(&mut self, name: &'static str, defs: Box<[ast::ModuleDef]>) -> Id<Scope> {
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
            self.import_defs(*scope, defs.iter().filter_map(ast::ModuleDef::as_def));
        }

        self.aliases(modules.iter().flat_map(|(scope, defs)| {
            let alias_defs = defs
                .iter()
                .filter_map(ast::ModuleDef::as_def)
                .filter_map(ast::Def::as_alias);

            iter::repeat(*scope).zip(alias_defs)
        }));

        for (scope, defs) in &modules {
            self.extern_defs(*scope, defs.iter().filter_map(ast::ModuleDef::as_def));
        }

        let global_defs = modules.iter().flat_map(|(scope, defs)| {
            let global_defs = defs.iter().filter_map(ast::ModuleDef::as_global);

            iter::repeat(*scope).zip(global_defs)
        });

        self.global_defs(global_defs);

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

            aliases: self.aliases,
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

            self.scopes[scope].vars.push(Visible::global(var));
        }
    }

    fn global_defs<'b>(&mut self, defs: impl IntoIterator<Item = (Id<Scope>, &'b ast::GlobalDef)>) {
        let mut global_defs = Vec::new();

        for (scope, def) in defs {
            let global = self.globals.reserve();

            let vis = match def.is_local {
                true => Visibility::Local,
                false => Visibility::Global,
            };

            let kind = VarKind::Global(global);
            let pat = self.pat(scope, vis, kind, &def.pat);

            global_defs.push((global, scope, pat, def));
        }

        for (global, scope, pat, def) in global_defs {
            let kind = ScopeKind::Global(global);
            let scope = self.add_scope(kind, scope);

            let expr = self.complete_let(scope, def.ty.as_ref(), &def.params, &def.expr, def.span);
            self.unify(&pat.ty(), &expr.ty(), def.span);

            self.globals.insert(global, Global { pat, expr });
        }
    }

    fn resolve_global_dependencies(&mut self) -> Vec<Id<Global>> {
        fn recurse(
            lowerer: &mut Lowerer<'_>,
            global: Id<Global>,
            rec: &mut HashSet<Id<Global>>,
            stack: &mut Vec<Id<Global>>,
            order: &mut Vec<Id<Global>>,
        ) {
            stack.push(global);

            for (dependency, tys) in lowerer.dependencies.remove(&global).into_iter().flatten() {
                recurse(lowerer, dependency, rec, stack, order);

                if let Some(i) = stack.iter().rposition(|x| *x == dependency) {
                    for global in stack[i..].iter().copied() {
                        rec.insert(global);
                    }
                }

                let is_recursive = rec.contains(&global);

                for (infer, span) in tys {
                    let mut ty = lowerer.globals[dependency].expr.ty();

                    if is_recursive {
                        ty = lowerer.instantiate_generics(ty);
                    } else {
                        ty = lowerer.instantiate_inferred(ty);
                    }

                    lowerer.unify(&infer, &ty, span);
                }
            }

            stack.pop();

            if !order.contains(&global) {
                order.push(global);
            }
        }

        let mut rec = HashSet::new();
        let mut stack = Vec::new();
        let mut order = Vec::new();

        for global in self.globals.keys().collect::<Vec<_>>() {
            recurse(self, global, &mut rec, &mut stack, &mut order);
        }

        order
    }
}
