use crate::diagnostic::Span;

#[derive(Clone, Debug)]
pub enum Pat {
    Paren(ParenPat),
    Wild(WildPat),
    Bind(BindPat),
    Str(StrPat),
    Cons(ConsPat),
    List(ListPat),
    Tuple(TuplePat),
    Variant(VariantPat),
    Error(Span),
}

#[derive(Clone, Debug)]
pub struct ParenPat {
    pub pat: Box<Pat>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct WildPat {
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct BindPat {
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StrPat {
    pub string: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ConsPat {
    pub first: Box<Pat>,
    pub rest: Box<Pat>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ListPat {
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TuplePat {
    pub fields: Vec<Pat>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct VariantPat {
    pub name: Option<&'static str>,
    pub pat: Option<Box<Pat>>,
    pub span: Span,
}

impl Pat {
    pub fn span(&self) -> Span {
        match self {
            Pat::Paren(pat) => pat.span,
            Pat::Wild(pat) => pat.span,
            Pat::Bind(pat) => pat.span,
            Pat::Str(pat) => pat.span,
            Pat::Cons(pat) => pat.span,
            Pat::List(pat) => pat.span,
            Pat::Tuple(pat) => pat.span,
            Pat::Variant(pat) => pat.span,
            Pat::Error(span) => *span,
        }
    }
}
