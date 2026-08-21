use std::{borrow::Cow, rc::Rc};

use crate::mir::Ty;

#[derive(Clone, Debug)]
pub enum Expr {
    Local(Local),
    Global(Global),
    Extern(Extern),

    Constant(Constant),
    Construct(Constructor),

    Let(Let),
    Bind(Bind),

    Lambda(Lambda),
    Payload(Rc<Expr>),
    Field(Rc<Expr>, usize),
    Call(Rc<Expr>, Rc<Expr>),
    Is(Rc<Expr>, usize),
    If(Rc<Expr>, Rc<Expr>, Rc<Expr>),
    Intrinsic(Intrinsic, Rc<[Expr]>),
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub globals: Vec<Expr>,
    pub output: Rc<Expr>,
}

#[derive(Clone, Debug)]
pub enum Constant {
    Nat(u64),
    Int(i64),
    Real(f64),
    Bool(bool),
    Str(Cow<'static, str>),
}

#[derive(Clone, Debug)]
pub enum Constructor {
    Pure(Rc<Expr>),
    Tuple(Rc<[Expr]>),
    Variant(usize, Rc<Expr>),
}

#[derive(Clone, Debug)]
pub struct Lambda {
    pub captures: Vec<Expr>,
    pub input: Local,
    pub output: Rc<Expr>,
}

#[derive(Clone, Debug)]
pub struct Let {
    pub input: Rc<Expr>,
    pub local: Local,
    pub output: Rc<Expr>,
}

#[derive(Clone, Debug)]
pub struct Bind {
    pub captures: Vec<Expr>,
    pub input: Rc<Expr>,
    pub local: Local,
    pub output: Rc<Expr>,
}

#[derive(Clone, Debug)]
pub struct Local {
    pub index: usize,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub index: usize,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct Extern {
    pub id: &'static str,
    pub ty: Ty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Intrinsic {
    NatAdd,
    NatSub,
    NatMul,
    NatXor,
    NatGt,
    NatLt,
    NatGe,
    NatLe,
    NatEq,

    IntAdd,
    IntSub,
    IntMul,
    IntGt,
    IntLt,
    IntGe,
    IntLe,
    IntEq,

    RealAdd,
    RealSub,
    RealMul,
    RealDiv,
    RealGt,
    RealLt,
    RealGe,
    RealLe,
    RealEq,

    BoolNot,
    BoolAnd,
    BoolOr,

    NatToInt,
    NatToReal,

    IntToNat,
    IntToReal,

    RealToNat,
    RealToInt,

    FormatNat,
    FormatInt,
    FormatReal,

    HashStr,
    HashNat,
    HashInt,
    HashReal,

    StrEq,
    StrLength,
    StrPrepend,
    StrSplitAt,
    StrFind,
}

impl Expr {
    pub fn unit() -> Self {
        Self::Construct(Constructor::Tuple(Rc::new([])))
    }
}
