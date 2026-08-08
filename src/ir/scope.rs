use crate::{
    arena::Id,
    ir::{Alias, Const, Var},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    Block,
    Bind,
    Lambda,
    Module,
    Const(Id<Const>),
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
    pub vars: Vec<Id<Var>>,
    pub caps: Vec<Id<Var>>,
}
