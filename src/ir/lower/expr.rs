use crate::{
    arena::Id,
    ast,
    diagnostic::{Diagnostic, Span},
    ir::{
        Arm, BinOp, BinaryExpr, BindExpr, CallExpr, Expr, ExprField, FieldExpr, LetExpr, MatchExpr,
        Numeric, Pat, PureExpr, RecordExpr, RecordTy, Scope, ScopeKind, TupleExpr, Ty, TyField,
        UnOp, UnaryExpr, Value, ValueExpr, VarExpr, VarKind, VariantExpr, Visibility, WildPat,
        WithExpr,
        lower::{Generics, Lowerer, exhaust},
    },
};

impl Lowerer<'_> {
    pub(super) fn expr(&mut self, scope: Id<Scope>, expr: &ast::Expr) -> Expr {
        match expr {
            ast::Expr::Paren(expr) => self.paren_expr(scope, expr),
            ast::Expr::Num(expr) => self.num_expr(scope, expr),
            ast::Expr::Str(expr) => self.str_expr(scope, expr),
            ast::Expr::Named(expr) => self.named_expr(scope, expr),
            ast::Expr::Field(expr) => self.field_expr(scope, expr),
            ast::Expr::With(expr) => self.with_expr(scope, expr),
            ast::Expr::Call(expr) => self.call_expr(scope, expr),
            ast::Expr::Lambda(expr) => self.lambda_expr(scope, expr),
            ast::Expr::Variant(expr) => self.variant_expr(scope, expr),
            ast::Expr::Record(expr) => self.record_expr(scope, expr),
            ast::Expr::Unary(expr) => self.unary_expr(scope, expr),
            ast::Expr::Binary(expr) => self.binary_expr(scope, expr),
            ast::Expr::Tuple(expr) => self.tuple_expr(scope, expr),
            ast::Expr::Block(expr) => self.block_expr(scope, expr),
            ast::Expr::Do(expr) => self.do_expr(scope, expr),
            ast::Expr::Match(expr) => self.match_expr(scope, expr),
            ast::Expr::Error(span) => self.error_expr(*span),
        }
    }

    fn paren_expr(&mut self, scope: Id<Scope>, expr: &ast::ParenExpr) -> Expr {
        self.expr(scope, &expr.expr)
    }

    fn num_expr(&mut self, _scope: Id<Scope>, expr: &ast::NumExpr) -> Expr {
        let value = Value::Num(expr.number);

        let ty = self.add_inferred_type();
        self.constrain_numeric(&ty, Numeric::Real, expr.span);

        Expr::Value(ValueExpr { value, ty })
    }

    fn str_expr(&mut self, _scope: Id<Scope>, expr: &ast::StrExpr) -> Expr {
        Expr::Value(ValueExpr {
            value: Value::Str(expr.string.into()),
            ty: Ty::Str,
        })
    }

    fn named_expr(&mut self, scope: Id<Scope>, expr: &ast::NamedExpr) -> Expr {
        let Some(var) = self.resolve_var(scope, expr.import, expr.name) else {
            return self.variable_undefined(expr.span, expr.name);
        };

        match self.vars[var].kind {
            VarKind::Global(global) => {
                let ty = self.add_inferred_type();
                let current = self.current_global(scope);

                self.dependencies
                    .entry(current)
                    .or_default()
                    .entry(global)
                    .or_default()
                    .push((ty.clone(), expr.span));

                Expr::Var(VarExpr { var, ty })
            }

            VarKind::Extern(..) | VarKind::Local => {
                let ty = self.vars[var].ty.clone();
                Expr::Var(VarExpr { var, ty })
            }
        }
    }

    fn variable_undefined(&mut self, span: Span, name: &str) -> Expr {
        let diagnostic = Diagnostic::error(format!("variable `{}` not defined", name))
            .with_label(span, "found here");

        self.emitter.emit(diagnostic);
        self.error_expr(span)
    }

    fn field_expr(&mut self, scope: Id<Scope>, expr: &ast::FieldExpr) -> Expr {
        let input = self.expr(scope, &expr.input);
        let input = Box::new(input);

        let Some(name) = expr.name else {
            return self.error_expr(expr.span);
        };

        let ty = self.add_inferred_type();
        self.constrain_field(&input.ty(), name, &ty, expr.span);

        Expr::Field(FieldExpr { input, name, ty })
    }

    fn with_expr(&mut self, scope: Id<Scope>, expr: &ast::WithExpr) -> Expr {
        let input = self.expr(scope, &expr.input);
        let input = Box::new(input);

        let ty = input.ty();

        let mut fields: Vec<ExprField> = Vec::new();

        for field in &expr.fields {
            let Some(name) = field.name else {
                continue;
            };

            if fields.iter().any(|f| f.name == name) {
                let diagnostic = Diagnostic::error(format!("field `{}` already defined", name))
                    .with_label(field.span, "here");

                self.emitter.emit(diagnostic);
                continue;
            }

            let expr = self.expr(scope, &field.expr);
            self.constrain_field(&ty, name, &expr.ty(), field.span);

            fields.push(ExprField { name, expr });
        }

        Expr::With(WithExpr { input, fields, ty })
    }

    fn call_expr(&mut self, scope: Id<Scope>, expr: &ast::CallExpr) -> Expr {
        let lambda = self.expr(scope, &expr.lambda);
        let input = self.expr(scope, &expr.input);

        let lambda = Box::new(lambda);
        let input = Box::new(input);

        let ty = self.add_inferred_type();
        let lambda_ty = Ty::lambda(input.ty(), ty.clone());

        self.unify(&lambda.ty(), &lambda_ty, expr.span);

        Expr::Call(CallExpr { lambda, input, ty })
    }

    fn lambda_expr(&mut self, scope: Id<Scope>, expr: &ast::LambdaExpr) -> Expr {
        self.lambda(scope, &expr.params, &expr.expr)
    }

    fn variant_expr(&mut self, scope: Id<Scope>, expr: &ast::VariantExpr) -> Expr {
        let Some(name) = expr.name else {
            return self.error_expr(expr.span);
        };

        let span = expr.span;
        let expr = expr
            .expr
            .as_ref()
            .map(|expr| self.expr(scope, expr))
            .map(Box::new);

        let ty = self.add_inferred_type();
        let payload = expr.as_deref().map(Expr::ty);

        self.constrain_variant(&ty, name, payload.as_ref(), span);

        Expr::Variant(VariantExpr {
            name,
            payload: expr,
            ty,
        })
    }

    fn record_expr(&mut self, scope: Id<Scope>, expr: &ast::RecordExpr) -> Expr {
        let mut fields: Vec<ExprField> = Vec::new();

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

            fields.push(ExprField { name, expr });
        }

        let ty = Ty::Record(RecordTy {
            fields: fields
                .iter()
                .map(|field| TyField {
                    name: field.name,
                    ty: field.expr.ty(),
                })
                .collect(),
        });

        Expr::Record(RecordExpr { fields, ty })
    }

    fn unary_expr(&mut self, scope: Id<Scope>, expr: &ast::UnaryExpr) -> Expr {
        let input = self.expr(scope, &expr.input);
        let input = Box::new(input);

        let ty = match expr.op {
            ast::UnOp::Nat => {
                self.constrain_numeric(&input.ty(), Numeric::Real, expr.span);
                Ty::NAT
            }

            ast::UnOp::Int => {
                self.constrain_numeric(&input.ty(), Numeric::Real, expr.span);
                Ty::INT
            }

            ast::UnOp::Real => {
                self.constrain_numeric(&input.ty(), Numeric::Real, expr.span);
                Ty::REAL
            }

            ast::UnOp::Not => {
                self.unify(&input.ty(), &Ty::bool(), expr.span);
                Ty::bool()
            }
        };

        let op = match expr.op {
            ast::UnOp::Nat => UnOp::Nat,
            ast::UnOp::Int => UnOp::Int,
            ast::UnOp::Real => UnOp::Real,
            ast::UnOp::Not => UnOp::Not,
        };

        Expr::Unary(UnaryExpr { op, input, ty })
    }

    fn binary_expr(&mut self, scope: Id<Scope>, expr: &ast::BinaryExpr) -> Expr {
        let lhs = self.expr(scope, &expr.lhs);
        let rhs = self.expr(scope, &expr.rhs);

        let lhs = Box::new(lhs);
        let rhs = Box::new(rhs);

        let ty = match expr.op {
            ast::BinOp::Add | ast::BinOp::Mul | ast::BinOp::Sub => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);
                self.constrain_numeric(&lhs.ty(), Numeric::Real, expr.span);

                lhs.ty()
            }

            ast::BinOp::Div => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);
                self.unify(&lhs.ty(), &Ty::REAL, expr.span);

                Ty::REAL
            }

            ast::BinOp::Gt | ast::BinOp::Lt | ast::BinOp::GtEq | ast::BinOp::LtEq => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);

                Ty::bool()
            }

            ast::BinOp::And | ast::BinOp::Or => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);
                self.unify(&lhs.ty(), &Ty::bool(), expr.span);

                Ty::bool()
            }

            ast::BinOp::Eq | ast::BinOp::Ne => {
                self.unify(&lhs.ty(), &rhs.ty(), expr.span);

                Ty::bool()
            }
        };

        let op = match expr.op {
            ast::BinOp::Add => BinOp::Add,
            ast::BinOp::Sub => BinOp::Sub,
            ast::BinOp::Mul => BinOp::Mul,
            ast::BinOp::Div => BinOp::Div,
            ast::BinOp::Gt => BinOp::Gt,
            ast::BinOp::Lt => BinOp::Lt,
            ast::BinOp::GtEq => BinOp::Ge,
            ast::BinOp::LtEq => BinOp::Le,
            ast::BinOp::Eq => BinOp::Eq,
            ast::BinOp::Ne => BinOp::Ne,
            ast::BinOp::And => BinOp::And,
            ast::BinOp::Or => BinOp::Or,
        };

        Expr::Binary(BinaryExpr { op, lhs, rhs, ty })
    }

    fn tuple_expr(&mut self, scope: Id<Scope>, expr: &ast::TupleExpr) -> Expr {
        let fields = expr
            .fields
            .iter()
            .map(|field| self.expr(scope, field))
            .collect::<Vec<_>>();

        let tys = fields.iter().map(Expr::ty).collect();
        let ty = Ty::Tuple(tys);

        Expr::Tuple(TupleExpr { fields, ty })
    }

    fn block_expr(&mut self, scope: Id<Scope>, expr: &ast::BlockExpr) -> Expr {
        let scope = self.add_scope(ScopeKind::Block, scope);

        let defs = expr.stmts.iter().filter_map(ast::BlockStmt::as_def);

        self.import_defs(scope, defs.clone());
        self.alias_defs(scope, defs.clone());
        self.extern_defs(scope, defs);

        let mut exprs = Vec::new();

        for def in expr.stmts.iter().filter_map(ast::BlockStmt::as_let) {
            let (pat, val) = self.let_stmt(scope, VarKind::Local, def);
            exprs.push((pat, val));
        }

        let expr = self.expr(scope, &expr.expr);

        exprs.into_iter().rfold(expr, |expr, (pat, val)| {
            Expr::Let(LetExpr {
                pat,
                input: Box::new(val),
                output: Box::new(expr),
            })
        })
    }

    fn let_stmt(&mut self, scope: Id<Scope>, kind: VarKind, stmt: &ast::LetStmt) -> (Pat, Expr) {
        let expr = self.complete_let(scope, stmt.ty.as_ref(), &stmt.params, &stmt.expr, stmt.span);
        let pat = self.pat(scope, Visibility::Local, kind, &stmt.pat);

        self.unify(&pat.ty(), &expr.ty(), stmt.span);

        (pat, expr)
    }

    fn do_expr(&mut self, scope: Id<Scope>, expr: &ast::DoExpr) -> Expr {
        match expr.kind {
            ast::DoKind::Block(ref stmts) => self.do_expr_block(scope, stmts),

            ast::DoKind::Expr(ref expr) => {
                let expr = self.expr(scope, expr);
                let expr = Box::new(expr);

                let ty = Ty::monad(expr.ty());

                Expr::Pure(PureExpr { input: expr, ty })
            }
        }
    }

    fn do_expr_block(&mut self, scope: Id<Scope>, stmts: &[ast::DoStmt]) -> Expr {
        enum LetOrBind {
            Let(Pat, Expr),
            Bind(Id<Scope>, Pat, Expr),
        }

        let mut scope = self.add_scope(ScopeKind::Block, scope);

        let defs = stmts.iter().filter_map(ast::DoStmt::as_def);

        self.import_defs(scope, defs.clone());
        self.alias_defs(scope, defs.clone());
        self.extern_defs(scope, defs);

        let mut exprs = Vec::new();

        let mut output = Expr::Pure(PureExpr {
            input: Box::new(Expr::unit()),
            ty: Ty::monad(Ty::UNIT),
        });

        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;

            match stmt {
                ast::DoStmt::Let(stmt) => {
                    let (pat, expr) = self.let_stmt(scope, VarKind::Local, stmt);
                    exprs.push(LetOrBind::Let(pat, expr));
                }

                ast::DoStmt::Bind(stmt) => {
                    scope = self.add_scope(ScopeKind::Bind, scope);
                    let (pat, expr) = self.bind_stmt(scope, stmt);
                    exprs.push(LetOrBind::Bind(scope, pat, expr))
                }

                ast::DoStmt::Expr(expr) => {
                    if !is_last {
                        scope = self.add_scope(ScopeKind::Bind, scope);
                    }

                    let span = expr.span();
                    let expr = self.expr(scope, expr);
                    let ty = expr.ty();

                    let monad_ty = Ty::monad(self.add_inferred_type());
                    self.unify(&monad_ty, &ty, span);

                    if is_last {
                        output = expr;
                    } else {
                        let pat = Pat::Wild(WildPat { ty, span });
                        exprs.push(LetOrBind::Bind(scope, pat, expr))
                    }
                }

                ast::DoStmt::Def(..) => {}
            }
        }

        exprs.into_iter().rfold(output, |expr, kind| match kind {
            LetOrBind::Let(pat, input) => Expr::Let(LetExpr {
                pat,
                input: Box::new(input),
                output: Box::new(expr),
            }),

            LetOrBind::Bind(scope, pat, input) => Expr::Bind(BindExpr {
                scope,
                pat,
                input: Box::new(input),
                output: Box::new(expr),
            }),
        })
    }

    fn bind_stmt(&mut self, scope: Id<Scope>, stmt: &ast::BindStmt) -> (Pat, Expr) {
        let expr = self.complete_let(scope, stmt.ty.as_ref(), &stmt.params, &stmt.expr, stmt.span);
        let pat = self.pat(scope, Visibility::Local, VarKind::Local, &stmt.pat);

        let ty = self.add_inferred_type();
        let monad_ty = Ty::monad(ty.clone());

        self.unify(&monad_ty, &expr.ty(), stmt.span);
        self.unify(&pat.ty(), &ty, stmt.span);

        if let Some(ref ty) = stmt.ty {
            let ty = self.ty(scope, &mut Generics::dynamic(), ty);
            self.unify(&pat.ty(), &ty, stmt.span);
        }

        (pat, expr)
    }

    fn match_expr(&mut self, scope: Id<Scope>, expr: &ast::MatchExpr) -> Expr {
        let ty = self.add_inferred_type();

        let mut arms = Vec::new();
        let mut closed_column = exhaust::Column::Wild;
        let mut exhaustiveness_matrix = exhaust::Matrix::new();

        for arm in &expr.arms {
            let span = arm.expr.span();
            let scope = self.add_scope(ScopeKind::Block, scope);

            let arm = Arm {
                pat: self.pat(scope, Visibility::Local, VarKind::Local, &arm.pat),
                expr: self.expr(scope, &arm.expr),
            };

            self.unify(&ty, &arm.expr.ty(), span);

            let pat = exhaust::Pattern::new(&arm.pat);
            let column = exhaust::Column::from_pat(&pat);
            let row = exhaust::Row::new(pat);

            let Ok(column) = closed_column.merge(&column) else {
                let diagnostic = Diagnostic::error("invalid pattern")
                    .with_label(arm.pat.span(), "found here")
                    .with_label(expr.span, "in `match` here");

                self.emitter.emit(diagnostic);

                continue;
            };

            closed_column = column;
            exhaustiveness_matrix.push(row);

            arms.push(arm);
        }

        let open_column = exhaustiveness_matrix.open_column(closed_column);

        let mut usefulness_matrix = exhaust::Matrix::new();
        for arm in &arms {
            let pat = exhaust::Pattern::new(&arm.pat);
            let row = exhaust::Row::new(pat);

            if !usefulness_matrix.is_useful(&open_column, &row) {
                let diagnostic = Diagnostic::warning("unreachable pattern")
                    .with_label(arm.pat.span(), "found here");

                self.emitter.emit(diagnostic);
            }

            usefulness_matrix.push(row);
        }

        let unexchausted_pats = exhaustiveness_matrix.unexhausted_pats(&open_column);

        if !unexchausted_pats.is_empty() {
            let mut note = String::from("ensure that all possible cases are handled\n\n");

            for pat in unexchausted_pats {
                note += "        ";
                note += &pat.format();
                note += "\n";
            }

            note += "\n";

            let diagnostic = Diagnostic::error("`match` expression is not exhaustive")
                .with_label(expr.span, "found here")
                .with_note(note);

            self.emitter.emit(diagnostic);
        }

        let expr = {
            let target = self.expr(scope, &expr.expr);

            let ty = self.match_input_type(
                &open_column,
                arms.iter().map(|arm| &arm.pat),
                expr.expr.span(),
            );

            self.unify(&ty, &target.ty(), expr.expr.span());

            Box::new(target)
        };

        Expr::Match(MatchExpr {
            input: expr,
            arms,
            ty,
        })
    }

    fn error_expr(&mut self, _span: Span) -> Expr {
        Expr::Error(Ty::Error)
    }
}
