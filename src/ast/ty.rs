use crate::diagnostic::Span;

#[derive(Clone, Debug)]
pub enum Ty {
    Nat,
    Int,
    Real,
    Str,
    Paren(ParenTy),
    List(ListTy),
    Lambda(LambdaTy),
    Generic(GenericTy),
    Record(RecordTy),
    Tuple(TupleTy),
    Union(UnionTy),
    Alias(AliasTy),
    Monad(MonadTy),
    Error(Span),
}

#[derive(Clone, Debug)]
pub struct ParenTy {
    pub ty: Box<Ty>,
}

#[derive(Clone, Debug)]
pub struct ListTy {
    pub item: Box<Ty>,
}

#[derive(Clone, Debug)]
pub struct LambdaTy {
    pub input: Box<Ty>,
    pub output: Box<Ty>,
}

#[derive(Clone, Debug)]
pub struct GenericTy {
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TupleTy {
    pub fields: Vec<Ty>,
}

#[derive(Clone, Debug)]
pub struct UnionTy {
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: Option<&'static str>,
    pub payload: Option<Ty>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RecordTy {
    pub fields: Vec<TyField>,
}

#[derive(Clone, Debug)]
pub struct TyField {
    pub name: Option<&'static str>,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AliasTy {
    pub import: Option<&'static str>,
    pub name: &'static str,
    pub args: Vec<Ty>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MonadTy {
    pub ty: Box<Ty>,
}
