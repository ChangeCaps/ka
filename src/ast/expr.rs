use crate::{
    ast::{Def, Pat, Ty},
    diagnostic::Span,
};

#[derive(Clone, Debug)]
pub enum Expr {
    Paren(ParenExpr),
    Num(NumExpr),
    Str(StrExpr),
    Named(NamedExpr),
    Field(FieldExpr),
    With(WithExpr),
    Call(CallExpr),
    Lambda(LambdaExpr),
    Variant(VariantExpr),
    Record(RecordExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Tuple(TupleExpr),
    Block(BlockExpr),
    Do(DoExpr),
    Match(MatchExpr),
    Error(Span),
}

#[derive(Clone, Debug)]
pub struct ParenExpr {
    pub expr: Box<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct NumExpr {
    pub number: f64,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StrExpr {
    pub string: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct NamedExpr {
    pub import: Option<&'static str>,
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FieldExpr {
    pub input: Box<Expr>,
    pub name: Option<&'static str>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct WithExpr {
    pub input: Box<Expr>,
    pub fields: Vec<ExprField>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    pub lambda: Box<Expr>,
    pub input: Box<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct LambdaExpr {
    pub params: Vec<Pat>,
    pub expr: Box<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct VariantExpr {
    pub name: Option<&'static str>,
    pub expr: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RecordExpr {
    pub fields: Vec<ExprField>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ExprField {
    pub name: Option<&'static str>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TupleExpr {
    pub fields: Vec<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct UnaryExpr {
    pub op: UnOp,
    pub input: Box<Expr>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnOp {
    Nat,
    Int,
    Real,
    Not,
}

#[derive(Clone, Debug)]
pub struct BinaryExpr {
    pub op: BinOp,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Gt,
    Lt,
    GtEq,
    LtEq,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub struct BlockExpr {
    pub stmts: Vec<BlockStmt>,
    pub expr: Box<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum BlockStmt {
    Def(Def),
    Let(LetStmt),
}

impl BlockStmt {
    pub fn as_def(&self) -> Option<&Def> {
        match self {
            Self::Def(def) => Some(def),
            _ => None,
        }
    }

    pub fn as_let(&self) -> Option<&LetStmt> {
        match self {
            Self::Let(stmt) => Some(stmt),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LetStmt {
    pub ty: Option<Ty>,
    pub pat: Pat,
    pub params: Vec<Pat>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct DoExpr {
    pub kind: DoKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum DoKind {
    Block(Vec<DoStmt>),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum DoStmt {
    Def(Def),
    Expr(Expr),
    Let(LetStmt),
    Bind(BindStmt),
}

impl DoStmt {
    pub fn as_def(&self) -> Option<&Def> {
        match self {
            Self::Def(def) => Some(def),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BindStmt {
    pub ty: Option<Ty>,
    pub pat: Pat,
    pub params: Vec<Pat>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<Arm>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pat,
    pub expr: Expr,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::Paren(expr) => expr.span,
            Self::Num(expr) => expr.span,
            Self::Str(expr) => expr.span,
            Self::Named(expr) => expr.span,
            Self::Field(expr) => expr.span,
            Self::With(expr) => expr.span,
            Self::Call(expr) => expr.span,
            Self::Lambda(expr) => expr.span,
            Self::Variant(expr) => expr.span,
            Self::Record(expr) => expr.span,
            Self::Unary(expr) => expr.span,
            Self::Binary(expr) => expr.span,
            Self::Tuple(expr) => expr.span,
            Self::Block(expr) => expr.span,
            Self::Do(expr) => expr.span,
            Self::Match(expr) => expr.span,
            Self::Error(span) => *span,
        }
    }
}
