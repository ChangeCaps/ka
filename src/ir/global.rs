use crate::ir::{Expr, Pat};

#[derive(Clone, Debug)]
pub struct Global {
    pub pat: Pat,
    pub expr: Expr,
}
