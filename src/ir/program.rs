use std::collections::HashMap;

use crate::{
    arena::{Arena, Id},
    ir::{Bound, Extern, Global, Scope, Ty, Var},
};

#[derive(Clone, Debug)]
pub struct Program {
    pub externs: Arena<Extern>,
    pub globals: Arena<Global>,
    pub order: Vec<Id<Global>>,

    pub scopes: Arena<Scope>,
    pub vars: Arena<Var>,

    pub bounds: Arena<Bound>,
    pub subst: HashMap<Id<Bound>, Ty>,
}
