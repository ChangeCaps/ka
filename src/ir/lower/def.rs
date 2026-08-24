use crate::{
    arena::Id,
    ast,
    diagnostic::{Diagnostic, Span},
    ir::{
        Alias, Bound, Expr, GenericTy, LambdaExpr, Scope, ScopeKind, Ty, VarKind, Visibility,
        Visible,
        lower::{
            Lowerer,
            ty::{Generic, Generics},
        },
    },
};

impl Lowerer<'_> {
    pub(super) fn complete_let(
        &mut self,
        scope: Id<Scope>,
        ty: Option<&ast::Ty>,
        params: &[ast::Pat],
        expr: &ast::Expr,
        span: Span,
    ) -> Expr {
        let expr = match params.is_empty() {
            true => self.expr(scope, expr),
            false => self.lambda(scope, params, expr),
        };

        if let Some(ty) = ty {
            let ty = self.ty(scope, &mut Generics::dynamic(), ty);
            self.unify(&ty, &expr.ty(), span);
        }

        expr
    }

    pub(super) fn aliases<'a>(
        &mut self,
        defs: impl IntoIterator<Item = (Id<Scope>, &'a ast::AliasDef)>,
    ) {
        let mut aliases = Vec::new();

        for (scope, def) in defs {
            if self.find_alias(scope, def.name).is_some() {
                let diagnostic =
                    Diagnostic::error(format!("alias `{}` is already defined", def.name))
                        .with_label(def.span, "here");

                self.emitter.emit(diagnostic);
                continue;
            }

            let mut generics: Vec<Generic> = Vec::new();
            let mut params = Vec::new();

            for param in &def.params {
                let bound = self.bounds.add(Bound::None);

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

                let generic = GenericTy { name, bound };
                let ty = Ty::Generic(generic);

                params.push(generic);
                generics.push(Generic { name, ty });
            }

            let name = def.name;
            let alias = self.aliases.add(Alias {
                name,
                params,
                ty: Ty::UNIT,
            });

            let vis = match def.is_local {
                true => Visibility::Local,
                false => Visibility::Global,
            };

            self.scopes[scope].aliases.push(Visible::new(alias, vis));
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
                let pat = self.pat(scope, Visibility::Local, VarKind::Local, pat);
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
                output: expr,
                ty,
            })
        })
    }
}
