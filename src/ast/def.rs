use crate::{
    ast::{Expr, Pat, Ty},
    diagnostic::Span,
};

#[derive(Clone, Debug)]
pub enum Def {
    Type(TypeDef),
    Let(LetDef),
    Error(Span),
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub name: Option<&'static str>,
    pub args: Vec<Option<&'static str>>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct LetDef {
    pub ty: Option<Ty>,
    pub pat: Pat,
    pub args: Vec<Pat>,
    pub expr: Expr,
    pub span: Span,
}
