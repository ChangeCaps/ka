use std::{
    collections::{BTreeMap, HashMap},
    env, fmt,
    hash::{BuildHasherDefault, DefaultHasher, Hash, Hasher},
    io::{self, Write},
    mem,
    rc::Rc,
};

use crate::{
    arena::Id,
    ir::{self, BinOp, Expr, Pat, Program, Var, VarKind},
};

type BuildFastHasher = BuildHasherDefault<seahash::SeaHasher>;
type FastHashMap<K, V> = HashMap<K, V, BuildFastHasher>;
type ExternFn<'a> = Rc<dyn Fn(Vec<Value<'a>>) -> Value<'a>>;

pub struct Runtime<'a> {
    program: &'a Program,
    globals: FastHashMap<Id<Var>, Value<'a>>,
    externs: FastHashMap<&'static str, Extern<'a>>,
}

struct Extern<'a> {
    params: usize,
    f: ExternFn<'a>,
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Value<'a> {
    kind: Rc<ValueKind<'a>>,
}

impl<'a> Value<'a> {
    pub fn new(value: ValueKind<'a>) -> Self {
        Self {
            kind: Rc::new(value),
        }
    }

    pub fn nat(x: u64) -> Self {
        Self::new(ValueKind::Nat(x))
    }

    pub fn int(x: i64) -> Self {
        Self::new(ValueKind::Int(x))
    }

    pub fn num(x: f64) -> Self {
        Self::new(ValueKind::Num(x))
    }

    pub fn str(s: String) -> Self {
        Self::new(ValueKind::Str(s))
    }

    pub fn bool(value: bool) -> Self {
        let variant = match value {
            true => "true",
            false => "false",
        };

        Self::variant(variant, None)
    }

    pub fn unit() -> Self {
        Self::record(Default::default())
    }

    pub fn list(items: impl IntoIterator<Item = Value<'a>>) -> Self {
        items.into_iter().fold(Value::option(None), |rest, value| {
            let tuple = Value::tuple(vec![value, rest].into());
            Value::option(Some(tuple))
        })
    }

    pub fn record(fields: BTreeMap<&'static str, Self>) -> Self {
        Self::new(ValueKind::Record(fields))
    }

    pub fn variant(name: &'static str, value: Option<Self>) -> Self {
        Self::new(ValueKind::Variant(name, value))
    }

    pub fn tuple(fields: Box<[Self]>) -> Self {
        Self::new(ValueKind::Tuple(fields))
    }

    pub fn monad(f: impl Fn() -> Self + 'a) -> Self {
        Self::new(ValueKind::Monad(MonadValue::Extern(Rc::new(f))))
    }

    pub fn pure(value: Self) -> Self {
        Self::new(ValueKind::Monad(MonadValue::Pure(value)))
    }

    pub fn option(option: Option<Self>) -> Self {
        match option {
            Some(value) => Value::variant("some", Some(value)),
            None => Value::variant("none", None),
        }
    }

    pub fn as_nat(&self) -> u64 {
        match self.kind() {
            ValueKind::Nat(x) => *x,
            _ => unreachable!("value is not a nat"),
        }
    }

    pub fn as_int(&self) -> i64 {
        match self.kind() {
            ValueKind::Int(x) => *x,
            _ => unreachable!("value is not a int"),
        }
    }

    pub fn as_real(&self) -> f64 {
        match self.kind() {
            ValueKind::Num(x) => *x,
            _ => unreachable!("value is not a real"),
        }
    }

    pub fn as_bool(&self) -> bool {
        matches!(self.kind(), ValueKind::Variant("true", ..))
    }

    pub fn as_str(&self) -> &str {
        match self.kind() {
            ValueKind::Str(s) => s,
            _ => unreachable!("value is not a str"),
        }
    }

    pub fn kind(&self) -> &ValueKind<'a> {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut ValueKind<'a> {
        Rc::make_mut(&mut self.kind)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueKind<'a> {
    Nat(u64),
    Int(i64),
    Num(f64),
    Str(String),
    Variant(&'static str, Option<Value<'a>>),
    Tuple(Box<[Value<'a>]>),
    Record(BTreeMap<&'static str, Value<'a>>),
    Monad(MonadValue<'a>),
    Lambda(LambdaValue<'a>),
}

impl Hash for ValueKind<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        mem::discriminant(self).hash(state);

        match self {
            ValueKind::Nat(x) => x.hash(state),
            ValueKind::Int(x) => x.hash(state),
            ValueKind::Num(x) => x.to_bits().hash(state),
            ValueKind::Str(x) => x.hash(state),

            ValueKind::Variant(name, value) => {
                name.hash(state);
                value.hash(state);
            }

            ValueKind::Tuple(values) => values.hash(state),

            ValueKind::Record(fields) => fields.hash(state),

            ValueKind::Monad(_) | ValueKind::Lambda(_) => {}
        }
    }
}

impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn recurse(value: &Value, f: &mut fmt::Formatter<'_>, precedence: u8) -> fmt::Result {
            match value.kind() {
                ValueKind::Nat(x) => write!(f, "{x}"),
                ValueKind::Int(x) => write!(f, "{x}"),
                ValueKind::Num(x) => write!(f, "{x}"),
                ValueKind::Str(x) => write!(f, "\"{x}\""),

                ValueKind::Variant(name, value) => {
                    if precedence >= 2 {
                        write!(f, "(")?;
                    }

                    write!(f, ":{name}")?;

                    if let Some(value) = value {
                        write!(f, " ")?;
                        recurse(value, f, 2)?;
                    }

                    if precedence >= 2 {
                        write!(f, ")")?;
                    }

                    Ok(())
                }

                ValueKind::Tuple(fields) => {
                    if precedence >= 1 {
                        write!(f, "(")?;
                    }

                    for (i, field) in fields.iter().enumerate() {
                        recurse(field, f, 1)?;

                        if i < fields.len() - 1 {
                            write!(f, ", ")?;
                        }
                    }

                    if precedence >= 1 {
                        write!(f, ")")?;
                    }

                    Ok(())
                }

                ValueKind::Record(fields) => {
                    if fields.is_empty() {
                        return write!(f, "{{}}");
                    }

                    write!(f, "{{ ")?;

                    for (i, (name, value)) in fields.iter().enumerate() {
                        write!(f, "{name}: ")?;
                        recurse(value, f, 0)?;

                        if i < fields.len() - 1 {
                            write!(f, "; ")?;
                        }
                    }

                    write!(f, " }}")
                }

                ValueKind::Monad(..) => write!(f, "{{monad}}"),
                ValueKind::Lambda(..) => write!(f, "{{lambda}}"),
            }
        }

        recurse(self, f, 0)
    }
}

#[derive(Clone, Debug)]
pub enum LambdaValue<'a> {
    Intern {
        pat: &'a Pat,
        expr: &'a Expr,
        vars: FastHashMap<Id<Var>, Value<'a>>,
    },

    Extern {
        id: &'static str,
        args: Vec<Value<'a>>,
    },
}

impl PartialEq for LambdaValue<'_> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[derive(Clone)]
pub enum MonadValue<'a> {
    Pure(Value<'a>),

    Extern(Rc<dyn Fn() -> Value<'a> + 'a>),

    Bind {
        input: Value<'a>,
        pat: &'a Pat,
        expr: &'a Expr,
        vars: FastHashMap<Id<Var>, Value<'a>>,
    },
}

impl PartialEq for MonadValue<'_> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl fmt::Debug for MonadValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pure(value) => f.debug_tuple("Pure").field(value).finish(),
            Self::Extern(..) => f.debug_tuple("Extern").finish(),
            Self::Bind {
                input,
                pat,
                expr,
                vars,
            } => f
                .debug_struct("Bind")
                .field("input", input)
                .field("pat", pat)
                .field("expr", expr)
                .field("vars", vars)
                .finish(),
        }
    }
}

#[derive(Clone)]
struct Frame<'a> {
    vars: FastHashMap<Id<Var>, Value<'a>>,
}

impl Frame<'_> {
    fn new() -> Self {
        Self {
            vars: HashMap::default(),
        }
    }
}

impl<'a> Runtime<'a> {
    pub fn new(program: &'a Program) -> Self {
        let mut rt = Self {
            program,
            globals: HashMap::default(),
            externs: HashMap::default(),
        };

        rt.add_extern("io::readln", 0, |_| {
            Value::monad(|| {
                let line = io::stdin().lines().next().unwrap().unwrap();
                Value::str(line)
            })
        });

        rt.add_extern("io::print", 1, |args| {
            Value::monad(move || {
                let s = args[0].as_str();

                let mut stdout = io::stdout().lock();
                let _ = stdout.write_all(s.as_bytes());
                let _ = stdout.flush();

                Value::unit()
            })
        });

        rt.add_extern("env::args", 0, |_| {
            Value::monad(move || Value::list(env::args().map(Value::str).rev()))
        });

        rt
    }

    fn add_extern(
        &mut self,
        name: &'static str,
        params: usize,
        function: impl Fn(Vec<Value<'a>>) -> Value<'a> + 'static,
    ) {
        self.externs.insert(
            name,
            Extern {
                params,
                f: Rc::new(function),
            },
        );
    }

    pub fn run(&mut self, main: Id<Var>) {
        let mut frame = Frame::new();

        for id in self.program.order.iter().copied() {
            let global = &self.program.globals[id];
            let value = self.eval_expr(&frame, &global.expr);
            self.assign_pat(&mut frame, &global.pat, value);
        }

        let main = self.globals[&main].clone();

        let ValueKind::Monad(main) = main.kind().clone() else {
            unreachable!();
        };

        self.eval_monad(main);
    }

    fn assign_pat(&mut self, frame: &mut Frame<'a>, pat: &Pat, value: Value<'a>) {
        match pat {
            Pat::Wild(..) => {}

            Pat::Bind(pat) => match self.program.vars[pat.var].kind {
                VarKind::Global { .. } => {
                    self.globals.insert(pat.var, value);
                }

                VarKind::Extern(..) => unreachable!(),

                VarKind::Local => {
                    frame.vars.insert(pat.var, value);
                }
            },

            Pat::Variant(pat) => {
                let ValueKind::Variant(_, value) = value.kind() else {
                    unreachable!();
                };

                if let Some(ref pat) = pat.payload
                    && let Some(value) = value
                {
                    self.assign_pat(frame, pat, value.clone());
                }
            }

            Pat::Tuple(pat) => {
                let ValueKind::Tuple(fields) = value.kind() else {
                    unreachable!();
                };

                for (pat, value) in pat.fields.iter().zip(fields) {
                    self.assign_pat(frame, pat, value.clone());
                }
            }

            Pat::Error(..) => unreachable!(),
        }
    }

    fn check_pat(&mut self, pat: &Pat, value: &Value) -> bool {
        match pat {
            Pat::Wild(..) | Pat::Bind(..) => true,

            Pat::Variant(pat) => {
                let ValueKind::Variant(name, value) = value.kind() else {
                    unreachable!();
                };

                match (pat.payload.as_ref(), value) {
                    (Some(inner), Some(value)) => *name == pat.name && self.check_pat(inner, value),
                    (_, _) => *name == pat.name,
                }
            }

            Pat::Tuple(pat) => {
                let ValueKind::Tuple(fields) = value.kind() else {
                    unreachable!();
                };

                pat.fields
                    .iter()
                    .zip(fields)
                    .all(|(pat, value)| self.check_pat(pat, value))
            }

            Pat::Error(..) => unreachable!(),
        }
    }

    fn eval_monad(&mut self, monad: MonadValue<'a>) -> Value<'a> {
        match monad {
            MonadValue::Pure(value) => value,

            MonadValue::Extern(f) => f(),

            MonadValue::Bind {
                input,
                pat,
                expr,
                vars,
            } => {
                let ValueKind::Monad(monad) = input.kind() else {
                    unreachable!();
                };

                let input = self.eval_monad(monad.clone());

                let mut frame = Frame::new();
                frame.vars = vars;
                self.assign_pat(&mut frame, pat, input);

                let monad = self.eval_expr(&frame, expr);

                let ValueKind::Monad(monad) = monad.kind() else {
                    unreachable!();
                };

                self.eval_monad(monad.clone())
            }
        }
    }

    fn eval_expr(&mut self, frame: &Frame<'a>, expr: &'a Expr) -> Value<'a> {
        match expr {
            Expr::Value(expr) => match expr.value {
                ir::Value::Num(x) => Value::num(x),
                ir::Value::Str(ref cow) => Value::str(cow.to_string()),
            },

            Expr::Var(expr) => match self.program.vars[expr.var].kind {
                VarKind::Global { .. } => self.globals.get(&expr.var).unwrap().clone(),

                VarKind::Extern(id) => {
                    let r#extern = &self.program.externs[id];

                    if let Some(r#extern) = self.externs.get(r#extern.id) {
                        if r#extern.params == 0 {
                            return (r#extern.f)(Vec::new());
                        }
                    } else {
                        unreachable!("extern `{}` not found", r#extern.id);
                    }

                    Value::new(ValueKind::Lambda(LambdaValue::Extern {
                        id: r#extern.id,
                        args: Vec::new(),
                    }))
                }

                VarKind::Local => match frame.vars.get(&expr.var).cloned() {
                    Some(value) => value,
                    None => {
                        unreachable!("variant not defined `{}`", self.program.vars[expr.var].name);
                    }
                },
            },

            Expr::Let(expr) => {
                let mut frame = frame.clone();
                let value = self.eval_expr(&frame, &expr.input);
                self.assign_pat(&mut frame, &expr.pat, value);
                self.eval_expr(&frame, &expr.expr)
            }

            Expr::Bind(expr) => {
                let input = self.eval_expr(frame, &expr.input);

                let vars = self.program.scopes[expr.scope]
                    .captures
                    .iter()
                    .copied()
                    .map(|id| (id, frame.vars[&id].clone()))
                    .collect();

                Value::new(ValueKind::Monad(MonadValue::Bind {
                    input: input.clone(),
                    pat: &expr.pat,
                    expr: &expr.expr,
                    vars,
                }))
            }

            Expr::Pure(expr) => {
                let value = self.eval_expr(frame, &expr.expr);
                Value::pure(value)
            }

            Expr::Call(expr) => {
                let mut lambda = self.eval_expr(frame, &expr.lambda);
                let input = self.eval_expr(frame, &expr.input);

                let ValueKind::Lambda(value) = lambda.kind_mut() else {
                    unreachable!()
                };

                match value {
                    LambdaValue::Intern { pat, expr, vars } => {
                        let mut frame = Frame::new();
                        frame.vars = mem::take(vars);

                        self.assign_pat(&mut frame, pat, input);
                        self.eval_expr(&frame, expr)
                    }

                    LambdaValue::Extern { id, args } => {
                        args.push(input);

                        let r#extern = &self.externs[id];
                        if r#extern.params != args.len() {
                            return lambda;
                        }

                        (r#extern.f)(mem::take(args))
                    }
                }
            }

            Expr::With(expr) => {
                let mut input = self.eval_expr(frame, &expr.input);

                let ValueKind::Record(fields) = input.kind_mut() else {
                    unreachable!();
                };

                for field in &expr.fields {
                    let value = self.eval_expr(frame, &field.expr);
                    fields.insert(field.name, value);
                }

                input
            }

            Expr::Field(expr) => {
                let input = self.eval_expr(frame, &expr.input);

                let ValueKind::Record(fields) = input.kind() else {
                    unreachable!();
                };

                fields[expr.name].clone()
            }

            Expr::Lambda(expr) => {
                let vars = self.program.scopes[expr.scope]
                    .captures
                    .iter()
                    .copied()
                    .map(|id| (id, frame.vars[&id].clone()))
                    .collect();

                Value::new(ValueKind::Lambda(LambdaValue::Intern {
                    pat: &expr.input,
                    expr: &expr.expr,
                    vars,
                }))
            }

            Expr::Variant(expr) => {
                let value = expr.expr.as_ref().map(|expr| self.eval_expr(frame, expr));

                Value::variant(expr.name, value)
            }

            Expr::Record(expr) => {
                let fields = expr
                    .fields
                    .iter()
                    .map(|field| {
                        let value = self.eval_expr(frame, &field.expr);
                        (field.name, value)
                    })
                    .collect();

                Value::record(fields)
            }

            Expr::Binary(expr) => {
                let lhs = self.eval_expr(frame, &expr.lhs);
                let rhs = self.eval_expr(frame, &expr.rhs);

                match expr.op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        let mut value = lhs;

                        match (value.kind_mut(), rhs.kind()) {
                            (ValueKind::Nat(lhs), ValueKind::Nat(rhs)) => match expr.op {
                                BinOp::Add => *lhs += rhs,
                                BinOp::Sub => *lhs -= rhs,
                                BinOp::Mul => *lhs *= rhs,
                                _ => unreachable!(),
                            },

                            (ValueKind::Int(lhs), ValueKind::Int(rhs)) => match expr.op {
                                BinOp::Add => *lhs += rhs,
                                BinOp::Sub => *lhs -= rhs,
                                BinOp::Mul => *lhs *= rhs,
                                _ => unreachable!(),
                            },

                            (ValueKind::Num(lhs), ValueKind::Num(rhs)) => match expr.op {
                                BinOp::Add => *lhs += rhs,
                                BinOp::Sub => *lhs -= rhs,
                                BinOp::Mul => *lhs *= rhs,
                                BinOp::Div => *lhs /= rhs,
                                _ => unreachable!(),
                            },

                            (_, _) => unreachable!(),
                        }

                        value
                    }

                    BinOp::Gt | BinOp::Lt | BinOp::GtEq | BinOp::LtEq => {
                        match (lhs.kind(), rhs.kind()) {
                            (ValueKind::Nat(lhs), ValueKind::Nat(rhs)) => match expr.op {
                                BinOp::Gt => Value::bool(lhs > rhs),
                                BinOp::Lt => Value::bool(lhs < rhs),
                                BinOp::GtEq => Value::bool(lhs >= rhs),
                                BinOp::LtEq => Value::bool(lhs <= rhs),
                                _ => unreachable!(),
                            },

                            (ValueKind::Int(lhs), ValueKind::Int(rhs)) => match expr.op {
                                BinOp::Gt => Value::bool(lhs > rhs),
                                BinOp::Lt => Value::bool(lhs < rhs),
                                BinOp::GtEq => Value::bool(lhs >= rhs),
                                BinOp::LtEq => Value::bool(lhs <= rhs),
                                _ => unreachable!(),
                            },

                            (ValueKind::Num(lhs), ValueKind::Num(rhs)) => match expr.op {
                                BinOp::Gt => Value::bool(lhs > rhs),
                                BinOp::Lt => Value::bool(lhs < rhs),
                                BinOp::GtEq => Value::bool(lhs >= rhs),
                                BinOp::LtEq => Value::bool(lhs <= rhs),
                                _ => unreachable!(),
                            },

                            (_, _) => unreachable!(),
                        }
                    }

                    BinOp::And | BinOp::Or => {
                        let lhs = lhs.as_bool();
                        let rhs = rhs.as_bool();

                        match expr.op {
                            BinOp::And => Value::bool(lhs && rhs),
                            BinOp::Or => Value::bool(lhs || rhs),
                            _ => unreachable!(),
                        }
                    }

                    BinOp::Eq => Value::bool(lhs == rhs),
                    BinOp::Ne => Value::bool(lhs != rhs),
                }
            }

            Expr::Tuple(expr) => {
                let fields = expr
                    .fields
                    .iter()
                    .map(|expr| self.eval_expr(frame, expr))
                    .collect::<Vec<_>>();

                Value::tuple(fields.into())
            }

            Expr::Match(expr) => {
                let value = self.eval_expr(frame, &expr.expr);

                for arm in &expr.arms {
                    if !self.check_pat(&arm.pat, &value) {
                        continue;
                    }

                    let mut frame = frame.clone();
                    self.assign_pat(&mut frame, &arm.pat, value.clone());
                    return self.eval_expr(&frame, &arm.expr);
                }

                unreachable!("no arm matched")
            }

            Expr::Intrinsic(expr) => {
                let inputs = expr
                    .inputs
                    .iter()
                    .map(|expr| self.eval_expr(frame, expr))
                    .collect::<Vec<_>>();

                match expr.intrinsic {
                    ir::Intrinsic::StringLength => {
                        let input = inputs[0].as_str();
                        Value::num(input.chars().count() as f64)
                    }

                    ir::Intrinsic::StringFormat => Value::str(inputs[0].to_string()),

                    ir::Intrinsic::StringPrepend => {
                        let lhs = inputs[0].as_str();
                        let rhs = inputs[1].as_str();

                        Value::str(lhs.to_string() + rhs)
                    }

                    ir::Intrinsic::StringSplitAt => {
                        let haystack = inputs[0].as_str();
                        let n = inputs[1].as_nat() as usize;

                        let option = haystack.char_indices().nth(n).map(|(i, _)| {
                            let (start, end) = haystack.split_at(i);
                            let start = Value::str(start.into());
                            let end = Value::str(end.into());

                            Value::tuple(vec![start, end].into())
                        });

                        Value::option(option)
                    }

                    ir::Intrinsic::StringFind => {
                        let haystack = inputs[0].as_str();
                        let needle = inputs[1].as_str();

                        let option = haystack.find(needle).map(|idx| {
                            let (n, _) = haystack.char_indices().find(|(i, _)| *i == idx).unwrap();
                            Value::num(n as f64)
                        });

                        Value::option(option)
                    }

                    ir::Intrinsic::Hash => {
                        let mut state = DefaultHasher::new();
                        inputs[0].hash(&mut state);

                        Value::nat(state.finish())
                    }
                }
            }

            Expr::Error(..) => unreachable!(),
        }
    }
}
