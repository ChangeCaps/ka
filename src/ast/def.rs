use crate::{
    ast::{Expr, Pat, Ty},
    diagnostic::Span,
};

#[derive(Clone, Debug)]
pub enum Def {
    Import(ImportDef),
    Extern(ExternDef),
    Alias(AliasDef),
    Let(LetDef),
    Error(Span),
}

impl Def {
    pub fn as_alias(&self) -> Option<&AliasDef> {
        match self {
            Self::Alias(def) => Some(def),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImportDef {
    pub path: &'static str,
    pub name: Option<&'static str>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ExternDef {
    pub id: &'static str,
    pub name: &'static str,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AliasDef {
    pub name: &'static str,
    pub params: Vec<Option<&'static str>>,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct LetDef {
    pub ty: Option<Ty>,
    pub is_rec: bool,
    pub pat: Pat,
    pub params: Vec<Pat>,
    pub is_bind: bool,
    pub expr: Expr,
    pub span: Span,
}
