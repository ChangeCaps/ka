use std::{
    borrow::Cow,
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    arena::Id,
    ir,
    mir::{
        Bind, Constant, Constructor, Entry, Expr, Extern, Global, Intrinsic, Lambda, Let, Local, Ty,
    },
};

pub fn lower(program: &ir::Program, main: Id<ir::Var>) -> Entry {
    let mut lowerer = Lowerer::new(program);

    let var = &lowerer.program.vars[main];
    let ir::VarKind::Global(global) = var.kind else {
        unreachable!();
    };

    let ty = Ty::action(Ty::unit());
    let global = lowerer.global(global, ty);

    assert!(!lowerer.globals.is_empty());
    assert_eq!(global.index, 0);

    let expr = lowerer.globals[0].clone();

    Entry {
        globals: lowerer.globals,
        output: Rc::new(expr),
    }
}

struct Lowerer<'a> {
    program: &'a ir::Program,

    globals: Vec<Expr>,
    global_map: HashMap<(Id<ir::Global>, Ty), Global>,

    dynamics: HashMap<Ty, Global>,
}

impl<'a> Lowerer<'a> {
    fn new(program: &'a ir::Program) -> Self {
        Self {
            program,

            globals: Vec::new(),
            global_map: HashMap::new(),

            dynamics: HashMap::new(),
        }
    }

    fn global(&mut self, id: Id<ir::Global>, ty: Ty) -> Global {
        let key = (id, ty);

        if let Some(global) = self.global_map.get(&key) {
            return global.clone();
        }

        let (id, ty) = key;

        let global = Global {
            index: self.globals.len(),
            ty: ty.clone(),
        };

        self.globals.push(Expr::unit());
        self.global_map.insert((id, ty.clone()), global.clone());

        self.globals[global.index] = {
            let global = &self.program.globals[id];
            let mut subst = HashMap::new();

            self.unify_ty(&global.pat.ty(), &ty, &HashMap::new(), &mut subst);

            let mut locals = HashMap::new();
            let mut scoped = ScopedLowerer {
                lowerer: self,
                locals: &mut 0,
                vars: &mut locals,
                subst: &subst,
            };

            scoped.expr(&global.expr)
        };

        global
    }

    fn unify_ty(
        &self,
        actual: &ir::Ty,
        expected: &Ty,
        generics: &HashMap<Id<ir::Bound>, ir::Ty>,
        subst: &mut HashMap<Id<ir::Bound>, Ty>,
    ) {
        match (actual, expected) {
            (ir::Ty::Infer(bound) | ir::Ty::Generic(ir::GenericTy { bound, .. }), _) => {
                if subst.contains_key(bound) {
                    return;
                }

                if let Some(actual) = generics.get(bound) {
                    self.unify_ty(actual, expected, generics, subst);
                } else if let Some(actual) = self.program.subst.get(bound) {
                    self.unify_ty(actual, expected, generics, subst);
                } else {
                    subst.insert(*bound, expected.clone());
                }
            }

            (ir::Ty::Str, Ty::Str)
            | (ir::Ty::Numeric(ir::Numeric::Nat), Ty::Nat)
            | (ir::Ty::Numeric(ir::Numeric::Int), Ty::Int)
            | (ir::Ty::Numeric(ir::Numeric::Real), Ty::Real) => {}

            (ir::Ty::Lambda(actual), Ty::Lambda(input, output)) => {
                self.unify_ty(&actual.input, input, generics, subst);
                self.unify_ty(&actual.output, output, generics, subst);
            }

            (ir::Ty::Monad(actual), Ty::Action(expected)) => {
                self.unify_ty(actual, expected, generics, subst)
            }

            (ir::Ty::Tuple(actual), Ty::Tuple(expected)) => {
                for (actual, expected) in actual.iter().zip(expected.iter()) {
                    self.unify_ty(actual, expected, generics, subst);
                }
            }

            (ir::Ty::Record(actual), Ty::Record(expected)) => {
                let mut fields = actual.fields.iter().collect::<Vec<_>>();
                fields.sort_by_key(|field| field.name);

                for (actual, (_, expected)) in fields.iter().zip(expected.as_ref()) {
                    self.unify_ty(&actual.ty, expected, generics, subst);
                }
            }

            (ir::Ty::Union(actual), Ty::Union(expected)) => {
                let mut variants = actual.variants.iter().collect::<Vec<_>>();
                variants.sort_by_key(|variant| variant.name);

                for (actual, (_, expected)) in variants.iter().zip(expected.as_ref()) {
                    if let Some(ref payload) = actual.payload {
                        self.unify_ty(payload, expected, generics, subst);
                    }
                }
            }

            (ir::Ty::Union(..), Ty::Bool) => {}

            (ir::Ty::Alias(actual), expected) => {
                let alias = &self.program.aliases[actual.alias];
                let mut generics = HashMap::new();

                for (generic, arg) in alias.params.iter().zip(&actual.args) {
                    generics.insert(generic.bound, arg.clone());
                }

                self.unify_ty(&alias.ty, expected, &generics, subst);
            }

            (_, Ty::Boxed(..)) => {}

            (_, _) => unreachable!("{:?}, {:?}", actual, expected),
        }
    }
}

struct ScopedLowerer<'a, 'b> {
    lowerer: &'a mut Lowerer<'b>,
    locals: &'a mut usize,
    vars: &'a mut HashMap<Id<ir::Var>, Expr>,
    subst: &'a HashMap<Id<ir::Bound>, Ty>,
}

impl ScopedLowerer<'_, '_> {
    fn expr(&mut self, expr: &ir::Expr) -> Expr {
        match expr {
            ir::Expr::Value(expr) => match expr.value {
                ir::Value::Num(x) => {
                    let constant = match self.ty(&expr.ty) {
                        Ty::Nat => Constant::Nat(x as u64),
                        Ty::Int => Constant::Int(x as i64),
                        Ty::Real => Constant::Real(x),

                        _ => panic!("invalid type for numeric value"),
                    };

                    Expr::Constant(constant)
                }

                ir::Value::Str(ref s) => Expr::Constant(Constant::Str(s.clone())),
            },

            ir::Expr::Var(expr) => {
                let var = &self.program.vars[expr.var];

                match var.kind {
                    ir::VarKind::Global(id) => {
                        let ty = self.ty(&expr.ty);
                        let global = self.global(id, ty);

                        Expr::Global(global)
                    }

                    ir::VarKind::Extern(id) => {
                        let r#extern = &self.program.externs[id];
                        let ty = self.ty(&r#extern.ty);

                        Expr::Extern(Extern {
                            id: r#extern.id,
                            ty,
                        })
                    }

                    ir::VarKind::Local => self.vars[&expr.var].clone(),
                }
            }

            ir::Expr::Let(expr) => {
                let input = self.expr(&expr.input);
                let input = Rc::new(input);

                let ty = self.ty(&expr.pat.ty());
                let local = self.add_local(ty);

                self.pat(&expr.pat, &Expr::Local(local.clone()));

                let output = self.expr(&expr.expr);
                let output = Rc::new(output);

                Expr::Let(Let {
                    input,
                    local,
                    output,
                })
            }

            ir::Expr::Bind(expr) => {
                let input = self.expr(&expr.input);
                let input = Rc::new(input);

                let captures = self.program.scopes[expr.scope]
                    .captures
                    .iter()
                    .map(|var| self.vars[var].clone())
                    .collect::<Vec<_>>();

                let mut vars = self.program.scopes[expr.scope]
                    .captures
                    .iter()
                    .enumerate()
                    .map(|(index, var)| {
                        let ty = &self.program.vars[*var].ty;
                        let ty = self.ty(ty);

                        let local = Local { index, ty };

                        (*var, Expr::Local(local))
                    })
                    .collect::<HashMap<_, _>>();

                let mut lowerer = ScopedLowerer {
                    lowerer: self.lowerer,
                    locals: &mut vars.len(),
                    vars: &mut vars,
                    subst: self.subst,
                };

                let ty = lowerer.ty(&expr.pat.ty());

                let local = lowerer.add_local(ty);

                lowerer.pat(&expr.pat, &Expr::Local(local.clone()));

                let output = lowerer.expr(&expr.expr);
                let output = Rc::new(output);

                Expr::Bind(Bind {
                    captures,
                    input,
                    local,
                    output,
                })
            }

            ir::Expr::Pure(expr) => {
                let input = self.expr(&expr.expr);
                let input = Rc::new(input);

                Expr::Construct(Constructor::Pure(input))
            }

            ir::Expr::Call(expr) => {
                let lambda = self.expr(&expr.lambda);
                let input = self.expr(&expr.input);

                let lambda = Rc::new(lambda);
                let input = Rc::new(input);

                Expr::Call(lambda, input)
            }

            ir::Expr::With(expr) => {
                let input = self.expr(&expr.input);
                let input = Rc::new(input);

                let Ty::Record(fields) = self.ty(&expr.input.ty()) else {
                    panic!();
                };

                let fields = fields.iter().enumerate().map(|(i, (name, _))| {
                    match expr.fields.iter().find(|field| field.name == *name) {
                        Some(field) => self.expr(&field.expr),
                        None => Expr::Field(input.clone(), i),
                    }
                });

                Expr::Construct(Constructor::Tuple(fields.collect()))
            }

            ir::Expr::Field(expr) => {
                let input = self.expr(&expr.input);
                let index = self.field_index(&expr.input.ty(), expr.name);

                Expr::Field(Rc::new(input), index)
            }

            ir::Expr::Lambda(expr) => {
                let captures = self.program.scopes[expr.scope]
                    .captures
                    .iter()
                    .map(|var| self.vars[var].clone())
                    .collect::<Vec<_>>();

                let mut vars = self.program.scopes[expr.scope]
                    .captures
                    .iter()
                    .enumerate()
                    .map(|(index, var)| {
                        let ty = &self.program.vars[*var].ty;
                        let ty = self.ty(ty);

                        let local = Local { index, ty };

                        (*var, Expr::Local(local))
                    })
                    .collect::<HashMap<_, _>>();

                let mut lowerer = ScopedLowerer {
                    lowerer: self.lowerer,
                    locals: &mut vars.len(),
                    vars: &mut vars,
                    subst: self.subst,
                };

                let ty = lowerer.ty(&expr.input.ty());

                let input = lowerer.add_local(ty);

                lowerer.pat(&expr.input, &Expr::Local(input.clone()));

                let output = lowerer.expr(&expr.expr);
                let output = Rc::new(output);

                Expr::Lambda(Lambda {
                    captures,
                    input,
                    output,
                })
            }

            ir::Expr::Variant(expr) => {
                let ty = self.ty(&expr.ty);

                if let Ty::Bool = ty {
                    return match expr.name {
                        "true" => Expr::Constant(Constant::Bool(true)),
                        "false" => Expr::Constant(Constant::Bool(false)),
                        _ => panic!(),
                    };
                }

                let payload = match expr.expr {
                    Some(ref payload) => self.expr(payload),
                    None => Expr::unit(),
                };

                let payload = Rc::new(payload);
                let index = self.variant_index(&expr.ty, expr.name);

                Expr::Construct(Constructor::Variant(index, payload))
            }

            ir::Expr::Record(expr) => {
                let mut fields = expr.fields.iter().collect::<Vec<_>>();
                fields.sort_by_key(|field| field.name);

                let fields = fields
                    .into_iter()
                    .map(|field| self.expr(&field.expr))
                    .collect();

                Expr::Construct(Constructor::Tuple(fields))
            }

            ir::Expr::Unary(expr) => {
                let input = self.expr(&expr.input);

                let ty = self.ty(&expr.input.ty());

                let intrinsic = match (expr.op, &ty) {
                    (ir::UnOp::Nat, Ty::Nat)
                    | (ir::UnOp::Int, Ty::Int)
                    | (ir::UnOp::Real, Ty::Real) => return input.clone(),

                    (ir::UnOp::Nat, Ty::Int) => Intrinsic::IntToNat,
                    (ir::UnOp::Nat, Ty::Real) => Intrinsic::RealToNat,

                    (ir::UnOp::Int, Ty::Nat) => Intrinsic::NatToInt,
                    (ir::UnOp::Int, Ty::Real) => Intrinsic::RealToInt,

                    (ir::UnOp::Real, Ty::Nat) => Intrinsic::NatToReal,
                    (ir::UnOp::Real, Ty::Int) => Intrinsic::IntToReal,

                    (op, ty) => unreachable!("{op:?}, {ty:?}"),
                };

                Expr::Intrinsic(intrinsic, Rc::new([input]))
            }

            ir::Expr::Binary(expr) => {
                let lhs = self.expr(&expr.lhs);
                let rhs = self.expr(&expr.rhs);

                let ty = self.ty(&expr.lhs.ty());

                let intrinsic = match (expr.op, &ty) {
                    (ir::BinOp::Add, Ty::Nat) => Intrinsic::NatAdd,
                    (ir::BinOp::Sub, Ty::Nat) => Intrinsic::NatSub,
                    (ir::BinOp::Mul, Ty::Nat) => Intrinsic::NatMul,
                    (ir::BinOp::Gt, Ty::Nat) => Intrinsic::NatGt,
                    (ir::BinOp::Lt, Ty::Nat) => Intrinsic::NatLt,
                    (ir::BinOp::Ge, Ty::Nat) => Intrinsic::NatGe,
                    (ir::BinOp::Le, Ty::Nat) => Intrinsic::NatLe,

                    (ir::BinOp::Add, Ty::Int) => Intrinsic::IntAdd,
                    (ir::BinOp::Sub, Ty::Int) => Intrinsic::IntSub,
                    (ir::BinOp::Mul, Ty::Int) => Intrinsic::IntMul,
                    (ir::BinOp::Gt, Ty::Int) => Intrinsic::IntGt,
                    (ir::BinOp::Lt, Ty::Int) => Intrinsic::IntLt,
                    (ir::BinOp::Ge, Ty::Int) => Intrinsic::IntGe,
                    (ir::BinOp::Le, Ty::Int) => Intrinsic::IntLe,

                    (ir::BinOp::Add, Ty::Real) => Intrinsic::RealAdd,
                    (ir::BinOp::Sub, Ty::Real) => Intrinsic::RealSub,
                    (ir::BinOp::Mul, Ty::Real) => Intrinsic::RealMul,
                    (ir::BinOp::Div, Ty::Real) => Intrinsic::RealDiv,
                    (ir::BinOp::Gt, Ty::Real) => Intrinsic::RealGt,
                    (ir::BinOp::Lt, Ty::Real) => Intrinsic::RealLt,
                    (ir::BinOp::Ge, Ty::Real) => Intrinsic::RealGe,
                    (ir::BinOp::Le, Ty::Real) => Intrinsic::RealLe,

                    (ir::BinOp::Eq, _) => return self.eq(&ty, &lhs, &rhs, &mut Vec::new()),

                    (ir::BinOp::And, Ty::Bool) => Intrinsic::BoolAnd,
                    (ir::BinOp::Or, Ty::Bool) => Intrinsic::BoolOr,

                    (op, ty) => unreachable!("{op:?}, {ty:?}"),
                };

                Expr::Intrinsic(intrinsic, Rc::new([lhs, rhs]))
            }

            ir::Expr::Tuple(expr) => {
                let fields = expr.fields.iter().map(|field| self.expr(field)).collect();

                Expr::Construct(Constructor::Tuple(fields))
            }

            ir::Expr::Match(expr) => {
                let input = self.expr(&expr.expr);
                let ty = self.ty(&expr.expr.ty());

                let mut arms = expr.arms.iter().rev();
                let default = arms.next().unwrap();

                self.pat(&default.pat, &input);

                let mut expr = self.expr(&default.expr);

                for arm in arms {
                    let condition = self.check(&arm.pat, &input, &ty);

                    self.pat(&arm.pat, &input);

                    let output = self.expr(&arm.expr);

                    expr = Expr::If(Rc::new(condition), Rc::new(output), Rc::new(expr));
                }

                expr
            }

            ir::Expr::Intrinsic(expr) => {
                let mut inputs = expr.inputs.iter().map(|expr| self.expr(expr));

                let intrinsic = match expr.intrinsic {
                    ir::Intrinsic::Dynamic => {
                        let input = inputs.next().unwrap();

                        let ty = expr.inputs[0].ty();
                        let ty = self.ty(&ty);

                        return self.dynamic(&ty, &input, &mut Vec::new());
                    }

                    ir::Intrinsic::FormatNat => Intrinsic::FormatNat,
                    ir::Intrinsic::FormatInt => Intrinsic::FormatInt,
                    ir::Intrinsic::FormatReal => Intrinsic::FormatReal,

                    ir::Intrinsic::HashStr => Intrinsic::HashStr,
                    ir::Intrinsic::HashNat => Intrinsic::HashNat,
                    ir::Intrinsic::HashInt => Intrinsic::HashInt,
                    ir::Intrinsic::HashReal => Intrinsic::HashReal,

                    ir::Intrinsic::NatXor => Intrinsic::NatXor,

                    ir::Intrinsic::StrLength => Intrinsic::StrLength,
                    ir::Intrinsic::StrPrepend => Intrinsic::StrPrepend,
                    ir::Intrinsic::StrSplitAt => Intrinsic::StrSplitAt,
                    ir::Intrinsic::StrFind => Intrinsic::StrFind,
                };

                Expr::Intrinsic(intrinsic, inputs.collect())
            }

            ir::Expr::Error(..) => unreachable!(),
        }
    }

    fn pat(&mut self, pat: &ir::Pat, expr: &Expr) {
        match pat {
            ir::Pat::Wild(..) => {}

            ir::Pat::Bind(pat) => {
                self.vars.insert(pat.var, expr.clone());
            }

            ir::Pat::Variant(pat) => {
                if let Some(ref payload) = pat.payload {
                    let expr = Rc::new(expr.clone());
                    let expr = Expr::Payload(expr);
                    self.pat(payload, &expr);
                }
            }

            ir::Pat::Tuple(pat) => {
                for (i, pat) in pat.fields.iter().enumerate() {
                    let expr = Rc::new(expr.clone());
                    let expr = Expr::Field(expr, i);
                    self.pat(pat, &expr);
                }
            }

            ir::Pat::Error(..) => unreachable!(),
        }
    }

    fn check(&mut self, pat: &ir::Pat, input: &Expr, ty: &Ty) -> Expr {
        match pat {
            ir::Pat::Wild(..) | ir::Pat::Bind(..) => Expr::Constant(Constant::Bool(true)),

            ir::Pat::Variant(pat) => {
                if let Ty::Bool = ty {
                    return match pat.name {
                        "true" => input.clone(),
                        "false" => Expr::Intrinsic(Intrinsic::BoolNot, Rc::new([input.clone()])),
                        _ => panic!(),
                    };
                }

                let Ty::Union(variants) = ty else {
                    panic!();
                };

                let i = variants
                    .iter()
                    .position(|(name, _)| *name == pat.name)
                    .unwrap();

                let (_, ref ty) = variants[i];

                let input = Rc::new(input.clone());
                let expr = Expr::Is(input.clone(), i);

                match pat.payload {
                    None => expr,
                    Some(ref payload) => {
                        let input = Expr::Payload(input);
                        let check = self.check(payload, &input, ty);
                        Expr::Intrinsic(Intrinsic::BoolAnd, Rc::new([expr, check]))
                    }
                }
            }

            ir::Pat::Tuple(pat) => {
                let input = Rc::new(input.clone());

                let Ty::Tuple(fields) = ty else {
                    panic!();
                };

                pat.fields
                    .iter()
                    .enumerate()
                    .map(|(i, pat)| {
                        let input = Expr::Field(input.clone(), i);
                        self.check(pat, &input, &fields[i])
                    })
                    .reduce(|a, b| Expr::Intrinsic(Intrinsic::BoolAnd, Rc::new([a, b])))
                    .unwrap()
            }

            ir::Pat::Error(..) => unreachable!(),
        }
    }

    fn eq<'a>(&mut self, ty: &'a Ty, lhs: &Expr, rhs: &Expr, stack: &mut Vec<&'a Ty>) -> Expr {
        stack.push(ty);

        let expr = match ty {
            Ty::Str => Expr::Intrinsic(Intrinsic::StrEq, Rc::new([lhs.clone(), rhs.clone()])),
            Ty::Nat => Expr::Intrinsic(Intrinsic::NatEq, Rc::new([lhs.clone(), rhs.clone()])),
            Ty::Int => Expr::Intrinsic(Intrinsic::IntEq, Rc::new([lhs.clone(), rhs.clone()])),
            Ty::Real => Expr::Intrinsic(Intrinsic::RealEq, Rc::new([lhs.clone(), rhs.clone()])),
            Ty::Bool => todo!(),
            Ty::Tuple(items) => todo!(),
            Ty::Record(items) => todo!(),
            Ty::Union(items) => todo!(),
            Ty::Action(ty) => todo!(),
            Ty::Lambda(ty, ty1) => todo!(),
            Ty::Boxed(_) => todo!(),
        };

        stack.pop();
        expr
    }

    fn dynamic<'a>(&mut self, ty: &'a Ty, input: &Expr, stack: &mut Vec<&'a Ty>) -> Expr {
        // action   0
        // int      1
        // lambda   2
        // nat      3
        // real     4
        // record   5
        // str      6
        // tuple    7
        // variant  8

        stack.push(ty);
        let input = Rc::new(input.clone());

        let expr = match ty {
            Ty::Str => Expr::Construct(Constructor::Variant(6, input)),
            Ty::Nat => Expr::Construct(Constructor::Variant(3, input)),
            Ty::Int => Expr::Construct(Constructor::Variant(1, input)),
            Ty::Real => Expr::Construct(Constructor::Variant(4, input)),

            Ty::Bool => todo!(),

            Ty::Tuple(fields) => {
                let none = Expr::Construct(Constructor::Variant(0, Rc::new(Expr::unit())));

                let fields = fields
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(i, ty)| {
                        let input = Expr::Field(input.clone(), i);
                        self.dynamic(ty, &input, stack)
                    })
                    .fold(none, |expr, field| {
                        let tuple = Expr::Construct(Constructor::Tuple(Rc::new([field, expr])));
                        Expr::Construct(Constructor::Variant(1, Rc::new(tuple)))
                    });

                Expr::Construct(Constructor::Variant(7, Rc::new(fields)))
            }

            Ty::Record(fields) => {
                let none = Expr::Construct(Constructor::Variant(0, Rc::new(Expr::unit())));

                let fields = fields
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(i, (name, ty))| {
                        Expr::Construct(Constructor::Tuple(Rc::new([
                            Expr::Constant(Constant::Str(Cow::Borrowed(name))),
                            self.dynamic(ty, &Expr::Field(input.clone(), i), stack),
                        ])))
                    })
                    .fold(none, |expr, field| {
                        let tuple = Expr::Construct(Constructor::Tuple(Rc::new([field, expr])));
                        Expr::Construct(Constructor::Variant(1, Rc::new(tuple)))
                    });

                Expr::Construct(Constructor::Variant(5, Rc::new(fields)))
            }

            Ty::Union(variants) => {
                let mut variants = variants
                    .iter()
                    .map(|(name, ty)| {
                        let name = Expr::Constant(Constant::Str(Cow::Borrowed(name)));
                        let input = Expr::Payload(input.clone());
                        let payload = self.dynamic(ty, &input, stack);

                        let tuple = Rc::new(Expr::Construct(Constructor::Tuple(Rc::new([
                            name, payload,
                        ]))));

                        Expr::Construct(Constructor::Variant(8, tuple))
                    })
                    .enumerate()
                    .collect::<Vec<_>>()
                    .into_iter();

                let (_, mut output) = variants.next().unwrap();

                for (i, expr) in variants {
                    output = Expr::If(
                        Rc::new(Expr::Is(input.clone(), i)),
                        Rc::new(expr),
                        Rc::new(output),
                    );
                }

                output
            }

            Ty::Action(..) => Expr::Construct(Constructor::Variant(0, Rc::new(Expr::unit()))),
            Ty::Lambda(..) => Expr::Construct(Constructor::Variant(2, Rc::new(Expr::unit()))),

            Ty::Boxed(index) => {
                let ty = stack[stack.len() - index - 1];

                if let Some(global) = self.dynamics.get(ty) {
                    let global = Rc::new(Expr::Global(global.clone()));
                    Expr::Call(global, input)
                } else {
                    let global = Global {
                        index: self.globals.len(),
                        ty: Ty::Lambda(Rc::new(ty.clone()), Rc::new(Ty::Str)),
                    };

                    self.globals.push(Expr::unit());
                    self.dynamics.insert(ty.clone(), global.clone());

                    let local = Local {
                        index: 0,
                        ty: ty.clone(),
                    };

                    let mut stack = stack.clone();
                    stack.truncate(stack.len() - index - 1);

                    let dynamic = self.dynamic(ty, &Expr::Local(local.clone()), &mut stack);
                    let lambda = Expr::Lambda(Lambda {
                        captures: Vec::new(),
                        input: local,
                        output: Rc::new(dynamic),
                    });

                    self.globals[global.index] = lambda;

                    let global = Rc::new(Expr::Global(global));
                    Expr::Call(global, input)
                }
            }
        };

        stack.pop();
        expr
    }

    fn add_local(&mut self, ty: Ty) -> Local {
        let index = *self.locals;
        *self.locals += 1;

        Local { index, ty }
    }

    fn variant_index(&mut self, ty: &ir::Ty, name: &str) -> usize {
        let Ty::Union(variants) = self.ty(ty) else {
            panic!();
        };

        variants.iter().position(|(n, _)| *n == name).unwrap()
    }

    fn field_index(&mut self, ty: &ir::Ty, name: &str) -> usize {
        let Ty::Record(fields) = self.ty(ty) else {
            panic!();
        };

        fields.iter().position(|(n, _)| *n == name).unwrap()
    }

    fn ty(&mut self, ty: &ir::Ty) -> Ty {
        fn recurse(
            lowerer: &mut ScopedLowerer<'_, '_>,
            ty: &ir::Ty,
            depth: usize,
            stack: &mut Vec<(ir::Ty, usize)>,
            generics: &mut Vec<(ir::GenericTy, ir::Ty)>,
        ) -> Ty {
            if let Some((_, outer_depth)) = stack.iter().rfind(|(x, _)| x == ty) {
                return Ty::Boxed(depth - outer_depth);
            } else if let ir::Ty::Generic(generic) = ty
                && let Some((_, ty)) = generics.iter().rev().find(|(g, _)| g == generic).cloned()
            {
                return recurse(lowerer, &ty, depth, stack, generics);
            } else if let ir::Ty::Infer(bound) | ir::Ty::Generic(ir::GenericTy { bound, .. }) = ty
                && let Some(subst) = lowerer.subst.get(bound)
            {
                return subst.clone();
            } else if let ir::Ty::Infer(bound) = ty
                && let Some(subst) = lowerer.program.subst.get(bound)
            {
                return recurse(lowerer, subst, depth, stack, generics);
            }

            stack.push((ty.clone(), depth));

            let ty = match ty {
                ir::Ty::Str => Ty::Str,

                ir::Ty::Numeric(n) => numeric(*n),

                ir::Ty::Tuple(fields) => {
                    let fields = fields
                        .iter()
                        .map(|ty| recurse(lowerer, ty, depth + 1, stack, generics))
                        .collect();

                    Ty::Tuple(fields)
                }

                ir::Ty::Lambda(ty) => {
                    let input = recurse(lowerer, &ty.input, depth + 1, stack, generics);
                    let output = recurse(lowerer, &ty.output, depth + 1, stack, generics);

                    let input = Rc::new(input);
                    let output = Rc::new(output);

                    Ty::Lambda(input, output)
                }

                ir::Ty::Monad(ty) => {
                    let output = recurse(lowerer, ty, depth + 1, stack, generics);
                    let output = Rc::new(output);

                    Ty::Action(output)
                }

                ir::Ty::Alias(ty) => {
                    let alias = &lowerer.program.aliases[ty.alias];

                    let len = generics.len();
                    generics.extend(alias.params.iter().copied().zip(ty.args.iter().cloned()));

                    let ty = recurse(lowerer, &alias.ty, depth, stack, generics);
                    generics.truncate(len);

                    ty
                }

                ir::Ty::Record(ty) => record(lowerer, ty, depth, stack, generics),

                ir::Ty::Union(ty) => union(lowerer, ty, depth, stack, generics),

                ir::Ty::Infer(bound) | ir::Ty::Generic(ir::GenericTy { bound, .. }) => {
                    match lowerer.program.bounds[*bound] {
                        ir::Bound::Numeric(n) => numeric(n),
                        ir::Bound::Record(ref ty) => record(lowerer, ty, depth, stack, generics),
                        ir::Bound::Union(ref ty) => union(lowerer, ty, depth, stack, generics),
                        ir::Bound::None => Ty::unit(),
                    }
                }

                ir::Ty::Error => unreachable!("{ty:?}"),
            };

            stack.pop();

            ty
        }

        fn record(
            lowerer: &mut ScopedLowerer<'_, '_>,
            ty: &ir::RecordTy,
            depth: usize,
            stack: &mut Vec<(ir::Ty, usize)>,
            generics: &mut Vec<(ir::GenericTy, ir::Ty)>,
        ) -> Ty {
            let mut fields = ty.fields.iter().collect::<Vec<_>>();
            fields.sort_by_key(|field| field.name);

            let fields = fields
                .into_iter()
                .map(|field| {
                    let ty = recurse(lowerer, &field.ty, depth + 1, stack, generics);

                    (field.name, ty)
                })
                .collect();

            Ty::Record(fields)
        }

        fn union(
            lowerer: &mut ScopedLowerer<'_, '_>,
            ty: &ir::UnionTy,
            depth: usize,
            stack: &mut Vec<(ir::Ty, usize)>,
            generics: &mut Vec<(ir::GenericTy, ir::Ty)>,
        ) -> Ty {
            let mut variants = ty.variants.iter().collect::<Vec<_>>();
            variants.sort_by_key(|variant| variant.name);

            if variants.len() == 2
                && variants[0].name == "false"
                && variants[0].payload.is_none()
                && variants[1].name == "true"
                && variants[1].payload.is_none()
            {
                return Ty::Bool;
            }

            let variants = variants
                .into_iter()
                .map(|variant| {
                    let payload = match variant.payload {
                        Some(ref payload) => recurse(lowerer, payload, depth + 1, stack, generics),
                        None => Ty::unit(),
                    };

                    (variant.name, payload)
                })
                .collect();

            Ty::Union(variants)
        }

        fn numeric(numeric: ir::Numeric) -> Ty {
            match numeric {
                ir::Numeric::Nat => Ty::Nat,
                ir::Numeric::Int => Ty::Int,
                ir::Numeric::Real => Ty::Real,
            }
        }

        recurse(self, ty, 0, &mut Vec::new(), &mut Vec::new())
    }
}

impl<'b> Deref for ScopedLowerer<'_, 'b> {
    type Target = Lowerer<'b>;

    fn deref(&self) -> &Self::Target {
        self.lowerer
    }
}

impl DerefMut for ScopedLowerer<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lowerer
    }
}
