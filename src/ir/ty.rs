use crate::arena::Id;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Nat,
    Int,
    Num,
    Str,
    Tuple(Vec<Ty>),
    Record(RecordTy),
    Lambda(LambdaTy),
    Union(UnionTy),
    Alias(AliasTy),
    Monad(Box<Ty>),
    Infer(Id<Bounds>),
}

impl Ty {
    pub const fn unit() -> Self {
        Self::Record(RecordTy { fields: Vec::new() })
    }

    pub fn lambda(input: Self, output: Self) -> Self {
        Self::Lambda(LambdaTy {
            input: Box::new(input),
            output: Box::new(output),
        })
    }

    pub fn bool() -> Self {
        Self::Union(UnionTy {
            variants: vec![
                Variant {
                    name: "true",
                    ty: None,
                },
                Variant {
                    name: "false",
                    ty: None,
                },
            ],
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LambdaTy {
    pub input: Box<Ty>,
    pub output: Box<Ty>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordTy {
    pub fields: Vec<TyField>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TyField {
    pub name: &'static str,
    pub ty: Ty,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnionTy {
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variant {
    pub name: &'static str,
    pub ty: Option<Ty>,
}

impl UnionTy {
    pub fn get(&self, name: &str) -> Option<Option<&Ty>> {
        self.variants
            .iter()
            .find(|variant| variant.name == name)
            .map(|variant| variant.ty.as_ref())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AliasTy {
    pub alias: Id<Alias>,
    pub args: Vec<Ty>,
}

#[derive(Clone, Debug)]
pub struct Alias {
    pub name: &'static str,
    pub params: Vec<Id<Bounds>>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub enum Bounds {
    Record(RecordTy),
    Union(UnionTy),
    None,
}
