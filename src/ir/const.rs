use crate::ir::{Expr, Pat};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConstKind {
    Value,
    Lambda,
}

#[derive(Clone, Debug)]
pub struct Const {
    pub kind: ConstKind,
    pub pat: Pat,
    pub expr: Expr,
}
