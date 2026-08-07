use crate::ir::Ty;

#[derive(Clone, Debug)]
pub struct Extern {
    pub id: &'static str,
    pub name: &'static str,
    pub ty: Ty,
}
