use crate::{
    arena::Id,
    ir::{Const, Extern, Ty},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VarKind {
    Const(Id<Const>),
    Extern(Id<Extern>),
    Local,
}

#[derive(Clone, Debug)]
pub struct Var {
    pub kind: VarKind,
    pub name: &'static str,
    pub ty: Ty,
}
