use std::borrow::Cow;

use crate::{arena::Id, ir::Ty};

#[derive(Clone, Debug)]
pub enum Expr {
    Constant(Constant),
    Construct(Constructor),
    Field(Id<Expr>, usize),
}

#[derive(Clone, Debug)]
pub enum Constant {
    Nat(u64),
    Int(i64),
    Real(f64),
    Str(Cow<'static, str>),
}

#[derive(Clone, Debug)]
pub enum Constructor {
    Pure(Id<Expr>),
    Tuple(Vec<Id<Expr>>),
    Variant(usize, Id<Expr>),
}

#[derive(Clone, Debug)]
pub struct Local {
    pub index: usize,
    pub ty: Ty,
}
