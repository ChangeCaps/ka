use std::{
    borrow::Cow,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    arena::{Arena, Id},
    ir,
    mir::{Constant, Expr, Program, Ty},
};

pub fn lower(program: &ir::Program, main: Id<ir::Var>) -> Program {
    let mut lowerer = Lowerer::new(program);

    let var = &lowerer.program.vars[main];
    let ir::VarKind::Global(global) = var.kind else {
        unreachable!();
    };

    let ty = Ty::action(Ty::unit());
    let expr = lowerer.global(global, ty);

    todo!()
}

struct Lowerer<'a> {
    program: &'a ir::Program,

    exprs: Arena<Expr>,

    globals: HashMap<(Id<ir::Global>, Ty), Id<Expr>>,
    tys: HashMap<ir::Ty, Ty>,
}

impl<'a> Lowerer<'a> {
    fn new(program: &'a ir::Program) -> Self {
        Self {
            program,

            exprs: Arena::new(),

            globals: HashMap::new(),
            tys: HashMap::new(),
        }
    }

    fn global(&mut self, global: Id<ir::Global>, ty: Ty) -> Id<Expr> {
        let key = (global, ty);

        if let Some(expr) = self.globals.get(&key) {
            return *expr;
        }

        let (global, ty) = key;

        let global = &self.program.globals[global];
        let mut subst = HashMap::new();
        self.unify_ty(&global.pat.ty(), &ty, &mut subst);

        let mut scoped = ScopedLowerer {
            lowerer: self,
            subst: &subst,
        };

        let expr = scoped.expr(&global.expr);

        todo!()
    }

    fn unify_ty(&self, actual: &ir::Ty, expected: &Ty, subst: &mut HashMap<ir::Ty, Ty>) {
        if subst.contains_key(actual) {
            return;
        }

        match (actual, expected) {
            (ir::Ty::Infer(bound) | ir::Ty::Generic(ir::GenericTy { bound, .. }), _) => {
                if let Some(actual) = self.program.subst.get(bound) {
                    self.unify_ty(actual, expected, subst);
                } else {
                    subst.insert(actual.clone(), expected.clone());
                }
            }

            (ir::Ty::Str, Ty::Str)
            | (ir::Ty::Numeric(ir::Numeric::Nat), Ty::Nat)
            | (ir::Ty::Numeric(ir::Numeric::Int), Ty::Int)
            | (ir::Ty::Numeric(ir::Numeric::Real), Ty::Real) => {}

            (ir::Ty::Monad(actual), Ty::Action(expected)) => self.unify_ty(actual, expected, subst),

            (ir::Ty::Record(actual), Ty::Tuple(expected)) => {
                let mut fields = actual.fields.clone();
                fields.sort_by_key(|field| field.name);

                for (actual, expected) in fields.iter().zip(expected.as_ref()) {
                    self.unify_ty(&actual.ty, expected, subst);
                }
            }

            (_, _) => unreachable!("{:?}, {:?}", actual, expected),
        }
    }
}

struct ScopedLowerer<'a, 'b> {
    lowerer: &'a mut Lowerer<'b>,
    subst: &'a HashMap<ir::Ty, Ty>,
}

impl ScopedLowerer<'_, '_> {
    fn expr(&mut self, expr: &ir::Expr) -> Id<Expr> {
        let expr = match expr {
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

            ir::Expr::Var(expr) => todo!(),
            ir::Expr::Let(expr) => todo!(),
            ir::Expr::Bind(expr) => todo!(),
            ir::Expr::Pure(expr) => todo!(),
            ir::Expr::Call(expr) => todo!(),
            ir::Expr::With(expr) => todo!(),
            ir::Expr::Field(expr) => todo!(),
            ir::Expr::Lambda(expr) => todo!(),
            ir::Expr::Variant(expr) => todo!(),
            ir::Expr::Record(expr) => todo!(),
            ir::Expr::Binary(expr) => todo!(),
            ir::Expr::Tuple(expr) => todo!(),
            ir::Expr::Match(expr) => todo!(),
            ir::Expr::Intrinsic(expr) => todo!(),
            ir::Expr::Error(..) => unreachable!(),
        };

        self.exprs.add(expr)
    }

    fn ty(&mut self, ty: &ir::Ty) -> Ty {
        if let Some(subst) = self.subst.get(ty) {
            return subst.clone();
        } else if let Some(subst) = self.tys.get(ty) {
            return subst.clone();
        }

        let mir_ty = match ty {
            ir::Ty::Str => Ty::Str,

            ir::Ty::Numeric(numeric) => self.numeric(*numeric),

            ir::Ty::Tuple(fields) => todo!(),

            ir::Ty::Lambda(ty) => todo!(),

            ir::Ty::Monad(ty) => todo!(),

            ir::Ty::Alias(ty) => todo!(),

            ir::Ty::Record(ty) => self.record(ty),

            ir::Ty::Union(ty) => self.union(ty),

            ir::Ty::Infer(bound) | ir::Ty::Generic(ir::GenericTy { bound, .. }) => {
                match self.program.bounds[*bound] {
                    ir::Bound::Numeric(numeric) => self.numeric(numeric),
                    ir::Bound::Record(ref ty) => self.record(ty),
                    ir::Bound::Union(ref ty) => self.union(ty),
                    ir::Bound::None => Ty::unit(),
                }
            }

            ir::Ty::Error => unreachable!(),
        };

        self.tys.insert(ty.clone(), mir_ty.clone());

        mir_ty
    }

    fn numeric(&mut self, numeric: ir::Numeric) -> Ty {
        match numeric {
            ir::Numeric::Nat => Ty::Nat,
            ir::Numeric::Int => Ty::Int,
            ir::Numeric::Real => Ty::Real,
        }
    }

    fn record(&mut self, record: &ir::RecordTy) -> Ty {
        todo!()
    }

    fn union(&mut self, union: &ir::UnionTy) -> Ty {
        todo!()
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
