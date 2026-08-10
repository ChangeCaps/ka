use std::{
    fmt,
    ops::{Deref, DerefMut},
};

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

    pub aliases: Vec<Visible<Alias>>,
    pub vars: Vec<Visible<Var>>,

    pub captures: Vec<Id<Var>>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Visibility {
    Local,
    Global,
}

pub struct Visible<T> {
    pub id: Id<T>,
    pub vis: Visibility,
}

impl<T> Visible<T> {
    pub const fn new(id: Id<T>, vis: Visibility) -> Self {
        Self { id, vis }
    }

    pub const fn global(id: Id<T>) -> Self {
        Self::new(id, Visibility::Global)
    }

    pub const fn local(id: Id<T>) -> Self {
        Self::new(id, Visibility::Local)
    }

    pub const fn is_global(&self) -> bool {
        matches!(self.vis, Visibility::Global)
    }
}

impl<T> Deref for Visible<T> {
    type Target = Id<T>;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<T> DerefMut for Visible<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}

impl<T> Clone for Visible<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Visible<T> {}

impl<T> fmt::Debug for Visible<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Visible")
            .field("id", &self.id)
            .field("vis", &self.vis)
            .finish()
    }
}
