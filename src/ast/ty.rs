use crate::diagnostic::Span;

#[derive(Clone, Debug)]
pub enum Ty {
    Num,
    Paren(ParenTy),
    Lambda(LambdaTy),
    Generic(GenericTy),
    Tuple(TupleTy),
    Union(UnionTy),
    Named(NamedTy),
    Monad(MonadTy),
    Error(Span),
}

#[derive(Clone, Debug)]
pub struct ParenTy {
    pub ty: Box<Ty>,
}

#[derive(Clone, Debug)]
pub struct LambdaTy {
    pub input: Box<Ty>,
    pub output: Box<Ty>,
}

#[derive(Clone, Debug)]
pub struct GenericTy {
    pub name: &'static str,
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
    pub ty: Option<Ty>,
}

#[derive(Clone, Debug)]
pub struct NamedTy {
    pub name: &'static str,
    pub args: Vec<Ty>,
}

#[derive(Clone, Debug)]
pub struct MonadTy {
    pub ty: Box<Ty>,
}
