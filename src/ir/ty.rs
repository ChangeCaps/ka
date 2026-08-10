use crate::arena::Id;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Str,
    Numeric(Numeric),
    Tuple(Vec<Ty>),
    Record(RecordTy),
    Lambda(LambdaTy),
    Union(UnionTy),
    Alias(AliasTy),
    Monad(Box<Ty>),
    Infer(Id<Bound>),
    Generic(GenericTy),
}

impl Ty {
    pub const UNIT: Self = Self::Record(RecordTy { fields: Vec::new() });

    pub const NAT: Self = Self::Numeric(Numeric::Nat);

    pub const INT: Self = Self::Numeric(Numeric::Int);

    pub const NUM: Self = Self::Numeric(Numeric::Num);

    pub fn option(inner: Self) -> Self {
        Self::Union(UnionTy {
            variants: vec![
                Variant {
                    name: "some",
                    payload: Some(inner),
                },
                Variant {
                    name: "none",
                    payload: None,
                },
            ],
        })
    }

    pub fn bool() -> Self {
        Self::Union(UnionTy {
            variants: vec![
                Variant {
                    name: "true",
                    payload: None,
                },
                Variant {
                    name: "false",
                    payload: None,
                },
            ],
        })
    }

    pub fn lambda(input: Self, output: Self) -> Self {
        Self::Lambda(LambdaTy {
            input: Box::new(input),
            output: Box::new(output),
        })
    }

    pub fn monad(result: Self) -> Self {
        Self::Monad(Box::new(result))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Numeric {
    Nat,
    Int,
    Num,
}

impl Numeric {
    pub fn as_str(&self) -> &'static str {
        match self {
            Numeric::Nat => "nat",
            Numeric::Int => "int",
            Numeric::Num => "num",
        }
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

impl RecordTy {
    pub fn get(&self, name: &str) -> Option<&Ty> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.ty)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericTy {
    pub name: &'static str,
    pub bound: Id<Bound>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnionTy {
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variant {
    pub name: &'static str,
    pub payload: Option<Ty>,
}

impl UnionTy {
    pub fn get(&self, name: &str) -> Option<Option<&Ty>> {
        self.variants
            .iter()
            .find(|variant| variant.name == name)
            .map(|variant| variant.payload.as_ref())
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
    pub params: Vec<Id<Bound>>,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub enum Bound {
    Numeric(Numeric),
    Record(RecordTy),
    Union(UnionTy),
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_ordering() {
        assert!(Numeric::Int > Numeric::Nat);
        assert!(Numeric::Num > Numeric::Nat);
        assert!(Numeric::Num > Numeric::Int);
    }
}
