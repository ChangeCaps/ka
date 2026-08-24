use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
    io::{self, Write},
    rc::Rc,
};

use crate::mir::{Constant, Constructor, Entry, Expr, Intrinsic};

pub fn run(entry: &Entry) {
    let mut runtime = Runtime::new();

    (runtime.globals).resize_with(entry.globals.len(), || Value::Uninit);

    for (i, global) in entry.globals.iter().enumerate().rev() {
        let value = runtime.eval(global);
        runtime.globals[i] = value;
    }

    let value = runtime.eval(&entry.output);

    let Value::Action(action) = value else {
        panic!();
    };

    runtime.action(&action);
}

#[derive(Clone, Debug)]
enum Value<'a> {
    Uninit,

    Nat(u64),
    Int(i64),
    Real(f64),
    Bool(bool),
    Str(String),

    Variant(usize, Rc<Self>),

    Tuple(Rc<[Self]>),

    Lambda {
        captures: Rc<[Self]>,
        output: &'a Expr,
    },

    Extern(Extern<'a>),

    Action(Action<'a>),
}

#[derive(Clone)]
struct Extern<'a>(Rc<dyn Fn(Value<'a>) -> Value<'a> + 'a>);

impl<'a> Extern<'a> {
    fn new(f: impl Fn(Value<'a>) -> Value<'a> + 'a) -> Self {
        Self(Rc::new(f))
    }
}

impl fmt::Debug for Extern<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Extern").finish()
    }
}

impl Value<'_> {
    #[track_caller]
    fn as_str(&self) -> &str {
        match self {
            Self::Str(s) => s,
            _ => panic!("{:?}", self),
        }
    }

    #[track_caller]
    fn as_nat(&self) -> u64 {
        match self {
            Self::Nat(x) => *x,
            _ => panic!("{:?}", self),
        }
    }

    #[track_caller]
    fn as_int(&self) -> i64 {
        match self {
            Self::Int(x) => *x,
            _ => panic!("{:?}", self),
        }
    }

    #[track_caller]
    fn as_real(&self) -> f64 {
        match self {
            Self::Real(x) => *x,
            _ => panic!("{:?}", self),
        }
    }

    #[track_caller]
    fn as_bool(&self) -> bool {
        match self {
            Self::Bool(x) => *x,
            _ => panic!("{:?}", self),
        }
    }
}

#[derive(Clone, Debug)]
enum Action<'a> {
    Pure(Rc<Value<'a>>),

    Bind {
        captures: Rc<[Value<'a>]>,
        input: Rc<Value<'a>>,
        output: &'a Expr,
    },

    Extern(Extern<'a>),
}

struct Runtime<'a> {
    globals: Vec<Value<'a>>,
    stack: Vec<Value<'a>>,
    frame: usize,
}

impl<'a> Runtime<'a> {
    fn new() -> Self {
        Self {
            globals: Vec::new(),
            stack: Vec::new(),
            frame: 0,
        }
    }

    fn action(&mut self, action: &Action<'a>) -> Value<'a> {
        match action {
            Action::Pure(value) => value.as_ref().clone(),

            Action::Bind {
                captures,
                input,
                output,
            } => {
                let Value::Action(action) = input.as_ref() else {
                    panic!();
                };

                let input = self.action(action);

                let frame = self.frame;
                self.frame = self.stack.len();
                self.stack.extend(captures.iter().cloned());
                self.stack.push(input);

                let Value::Action(action) = self.eval(output) else {
                    panic!();
                };

                let output = self.action(&action);

                self.stack.truncate(self.frame);
                self.frame = frame;

                output
            }

            Action::Extern(r#extern) => (r#extern.0)(Value::Uninit),
        }
    }

    fn eval(&mut self, expr: &'a Expr) -> Value<'a> {
        match expr {
            Expr::Local(local) => self.stack[self.frame + local.index].clone(),

            Expr::Global(global) => self.globals[global.index].clone(),

            Expr::Extern(r#extern) => match r#extern.id {
                "io::print" => Value::Extern(Extern::new(|f| {
                    Value::Action(Action::Extern(Extern::new(move |_| {
                        let mut stdout = io::stdout();
                        let _ = stdout.write_all(f.as_str().as_bytes());
                        let _ = stdout.flush();

                        Value::Tuple(Rc::new([]))
                    })))
                })),

                _ => panic!("{}", r#extern.id),
            },

            Expr::Constant(constant) => match constant {
                Constant::Nat(x) => Value::Nat(*x),
                Constant::Int(x) => Value::Int(*x),
                Constant::Real(x) => Value::Real(*x),
                Constant::Bool(b) => Value::Bool(*b),
                Constant::Str(s) => Value::Str(s.to_string()),
            },

            Expr::Construct(constructor) => match constructor {
                Constructor::Pure(expr) => {
                    let value = self.eval(expr);
                    Value::Action(Action::Pure(Rc::new(value)))
                }

                Constructor::Tuple(exprs) => {
                    let fields = exprs.iter().map(|expr| self.eval(expr)).collect();
                    Value::Tuple(fields)
                }

                Constructor::Variant(index, expr) => {
                    let value = self.eval(expr);
                    Value::Variant(*index, Rc::new(value))
                }
            },

            Expr::Let(expr) => {
                let input = self.eval(&expr.input);

                let index = self.frame + expr.local.index;
                (self.stack).resize_with(index + 1, || Value::Uninit);
                self.stack[index] = input;

                self.eval(&expr.output)
            }

            Expr::Bind(bind) => {
                let captures = bind.captures.iter().map(|expr| self.eval(expr)).collect();

                let input = self.eval(&bind.input);

                Value::Action(Action::Bind {
                    captures,
                    input: Rc::new(input),
                    output: &bind.output,
                })
            }

            Expr::Lambda(lambda) => {
                let captures = lambda.captures.iter().map(|expr| self.eval(expr)).collect();

                Value::Lambda {
                    captures,
                    output: &lambda.output,
                }
            }

            Expr::Payload(input) => {
                let input = self.eval(input);

                let Value::Variant(_, payload) = input else {
                    panic!("{input:?}");
                };

                payload.as_ref().clone()
            }

            Expr::Field(input, index) => {
                let input = self.eval(input);

                let Value::Tuple(fields) = input else {
                    panic!("{input:?}, {index}");
                };

                fields[*index].clone()
            }

            Expr::Call(lambda, input) => {
                let lambda = self.eval(lambda);
                let input = self.eval(input);

                if let Value::Extern(r#extern) = lambda {
                    return (r#extern.0)(input);
                }

                let Value::Lambda { captures, output } = lambda else {
                    panic!("{lambda:?}");
                };

                let frame = self.frame;
                self.frame = self.stack.len();
                self.stack.extend(captures.iter().cloned());
                self.stack.push(input);

                let output = self.eval(output);

                self.stack.truncate(self.frame);
                self.frame = frame;

                output
            }

            Expr::Is(input, index) => {
                let input = self.eval(input);

                Value::Bool(matches!(input, Value::Variant(i, _) if i == *index))
            }

            Expr::If(condition, then, otherwise) => {
                let condition = self.eval(condition);

                let Value::Bool(condition) = condition else {
                    panic!();
                };

                if condition {
                    self.eval(then)
                } else {
                    self.eval(otherwise)
                }
            }

            Expr::Intrinsic(intrinsic, inputs) => {
                let mut inputs = inputs.iter().map(|input| self.eval(input));

                macro_rules! binary {
                    ($kind:ident, $op:tt, $as:ident) => {{
                        let lhs = inputs.next().unwrap();
                        let rhs = inputs.next().unwrap();

                        Value::$kind(lhs.$as() $op rhs.$as())
                    }};
                }

                match intrinsic {
                    Intrinsic::NatAdd => binary!(Nat, +, as_nat),
                    Intrinsic::NatSub => binary!(Nat, -, as_nat),
                    Intrinsic::NatMul => binary!(Nat, *, as_nat),
                    Intrinsic::NatXor => binary!(Nat, ^, as_nat),
                    Intrinsic::NatGt => binary!(Bool, >, as_nat),
                    Intrinsic::NatLt => binary!(Bool, <, as_nat),
                    Intrinsic::NatGe => binary!(Bool, >=, as_nat),
                    Intrinsic::NatLe => binary!(Bool, <=, as_nat),
                    Intrinsic::NatEq => binary!(Bool, ==, as_nat),

                    Intrinsic::IntAdd => binary!(Int, +, as_int),
                    Intrinsic::IntSub => binary!(Int, -, as_int),
                    Intrinsic::IntMul => binary!(Int, *, as_int),
                    Intrinsic::IntGt => binary!(Bool, >, as_int),
                    Intrinsic::IntLt => binary!(Bool, <, as_int),
                    Intrinsic::IntGe => binary!(Bool, >=, as_int),
                    Intrinsic::IntLe => binary!(Bool, <=, as_int),
                    Intrinsic::IntEq => binary!(Bool, ==, as_int),

                    Intrinsic::RealAdd => binary!(Real, +, as_real),
                    Intrinsic::RealSub => binary!(Real, -, as_real),
                    Intrinsic::RealMul => binary!(Real, *, as_real),
                    Intrinsic::RealDiv => binary!(Real, /, as_real),
                    Intrinsic::RealGt => binary!(Bool, >, as_real),
                    Intrinsic::RealLt => binary!(Bool, <, as_real),
                    Intrinsic::RealGe => binary!(Bool, >=, as_real),
                    Intrinsic::RealLe => binary!(Bool, <=, as_real),
                    Intrinsic::RealEq => binary!(Bool, ==, as_real),

                    Intrinsic::BoolAnd => binary!(Bool, &&, as_bool),
                    Intrinsic::BoolOr => binary!(Bool, ||, as_bool),

                    Intrinsic::BoolNot => {
                        let input = inputs.next().unwrap();

                        Value::Bool(!input.as_bool())
                    }

                    Intrinsic::FormatNat => {
                        let x = inputs.next().unwrap();
                        Value::Str(format!("{}", x.as_nat()))
                    }

                    Intrinsic::FormatInt => {
                        let x = inputs.next().unwrap();
                        Value::Str(format!("{}", x.as_int()))
                    }

                    Intrinsic::FormatReal => {
                        let x = inputs.next().unwrap();
                        Value::Str(format!("{}", x.as_real()))
                    }

                    Intrinsic::NatToInt => {
                        let input = inputs.next().unwrap();
                        Value::Int(input.as_nat() as i64)
                    }

                    Intrinsic::NatToReal => {
                        let input = inputs.next().unwrap();
                        Value::Real(input.as_nat() as f64)
                    }

                    Intrinsic::IntToNat => {
                        let input = inputs.next().unwrap();
                        Value::Nat(input.as_int() as u64)
                    }

                    Intrinsic::IntToReal => {
                        let input = inputs.next().unwrap();
                        Value::Real(input.as_int() as f64)
                    }

                    Intrinsic::RealToNat => {
                        let input = inputs.next().unwrap();
                        Value::Nat(input.as_real() as u64)
                    }

                    Intrinsic::RealToInt => {
                        let input = inputs.next().unwrap();
                        Value::Int(input.as_real() as i64)
                    }

                    Intrinsic::HashStr
                    | Intrinsic::HashNat
                    | Intrinsic::HashInt
                    | Intrinsic::HashReal => {
                        let input = inputs.next().unwrap();

                        let mut state = DefaultHasher::new();

                        match input {
                            Value::Str(x) => x.hash(&mut state),
                            Value::Nat(x) => x.hash(&mut state),
                            Value::Int(x) => x.hash(&mut state),
                            Value::Real(x) => x.to_bits().hash(&mut state),

                            _ => panic!(),
                        };

                        Value::Nat(state.finish())
                    }

                    Intrinsic::StrEq => {
                        let lhs = inputs.next().unwrap();
                        let rhs = inputs.next().unwrap();

                        Value::Bool(lhs.as_str() == rhs.as_str())
                    }

                    Intrinsic::StrLength => {
                        let input = inputs.next().unwrap();
                        let len = input.as_str().chars().count();
                        Value::Nat(len as u64)
                    }

                    Intrinsic::StrPrepend => {
                        let a = inputs.next().unwrap();
                        let b = inputs.next().unwrap();

                        Value::Str(format!("{}{}", a.as_str(), b.as_str()))
                    }

                    Intrinsic::StrSplitAt => {
                        let string = inputs.next().unwrap();
                        let index = inputs.next().unwrap();

                        let string = string.as_str();
                        let index = index.as_nat();

                        let byte = string
                            .chars()
                            .take(index as usize)
                            .map(char::len_utf8)
                            .sum::<usize>();

                        let (start, end) = string.split_at(byte);

                        Value::Tuple(Rc::new([Value::Str(start.into()), Value::Str(end.into())]))
                    }

                    Intrinsic::StrFind => {
                        let haystack = inputs.next().unwrap();
                        let needle = inputs.next().unwrap();

                        let haystack = haystack.as_str();
                        let needle = needle.as_str();

                        haystack
                            .find(needle)
                            .and_then(|byte| haystack.char_indices().position(|(i, _)| i == byte))
                            .map_or_else(
                                || Value::Variant(0, Rc::new(Value::Tuple(Rc::new([])))),
                                |index| Value::Variant(1, Rc::new(Value::Nat(index as u64))),
                            )
                    }
                }
            }
        }
    }
}
