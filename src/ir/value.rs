use std::borrow::Cow;

#[derive(Clone, Debug)]
pub enum Value {
    Num(f64),
    Str(Cow<'static, str>),
}
