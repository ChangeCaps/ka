use std::collections::HashMap;

use crate::{
    arena::{Arena, Id},
    ir::{Alias, Bound, Extern, Global, Scope, Ty, Var},
};

#[derive(Clone, Debug)]
pub struct Program {
    pub externs: Arena<Extern>,
    pub globals: Arena<Global>,

    pub scopes: Arena<Scope>,
    pub vars: Arena<Var>,

    pub aliases: Arena<Alias>,
    pub bounds: Arena<Bound>,
    pub subst: HashMap<Id<Bound>, Ty>,
}
