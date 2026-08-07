use crate::{
    arena::Id,
    ir::{Lambda, Pat, Ty, Value, Var},
};

#[derive(Clone, Debug)]
pub enum Expr {
    Value(ValueExpr),
    Var(VarExpr),
    Let(LetExpr),
    Bind(BindExpr),
    Pure(PureExpr),
    Call(CallExpr),
    Lambda(LambdaExpr),
    Variant(VariantExpr),
    Record(RecordExpr),
    Binary(BinaryExpr),
    Tuple(TupleExpr),
    Match(MatchExpr),
    Error(Ty),
}

#[derive(Clone, Debug)]
pub struct ValueExpr {
    pub value: Value,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct VarExpr {
    pub var: Id<Var>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct LetExpr {
    pub input: Box<Expr>,
    pub pat: Pat,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct BindExpr {
    pub input: Box<Expr>,
    pub pat: Pat,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct PureExpr {
    pub expr: Box<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    pub lambda: Box<Expr>,
    pub input: Box<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct LambdaExpr {
    pub lambda: Id<Lambda>,
    pub caps: Vec<Id<Var>>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct VariantExpr {
    pub name: &'static str,
    pub expr: Option<Box<Expr>>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct RecordExpr {
    pub fields: Vec<ExprField>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct ExprField {
    pub name: &'static str,
    pub expr: Expr,
}

#[derive(Clone, Debug)]
pub struct BinaryExpr {
    pub op: BinOp,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub ty: Ty,
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
}

#[derive(Clone, Debug)]
pub struct TupleExpr {
    pub fields: Vec<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<Arm>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pat,
    pub expr: Expr,
}

impl Expr {
    pub fn unit() -> Self {
        Self::Record(RecordExpr {
            fields: Vec::new(),
            ty: Ty::unit(),
        })
    }

    pub fn ty(&self) -> Ty {
        match self {
            Expr::Value(expr) => expr.ty.clone(),
            Expr::Var(expr) => expr.ty.clone(),
            Expr::Let(expr) => expr.expr.ty(),
            Expr::Bind(expr) => expr.expr.ty(),
            Expr::Pure(expr) => expr.ty.clone(),
            Expr::Call(expr) => expr.ty.clone(),
            Expr::Lambda(expr) => expr.ty.clone(),
            Expr::Variant(expr) => expr.ty.clone(),
            Expr::Record(expr) => expr.ty.clone(),
            Expr::Binary(expr) => expr.ty.clone(),
            Expr::Tuple(expr) => expr.ty.clone(),
            Expr::Match(expr) => expr.ty.clone(),
            Expr::Error(ty) => ty.clone(),
        }
    }
}
