use std::collections::{HashMap, hash_map::Entry};

use crate::{
    arena::Id,
    ast,
    diagnostic::{Diagnostic, Span},
    ir,
    lower::{Generics, Lowerer},
};

impl Lowerer<'_> {
    pub(super) fn expr(&mut self, scope: Id<ir::Scope>, expr: &ast::Expr) -> ir::Expr {
        match expr {
            ast::Expr::Paren(expr) => self.paren_expr(scope, expr),
            ast::Expr::Num(expr) => self.num_expr(scope, expr),
            ast::Expr::String(expr) => self.string_expr(scope, expr),
            ast::Expr::Named(expr) => self.named_expr(scope, expr),
            ast::Expr::Call(expr) => self.call_expr(scope, expr),
            ast::Expr::Lambda(expr) => self.lambda_expr(scope, expr),
            ast::Expr::Variant(expr) => self.variant_expr(scope, expr),
            ast::Expr::Record(expr) => self.record_expr(scope, expr),
            ast::Expr::Binary(expr) => self.binary_expr(scope, expr),
            ast::Expr::Tuple(expr) => self.tuple_expr(scope, expr),
            ast::Expr::Block(expr) => self.block_expr(scope, expr),
            ast::Expr::Do(expr) => self.do_expr(scope, expr),
            ast::Expr::Match(expr) => self.match_expr(scope, expr),
            ast::Expr::Error(span) => self.error_expr(*span),
        }
    }

    fn paren_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::ParenExpr) -> ir::Expr {
        self.expr(scope, &expr.expr)
    }

    fn num_expr(&mut self, _scope: Id<ir::Scope>, expr: &ast::NumExpr) -> ir::Expr {
        ir::Expr::Value(ir::ValueExpr {
            value: ir::Value::Num(expr.number),
            ty: ir::Ty::Num,
        })
    }

    fn string_expr(&mut self, _scope: Id<ir::Scope>, expr: &ast::StringExpr) -> ir::Expr {
        ir::Expr::Value(ir::ValueExpr {
            value: ir::Value::String(expr.string.into()),
            ty: ir::Ty::Str,
        })
    }

    fn named_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::NamedExpr) -> ir::Expr {
        let Some(var) = self.resolve_var(scope, expr.import, expr.name) else {
            return self.variable_undefined(expr.span, expr.name);
        };

        match self.vars[var].kind {
            ir::VarKind::Const(target) => {
                let ty = self.add_inferred_type();
                let current = self.current_const(scope);

                self.dependencies
                    .entry(current)
                    .or_default()
                    .entry(target)
                    .or_default()
                    .push((ty.clone(), expr.span));

                ir::Expr::Var(ir::VarExpr { var, ty })
            }

            ir::VarKind::Extern(..) | ir::VarKind::Local => {
                let ty = self.vars[var].ty.clone();
                ir::Expr::Var(ir::VarExpr { var, ty })
            }
        }
    }

    fn variable_undefined(&mut self, span: Span, name: &str) -> ir::Expr {
        let diagnostic = Diagnostic::error(format!("variable `{}` not defined", name))
            .with_label(span, "found here");

        self.emitter.emit(diagnostic);

        let ty = self.add_inferred_type();
        ir::Expr::Error(ty)
    }

    fn call_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::CallExpr) -> ir::Expr {
        let lambda = self.expr(scope, &expr.lambda);
        let input = self.expr(scope, &expr.input);

        let lambda = Box::new(lambda);
        let input = Box::new(input);

        let ty = self.add_inferred_type();
        let lambda_ty = ir::Ty::lambda(input.ty(), ty.clone());

        self.unify(&lambda.ty(), &lambda_ty, expr.span);

        ir::Expr::Call(ir::CallExpr { lambda, input, ty })
    }

    fn lambda_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::LambdaExpr) -> ir::Expr {
        self.lambda_def(scope, &expr.params, &expr.expr)
    }

    fn variant_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::VariantExpr) -> ir::Expr {
        let Some(name) = expr.name else {
            let ty = self.add_inferred_type();
            return ir::Expr::Error(ty);
        };

        let span = expr.span;
        let expr = expr
            .expr
            .as_ref()
            .map(|expr| self.expr(scope, expr))
            .map(Box::new);

        let ty = self.add_inferred_type();
        let payload = expr.as_deref().map(ir::Expr::ty);

        self.constrain_tag(&ty, name, payload.as_ref(), span);

        ir::Expr::Variant(ir::VariantExpr { name, expr, ty })
    }

    fn record_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::RecordExpr) -> ir::Expr {
        let mut fields: Vec<ir::ExprField> = Vec::new();

        for field in &expr.fields {
            let Some(name) = field.name else {
                continue;
            };

            let expr = self.expr(scope, &field.expr);

            if fields.iter().any(|f| f.name == name) {
                let diagnostic = Diagnostic::error(format!("field `{}` already defined", name))
                    .with_label(field.span, "here");

                self.emitter.emit(diagnostic);
                continue;
            }

            fields.push(ir::ExprField { name, expr });
        }

        let ty = ir::Ty::Record(ir::RecordTy {
            fields: fields
                .iter()
                .map(|field| ir::TyField {
                    name: field.name,
                    ty: field.expr.ty(),
                })
                .collect(),
        });

        ir::Expr::Record(ir::RecordExpr { fields, ty })
    }

    fn binary_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::BinaryExpr) -> ir::Expr {
        let lhs = self.expr(scope, &expr.lhs);
        let rhs = self.expr(scope, &expr.rhs);

        let lhs = Box::new(lhs);
        let rhs = Box::new(rhs);

        let ty = match expr.op {
            ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul | ast::BinOp::Div => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);
                self.unify(&lhs.ty(), &ir::Ty::Num, expr.span);

                ir::Ty::Num
            }

            ast::BinOp::Gt | ast::BinOp::Lt | ast::BinOp::GtEq | ast::BinOp::LtEq => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);
                self.unify(&lhs.ty(), &ir::Ty::Num, expr.span);

                ir::Ty::bool()
            }

            ast::BinOp::Eq | ast::BinOp::Ne => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);

                ir::Ty::bool()
            }
        };

        let op = match expr.op {
            ast::BinOp::Add => ir::BinOp::Add,
            ast::BinOp::Sub => ir::BinOp::Sub,
            ast::BinOp::Mul => ir::BinOp::Mul,
            ast::BinOp::Div => ir::BinOp::Div,
            ast::BinOp::Gt => ir::BinOp::Gt,
            ast::BinOp::Lt => ir::BinOp::Lt,
            ast::BinOp::GtEq => ir::BinOp::GtEq,
            ast::BinOp::LtEq => ir::BinOp::LtEq,
            ast::BinOp::Eq => ir::BinOp::Eq,
            ast::BinOp::Ne => ir::BinOp::Ne,
        };

        ir::Expr::Binary(ir::BinaryExpr { op, lhs, rhs, ty })
    }

    fn tuple_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::TupleExpr) -> ir::Expr {
        let fields = expr
            .fields
            .iter()
            .map(|field| self.expr(scope, field))
            .collect::<Vec<_>>();

        let tys = fields.iter().map(ir::Expr::ty).collect();
        let ty = ir::Ty::Tuple(tys);

        ir::Expr::Tuple(ir::TupleExpr { fields, ty })
    }

    fn block_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::BlockExpr) -> ir::Expr {
        let scope = self.add_scope(ir::ScopeKind::Block, scope);

        self.import_defs(scope, &expr.defs);
        self.alias_defs(scope, &expr.defs);
        self.extern_defs(scope, &expr.defs);

        let mut exprs = Vec::new();

        for def in &expr.defs {
            if let ast::Def::Let(def) = def {
                let (pat, val) = self.let_def(scope, ir::VarKind::Local, def);
                exprs.push((pat, val));
            }
        }

        let expr = self.expr(scope, &expr.expr);

        exprs.into_iter().rfold(expr, |expr, (pat, val)| {
            ir::Expr::Let(ir::LetExpr {
                pat,
                input: Box::new(val),
                expr: Box::new(expr),
            })
        })
    }

    fn do_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::DoExpr) -> ir::Expr {
        match expr.kind {
            ast::DoKind::Block(ref stmts) => self.do_expr_block(scope, stmts),

            ast::DoKind::Expr(ref expr) => {
                let expr = self.expr(scope, expr);
                let expr = Box::new(expr);

                let ty = Box::new(expr.ty());
                let ty = ir::Ty::Monad(ty);

                ir::Expr::Pure(ir::PureExpr { expr, ty })
            }
        }
    }

    fn do_expr_block(&mut self, scope: Id<ir::Scope>, stmts: &[ast::DoStmt]) -> ir::Expr {
        enum LetOrBind {
            Let(ir::Pat, ir::Expr),
            Bind(ir::Pat, ir::Expr),
        }

        let scope = self.add_scope(ir::ScopeKind::Block, scope);

        let defs = stmts.iter().filter_map(|stmt| match stmt {
            ast::DoStmt::Def(def) => Some(def),
            ast::DoStmt::Expr(..) => None,
        });

        self.import_defs(scope, defs.clone());
        self.alias_defs(scope, defs.clone());
        self.extern_defs(scope, defs);

        let mut exprs = Vec::new();

        let mut output = ir::Expr::Pure(ir::PureExpr {
            expr: Box::new(ir::Expr::unit()),
            ty: ir::Ty::Monad(Box::new(ir::Ty::unit())),
        });

        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;

            match stmt {
                ast::DoStmt::Def(def) => {
                    if let ast::Def::Let(def) = def {
                        let (pat, expr) = self.do_let_def(scope, def);

                        let expr = match def.is_bind {
                            true => LetOrBind::Bind(pat, expr),
                            false => LetOrBind::Let(pat, expr),
                        };

                        exprs.push(expr);
                    }
                }

                ast::DoStmt::Expr(expr) => {
                    let span = expr.span();

                    let expr = self.expr(scope, expr);
                    let ty = expr.ty();

                    let monad_ty = ir::Ty::Monad(Box::new(self.add_inferred_type()));
                    self.unify(&monad_ty, &ty, span);

                    if is_last {
                        output = expr;
                    } else {
                        let pat = ir::Pat::Wild(ir::WildPat { ty, span });
                        exprs.push(LetOrBind::Bind(pat, expr))
                    }
                }
            }
        }

        exprs.into_iter().rfold(output, |expr, kind| match kind {
            LetOrBind::Let(pat, input) => ir::Expr::Let(ir::LetExpr {
                pat,
                input: Box::new(input),
                expr: Box::new(expr),
            }),

            LetOrBind::Bind(pat, input) => ir::Expr::Bind(ir::BindExpr {
                pat,
                input: Box::new(input),
                expr: Box::new(expr),
            }),
        })
    }

    fn do_let_def(&mut self, scope: Id<ir::Scope>, def: &ast::LetDef) -> (ir::Pat, ir::Expr) {
        if !def.is_bind {
            return self.let_def(scope, ir::VarKind::Local, def);
        }

        let expr = self.complete_let_def(scope, def);
        let pat = self.pat(scope, ir::VarKind::Local, &def.pat);

        let ty = self.add_inferred_type();
        let monad_ty = ir::Ty::Monad(Box::new(ty.clone()));

        self.unify(&monad_ty, &expr.ty(), def.span);
        self.unify(&pat.ty(), &ty, def.span);

        if let Some(ref ty) = def.ty {
            let ty = self.ty(scope, &mut Generics::dynamic(), ty);
            self.unify(&pat.ty(), &ty, def.span);
        }

        (pat, expr)
    }

    fn match_expr(&mut self, scope: Id<ir::Scope>, expr: &ast::MatchExpr) -> ir::Expr {
        assert!(!expr.arms.is_empty());

        let ty = self.add_inferred_type();

        let arms = expr
            .arms
            .iter()
            .map(|arm| {
                let scope = self.add_scope(ir::ScopeKind::Block, scope);
                let pat = self.pat(scope, ir::VarKind::Local, &arm.pat);
                let expr = self.expr(scope, &arm.expr);

                // ensure that the type of all arm expressions are the same
                self.unify(&ty, &expr.ty(), arm.expr.span());

                ir::Arm { pat, expr }
            })
            .collect::<Vec<_>>();

        let pats = arms.iter().map(|arm| &arm.pat).collect::<Vec<_>>();
        let pats_ty = self.pats_ty(&pats);

        let expr = {
            let target = self.expr(scope, &expr.expr);
            self.unify(&target.ty(), &pats_ty, expr.span);

            Box::new(target)
        };

        ir::Expr::Match(ir::MatchExpr { expr, arms, ty })
    }

    fn pats_ty(&mut self, pats: &[&ir::Pat]) -> ir::Ty {
        let tame = pats
            .iter()
            .copied()
            .filter(|pat| !pat.is_wild())
            .collect::<Vec<_>>();

        let ty = match tame.first() {
            Some(first) => match first {
                ir::Pat::Tuple(first) => self.tuple_pats_ty(first, &tame),
                ir::Pat::Tag(..) => self.tag_pats_ty(&tame),

                ir::Pat::Wild(..) | ir::Pat::Bind(..) | ir::Pat::Error(..) => unreachable!(),
            },

            _ => self.add_inferred_type(),
        };

        for pat in pats.iter().copied().filter(|pat| pat.is_wild()) {
            self.unify(&ty, &pat.ty(), pat.span());
        }

        ty
    }

    fn tuple_pats_ty(&mut self, first: &ir::TuplePat, pats: &[&ir::Pat]) -> ir::Ty {
        let mut fields = vec![Vec::new(); first.fields.len()];

        for pat in pats {
            let ir::Pat::Tuple(pat) = pat else {
                let diagnostic = Diagnostic::error("invalid pattern, expected tuple")
                    .with_label(pat.span(), "found here");

                self.emitter.emit(diagnostic);

                continue;
            };

            if pat.fields.len() != fields.len() {
                let diagnostic = Diagnostic::error(format!(
                    "invalid pattern, expected `{}-tuple`, but found `{}-tuple`",
                    fields.len(),
                    pat.fields.len(),
                ))
                .with_label(pat.span, "here");

                self.emitter.emit(diagnostic);

                continue;
            }

            for (pats, field) in fields.iter_mut().zip(&pat.fields) {
                pats.push(field);
            }
        }

        let fields = fields
            .into_iter()
            .map(|pats| self.pats_ty(&pats))
            .collect::<Vec<_>>();

        ir::Ty::Tuple(fields)
    }

    fn tag_pats_ty(&mut self, pats: &[&ir::Pat]) -> ir::Ty {
        let mut variants: HashMap<&str, Option<Vec<&ir::Pat>>> = HashMap::new();

        for pat in pats {
            let ir::Pat::Tag(pat) = pat else {
                let diagnostic = Diagnostic::error("invalid pattern, expected tag")
                    .with_label(pat.span(), "found here");

                self.emitter.emit(diagnostic);

                continue;
            };

            match variants.entry(pat.name) {
                Entry::Occupied(mut entry) => match (entry.get_mut(), &pat.pat) {
                    (Some(pats), Some(pat)) => pats.push(pat),
                    (None, None) => {}
                    (_, _) => {}
                },

                Entry::Vacant(entry) => {
                    let pats = pat.pat.as_ref().map(|pat| vec![pat.as_ref()]);
                    entry.insert(pats);
                }
            }
        }

        let variants = variants
            .into_iter()
            .map(|(name, pats)| {
                let ty = pats.map(|pats| self.pats_ty(&pats));
                ir::Variant { name, ty }
            })
            .collect::<Vec<_>>();

        ir::Ty::Union(ir::UnionTy { variants })
    }

    fn error_expr(&mut self, _span: Span) -> ir::Expr {
        ir::Expr::Error(self.add_inferred_type())
    }
}
