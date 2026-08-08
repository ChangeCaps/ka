use crate::ir::{Expr, Pat};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlobalKind {
    Value,
    Lambda,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub kind: GlobalKind,
    pub pat: Pat,
    pub expr: Expr,
}
