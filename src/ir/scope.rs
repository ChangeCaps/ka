use crate::{
    arena::Id,
    ir::{Alias, Global, Var},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    Block,
    Bind,
    Lambda,
    Module,
    Global(Id<Global>),
}

#[derive(Clone, Debug)]
pub struct Import {
    pub name: Option<&'static str>,
    pub scope: Id<Scope>,
}

#[derive(Clone, Debug)]
pub struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<Id<Scope>>,
    pub imports: Vec<Import>,
    pub aliases: Vec<Id<Alias>>,
    pub captures: Vec<Id<Var>>,
    pub vars: Vec<Id<Var>>,
}

impl Scope {
    pub fn new(kind: ScopeKind, parent: Option<Id<Scope>>) -> Self {
        Self {
            kind,
            parent,
            imports: Vec::new(),
            aliases: Vec::new(),
            captures: Vec::new(),
            vars: Vec::new(),
        }
    }
}
