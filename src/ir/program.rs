use std::collections::HashMap;

use crate::{
    arena::{Arena, Id},
    ir::{Bounds, Extern, Global, Scope, Ty, Var},
};

#[derive(Clone, Debug)]
pub struct Program {
    pub externs: Arena<Extern>,
    pub globals: Arena<Global>,
    pub order: Vec<Id<Global>>,

    pub scopes: Arena<Scope>,
    pub vars: Arena<Var>,

    pub bounds: Arena<Bounds>,
    pub subst: HashMap<Id<Bounds>, Ty>,
}
