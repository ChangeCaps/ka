use crate::{
    arena::Id,
    ir::{Expr, Pat, Scope, Ty},
};

#[derive(Clone, Debug)]
pub struct Lambda {
    pub scope: Id<Scope>,
    pub input: Pat,
    pub expr: Expr,
    pub ty: Ty,
}
