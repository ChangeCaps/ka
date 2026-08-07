use std::collections::HashMap;

use crate::{
    arena::{Arena, Id},
    ir::{Bounds, Const, Extern, Lambda, Scope, Ty, Var},
};

#[derive(Clone, Debug)]
pub struct Program {
    pub externs: Arena<Extern>,
    pub lambdas: Arena<Lambda>,
    pub consts: Arena<Const>,
    pub order: Vec<Id<Const>>,

    pub scopes: Arena<Scope>,
    pub vars: Arena<Var>,

    pub bounds: Arena<Bounds>,
    pub subst: HashMap<Id<Bounds>, Ty>,
}
