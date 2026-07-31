use crate::diagnostic::Span;

#[derive(Clone, Debug)]
pub enum Pat {
    Paren(ParenPat),
    Bind(BindPat),
    Tuple(TuplePat),
    Tag(TagPat),
    Error(Span),
}

#[derive(Clone, Debug)]
pub struct ParenPat {
    pub pat: Box<Pat>,
}

#[derive(Clone, Debug)]
pub struct BindPat {
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TuplePat {
    pub pats: Vec<Pat>,
}

#[derive(Clone, Debug)]
pub struct TagPat {
    pub name: Option<&'static str>,
    pub pat: Option<Box<Pat>>,
}
