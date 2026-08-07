use std::borrow::Cow;

#[derive(Clone, Debug)]
pub enum Value {
    Num(f64),
    String(Cow<'static, str>),
}
