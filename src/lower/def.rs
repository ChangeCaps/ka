use crate::{
    arena::Id,
    ast,
    diagnostic::{Diagnostic, Span},
    ir::{Alias, Bound, Expr, LambdaExpr, Pat, Scope, ScopeKind, Ty, VarKind},
    lower::{
        Lowerer,
        ty::{Generic, Generics},
    },
};

impl Lowerer<'_> {
    pub(super) fn register_let(
        &mut self,
        scope: Id<Scope>,
        kind: VarKind,
        ty: Option<&ast::Ty>,
        pat: &ast::Pat,
        span: Span,
    ) -> Pat {
        let pat = self.pat(scope, kind, pat);

        if let Some(ty) = ty {
            let ty = self.ty(scope, &mut Generics::dynamic(), ty);
            self.unify(&pat.ty(), &ty, span);
        }

        pat
    }

    pub(super) fn complete_let(
        &mut self,
        scope: Id<Scope>,
        params: &[ast::Pat],
        expr: &ast::Expr,
    ) -> Expr {
        if !params.is_empty() {
            let expr = self.lambda(scope, params, expr);
            return expr;
        }

        self.expr(scope, expr)
    }

    pub(super) fn aliases<'a>(
        &mut self,
        defs: impl IntoIterator<Item = (Id<Scope>, &'a ast::AliasDef)>,
    ) {
        let mut aliases = Vec::new();

        for (scope, def) in defs {
            if self.scopes[scope]
                .aliases
                .iter()
                .any(|id| self.aliases[*id].name == def.name)
            {
                let diagnostic =
                    Diagnostic::error(format!("alias `{}` is already defined", def.name))
                        .with_label(def.span, "here");

                self.emitter.emit(diagnostic);
                continue;
            }

            let mut generics: Vec<Generic> = Vec::new();
            let mut params = Vec::new();

            for param in &def.params {
                let bounds = self.bounds.add(Bound::None);
                params.push(bounds);

                let Some(name) = param else {
                    continue;
                };

                if generics.iter().any(|generic| generic.name == *name) {
                    let diagnostic =
                        Diagnostic::error(format!("generic `'{}` is already defined", name))
                            .with_label(def.span, "here");

                    self.emitter.emit(diagnostic);
                    continue;
                }

                let ty = Ty::Infer(bounds);
                generics.push(Generic { name, ty });
            }

            let name = def.name;
            let alias = self.aliases.add(Alias {
                name,
                params,
                ty: Ty::UNIT,
            });

            self.scopes[scope].aliases.push(alias);
            aliases.push((scope, generics, alias, def));
        }

        for (scope, generics, alias, def) in aliases {
            let ty = self.ty(scope, &mut Generics::Static(&generics), &def.ty);
            self.aliases[alias].ty = ty;
        }
    }

    pub(super) fn lambda(
        &mut self,
        mut scope: Id<Scope>,
        params: &[ast::Pat],
        expr: &ast::Expr,
    ) -> Expr {
        // this function works by going through each parameter in the forward direction, creating a
        // new scope and lowering the pattern, then going back through the parameters in the reverse
        // direction, creating lambdas and wrapping the inner expression

        let lambdas = params
            .iter()
            .map(|pat| {
                scope = self.add_scope(ScopeKind::Lambda, scope);
                let pat = self.pat(scope, VarKind::Local, pat);
                (scope, pat)
            })
            .collect::<Vec<_>>();

        let expr = self.expr(scope, expr);

        lambdas.into_iter().rfold(expr, |expr, (scope, input)| {
            let ty = Ty::lambda(input.ty(), expr.ty());
            let expr = Box::new(expr);

            Expr::Lambda(LambdaExpr {
                scope,
                input,
                expr,
                ty,
            })
        })
    }
}
