use crate::{
    ast::{Def, Pat},
    diagnostic::Span,
};

#[derive(Clone, Debug)]
pub enum Expr {
    Paren(ParenExpr),
    Num(NumExpr),
    Named(NamedExpr),
    Call(CallExpr),
    Tag(TagExpr),
    Tuple(TupleExpr),
    Block(BlockExpr),
    Do(DoExpr),
    Match(MatchExpr),
    Error(Span),
}

#[derive(Clone, Debug)]
pub struct ParenExpr {
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct NumExpr {
    pub number: f64,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct NamedExpr {
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    pub lambda: Box<Expr>,
    pub input: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct TagExpr {
    pub name: Option<&'static str>,
    pub expr: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub struct TupleExpr {
    pub fields: Vec<Expr>,
}

#[derive(Clone, Debug)]
pub struct BlockExpr {
    pub defs: Vec<Def>,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct DoExpr {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Def(Def),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<Arm>,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pat,
    pub expr: Expr,
}
