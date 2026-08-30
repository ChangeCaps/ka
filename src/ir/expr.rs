use crate::{
    arena::Id,
    ir::{Pat, Scope, Ty, Value, Var},
};

#[derive(Clone, Debug)]
pub enum Expr {
    Value(ValueExpr),
    Var(VarExpr),
    Let(LetExpr),
    Bind(BindExpr),
    Pure(PureExpr),
    Call(CallExpr),
    With(WithExpr),
    Field(FieldExpr),
    Lambda(LambdaExpr),
    Cons(ConsExpr),
    Empty(EmptyExpr),
    Variant(VariantExpr),
    Record(RecordExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Tuple(TupleExpr),
    Match(MatchExpr),
    Intrinsic(IntrinsicExpr),
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
    pub output: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct BindExpr {
    pub scope: Id<Scope>,
    pub input: Box<Expr>,
    pub pat: Pat,
    pub output: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct PureExpr {
    pub input: Box<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    pub lambda: Box<Expr>,
    pub input: Box<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct WithExpr {
    pub input: Box<Expr>,
    pub fields: Vec<ExprField>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct FieldExpr {
    pub input: Box<Expr>,
    pub name: &'static str,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct LambdaExpr {
    pub scope: Id<Scope>,
    pub input: Pat,
    pub output: Box<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct ConsExpr {
    pub item: Box<Expr>,
    pub list: Box<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct EmptyExpr {
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct VariantExpr {
    pub name: &'static str,
    pub payload: Option<Box<Expr>>,
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
pub struct UnaryExpr {
    pub op: UnOp,
    pub input: Box<Expr>,
    pub ty: Ty,
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
    Ge,
    Le,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub struct TupleExpr {
    pub fields: Vec<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct MatchExpr {
    pub input: Box<Expr>,
    pub arms: Vec<Arm>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pat,
    pub expr: Expr,
}

#[derive(Clone, Debug)]
pub struct IntrinsicExpr {
    pub intrinsic: Intrinsic,
    pub inputs: Vec<Expr>,
    pub ty: Ty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Intrinsic {
    Dynamic,

    Trace,

    FormatNat,
    FormatInt,
    FormatReal,

    HashStr,
    HashNat,
    HashInt,
    HashReal,

    NatXor,

    StrLength,
    StrPrepend,
    StrSplitAt,
    StrFind,
}

impl Expr {
    pub fn unit() -> Self {
        Self::Record(RecordExpr {
            fields: Vec::new(),
            ty: Ty::UNIT,
        })
    }

    pub fn ty(&self) -> Ty {
        match self {
            Expr::Value(expr) => expr.ty.clone(),
            Expr::Var(expr) => expr.ty.clone(),
            Expr::Let(expr) => expr.output.ty(),
            Expr::Bind(expr) => expr.output.ty(),
            Expr::Pure(expr) => expr.ty.clone(),
            Expr::Call(expr) => expr.ty.clone(),
            Expr::With(expr) => expr.ty.clone(),
            Expr::Field(expr) => expr.ty.clone(),
            Expr::Lambda(expr) => expr.ty.clone(),
            Expr::Cons(expr) => expr.ty.clone(),
            Expr::Empty(expr) => expr.ty.clone(),
            Expr::Variant(expr) => expr.ty.clone(),
            Expr::Record(expr) => expr.ty.clone(),
            Expr::Unary(expr) => expr.ty.clone(),
            Expr::Binary(expr) => expr.ty.clone(),
            Expr::Tuple(expr) => expr.ty.clone(),
            Expr::Match(expr) => expr.ty.clone(),
            Expr::Intrinsic(expr) => expr.ty.clone(),
            Expr::Error(ty) => ty.clone(),
        }
    }
}
