use crate::{
    arena::Id,
    diagnostic::Span,
    ir::{Ty, Var},
};

#[derive(Clone, Debug)]
pub enum Pat {
    Wild(WildPat),
    Bind(BindPat),
    Str(StrPat),
    Variant(VariantPat),
    Tuple(TuplePat),
    Error(ErrorPat),
}

#[derive(Clone, Debug)]
pub struct WildPat {
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct BindPat {
    pub var: Id<Var>,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StrPat {
    pub string: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct VariantPat {
    pub name: &'static str,
    pub payload: Option<Box<Pat>>,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TuplePat {
    pub fields: Vec<Pat>,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ErrorPat {
    pub ty: Ty,
    pub span: Span,
}

impl Pat {
    pub fn is_wild(&self) -> bool {
        matches!(self, Self::Wild(..) | Self::Bind(..) | Self::Error(..))
    }

    pub fn ty(&self) -> Ty {
        match self {
            Pat::Str(..) => Ty::Str,

            Pat::Wild(pat) => pat.ty.clone(),
            Pat::Bind(pat) => pat.ty.clone(),
            Pat::Variant(pat) => pat.ty.clone(),
            Pat::Tuple(pat) => pat.ty.clone(),
            Pat::Error(pat) => pat.ty.clone(),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Pat::Wild(pat) => pat.span,
            Pat::Bind(pat) => pat.span,
            Pat::Str(pat) => pat.span,
            Pat::Variant(pat) => pat.span,
            Pat::Tuple(pat) => pat.span,
            Pat::Error(pat) => pat.span,
        }
    }

    pub fn is_refutable(&self) -> bool {
        match self {
            Pat::Str(..) => true,

            Pat::Wild(..) | Pat::Bind(..) | Pat::Error(..) => false,

            Pat::Variant(pat) => pat.payload.as_deref().is_some_and(Pat::is_refutable),
            Pat::Tuple(pat) => pat.fields.iter().any(Pat::is_refutable),
        }
    }
}
