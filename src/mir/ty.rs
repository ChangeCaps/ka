use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ty {
    Str,
    Nat,
    Int,
    Real,
    Tuple(Rc<[Ty]>),
    Action(Rc<Ty>),
}

impl Ty {
    pub fn unit() -> Self {
        Self::Tuple(Rc::new([]))
    }

    pub fn action(output: Self) -> Self {
        Self::Action(Rc::new(output))
    }
}
