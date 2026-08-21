use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ty {
    Str,
    Nat,
    Int,
    Real,
    Bool,
    Tuple(Rc<[Self]>),
    Record(Rc<[(&'static str, Self)]>),
    Union(Rc<[(&'static str, Self)]>),
    Action(Rc<Self>),
    Lambda(Rc<Self>, Rc<Self>),
    Boxed(usize),
}

impl Ty {
    pub fn unit() -> Self {
        Self::Record(Rc::new([]))
    }

    pub fn action(output: Self) -> Self {
        Self::Action(Rc::new(output))
    }
}
