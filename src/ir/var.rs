use crate::{
    arena::Id,
    diagnostic::Span,
    ir::{Extern, Global, Ty},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VarKind {
    Global(Id<Global>),
    Extern(Id<Extern>),
    Local,
}

#[derive(Clone, Debug)]
pub struct Var {
    pub kind: VarKind,
    pub name: &'static str,
    pub ty: Ty,
    pub span: Span,
}
