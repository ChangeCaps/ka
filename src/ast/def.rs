use crate::{
    ast::{Expr, Pat, Ty},
    diagnostic::Span,
};

#[derive(Clone, Debug)]
pub enum ModuleDef {
    Def(Def),
    Global(GlobalDef),
}

impl ModuleDef {
    pub fn as_global(&self) -> Option<&GlobalDef> {
        match self {
            Self::Global(def) => Some(def),
            _ => None,
        }
    }

    pub fn as_def(&self) -> Option<&Def> {
        match self {
            Self::Def(def) => Some(def),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Def {
    Import(ImportDef),
    Extern(ExternDef),
    Alias(AliasDef),
    Error(Span),
}

impl Def {
    pub fn as_import(&self) -> Option<&ImportDef> {
        match self {
            Self::Import(def) => Some(def),
            _ => None,
        }
    }

    pub fn as_extern(&self) -> Option<&ExternDef> {
        match self {
            Self::Extern(def) => Some(def),
            _ => None,
        }
    }

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
    pub is_local: bool,
    pub name: &'static str,
    pub params: Vec<Option<&'static str>>,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct GlobalDef {
    pub is_local: bool,
    pub ty: Option<Ty>,
    pub pat: Pat,
    pub params: Vec<Pat>,
    pub expr: Expr,
    pub span: Span,
}
