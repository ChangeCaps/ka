use crate::{
    arena::Id,
    ir::{Alias, Global, Scope, ScopeKind, Var, VarKind},
    lower::Lowerer,
};

impl Lowerer<'_> {
    pub(super) fn add_scope(
        &mut self,
        kind: ScopeKind,
        parent: impl Into<Option<Id<Scope>>>,
    ) -> Id<Scope> {
        self.scopes.add(Scope::new(kind, parent.into()))
    }

    pub(super) fn resolve_var(
        &mut self,
        scope: Id<Scope>,
        import: Option<&str>,
        name: &str,
    ) -> Option<Id<Var>> {
        match import {
            Some(import) => self.find_var_in_import(scope, import, name),
            None => self.find_or_capture_var(scope, name),
        }
    }

    fn find_import(&self, scope: Id<Scope>, name: &str) -> Option<Id<Scope>> {
        self.scopes[scope]
            .imports
            .iter()
            .rev()
            .find(|import| import.name == Some(name))
            .map(|import| import.scope)
            .or_else(|| self.find_import_in_parent(scope, name))
    }

    fn find_import_in_parent(&self, scope: Id<Scope>, name: &str) -> Option<Id<Scope>> {
        let parent = self.scopes[scope].parent?;
        self.find_import(parent, name)
    }

    fn find_var_in_import(&self, scope: Id<Scope>, import: &str, name: &str) -> Option<Id<Var>> {
        self.find_import(scope, import)
            .and_then(|scope| self.find_var(scope, name))
    }

    fn find_or_capture_var(&mut self, scope: Id<Scope>, name: &str) -> Option<Id<Var>> {
        self.find_var(scope, name)
            .or_else(|| self.import_var(scope, name))
            .or_else(|| self.capture_var(scope, name))
    }

    pub(super) fn find_var(&self, scope: Id<Scope>, name: &str) -> Option<Id<Var>> {
        self.scopes[scope]
            .vars
            .iter()
            .copied()
            .rev()
            .find(|id| self.vars[*id].name == name)
    }

    fn capture_var(&mut self, scope: Id<Scope>, name: &str) -> Option<Id<Var>> {
        let parent = self.scopes[scope].parent?;

        self.find_or_capture_var(parent, name).inspect(|&id| {
            if self.vars[id].kind == VarKind::Local {
                self.scopes[scope].vars.push(id);
                self.scopes[scope].captures.push(id);
            }
        })
    }

    fn import_var(&mut self, scope: Id<Scope>, name: &str) -> Option<Id<Var>> {
        self.scopes[scope]
            .imports
            .iter()
            .rev()
            .filter(|import| import.name.is_none())
            .find_map(|import| self.find_var(import.scope, name))
    }

    pub(super) fn resolve_alias(
        &self,
        scope: Id<Scope>,
        import: Option<&str>,
        name: &str,
    ) -> Option<Id<Alias>> {
        match import {
            Some(import) => self
                .find_import(scope, import)
                .and_then(|scope| self.find_alias(scope, name)),

            None => self.find_or_import_alias(scope, name),
        }
    }

    fn find_or_import_alias(&self, scope: Id<Scope>, name: &str) -> Option<Id<Alias>> {
        self.find_alias(scope, name)
            .or_else(|| self.find_imported_alias(scope, name))
            .or_else(|| self.find_alias_in_parent(scope, name))
    }

    fn find_alias(&self, scope: Id<Scope>, name: &str) -> Option<Id<Alias>> {
        self.scopes[scope]
            .aliases
            .iter()
            .copied()
            .find(|id| self.aliases[*id].name == name)
    }

    fn find_alias_in_parent(&self, scope: Id<Scope>, name: &str) -> Option<Id<Alias>> {
        let parent = self.scopes[scope].parent?;
        self.find_alias(parent, name)
    }

    fn find_imported_alias(&self, scope: Id<Scope>, name: &str) -> Option<Id<Alias>> {
        self.scopes[scope]
            .imports
            .iter()
            .filter(|import| import.name.is_none())
            .find_map(|import| self.find_alias(import.scope, name))
    }

    pub(super) fn current_global(&self, scope: Id<Scope>) -> Id<Global> {
        if let ScopeKind::Global(global) = self.scopes[scope].kind {
            return global;
        }

        let parent = self.scopes[scope].parent.unwrap();
        self.current_global(parent)
    }
}
