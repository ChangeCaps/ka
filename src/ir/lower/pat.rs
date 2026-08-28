use crate::{
    arena::Id,
    ast,
    diagnostic::{Diagnostic, Span},
    ir::{
        BindPat, ConsPat, EmptyPat, ErrorPat, Pat, Scope, ScopeKind, StrPat, TuplePat, Ty, UnionTy,
        Var, VarKind, Variant, VariantPat, Visibility, Visible, WildPat, lower::Lowerer,
    },
};

impl Lowerer<'_> {
    pub(super) fn pat(
        &mut self,
        scope: Id<Scope>,
        vis: Visibility,
        kind: VarKind,
        pat: &ast::Pat,
    ) -> Pat {
        match pat {
            ast::Pat::Paren(pat) => self.paren_pat(scope, vis, kind, pat),
            ast::Pat::Wild(pat) => self.wild_pat(scope, vis, kind, pat),
            ast::Pat::Bind(pat) => self.bind_pat(scope, vis, kind, pat),
            ast::Pat::Str(pat) => self.str_pat(scope, vis, kind, pat),
            ast::Pat::Cons(pat) => self.cons_pat(scope, vis, kind, pat),
            ast::Pat::List(pat) => self.list_pat(scope, vis, kind, pat),
            ast::Pat::Tuple(pat) => self.tuple_pat(scope, vis, kind, pat),
            ast::Pat::Variant(pat) => self.variant_pat(scope, vis, kind, pat),
            ast::Pat::Error(span) => self.error_pat(*span),
        }
    }

    fn paren_pat(
        &mut self,
        scope: Id<Scope>,
        vis: Visibility,
        kind: VarKind,
        pat: &ast::ParenPat,
    ) -> Pat {
        self.pat(scope, vis, kind, &pat.pat)
    }

    fn wild_pat(
        &mut self,
        _scope: Id<Scope>,
        _vis: Visibility,
        _kind: VarKind,
        pat: &ast::WildPat,
    ) -> Pat {
        let ty = self.add_inferred_type();
        let span = pat.span;
        Pat::Wild(WildPat { ty, span })
    }

    fn bind_pat(
        &mut self,
        scope: Id<Scope>,
        vis: Visibility,
        kind: VarKind,
        pat: &ast::BindPat,
    ) -> Pat {
        if self.find_var(scope, pat.name).is_some()
            && matches!(self.scopes[scope].kind, ScopeKind::Global(..))
        {
            return self.duplicate_variable_binding(pat.span, pat.name);
        }

        let ty = self.add_inferred_type();
        let var = self.vars.add(Var {
            kind,
            name: pat.name,
            ty: ty.clone(),
            span: pat.span,
        });

        self.scopes[scope].vars.push(Visible::new(var, vis));

        let span = pat.span;
        Pat::Bind(BindPat { var, ty, span })
    }

    fn str_pat(
        &mut self,
        _scope: Id<Scope>,
        _vis: Visibility,
        _kind: VarKind,
        pat: &ast::StrPat,
    ) -> Pat {
        Pat::Str(StrPat {
            string: pat.string,
            span: pat.span,
        })
    }

    fn cons_pat(
        &mut self,
        scope: Id<Scope>,
        vis: Visibility,
        kind: VarKind,
        pat: &ast::ConsPat,
    ) -> Pat {
        let first = self.pat(scope, vis, kind, &pat.first);
        let rest = self.pat(scope, vis, kind, &pat.rest);

        let first = Box::new(first);
        let rest = Box::new(rest);

        let ty = rest.ty();
        let span = pat.span;

        Pat::Cons(ConsPat {
            first,
            rest,
            ty,
            span,
        })
    }

    fn list_pat(
        &mut self,
        _scope: Id<Scope>,
        _vis: Visibility,
        _kind: VarKind,
        pat: &ast::ListPat,
    ) -> Pat {
        let ty = self.add_inferred_type();
        let span = pat.span;

        Pat::Empty(EmptyPat { ty, span })
    }

    fn tuple_pat(
        &mut self,
        scope: Id<Scope>,
        vis: Visibility,
        kind: VarKind,
        pat: &ast::TuplePat,
    ) -> Pat {
        let fields = pat
            .fields
            .iter()
            .map(|field| self.pat(scope, vis, kind, field))
            .collect::<Vec<_>>();

        let field_tys = fields.iter().map(|field| field.ty()).collect();
        let ty = Ty::Tuple(field_tys);

        let span = pat.span;
        Pat::Tuple(TuplePat { fields, ty, span })
    }

    fn variant_pat(
        &mut self,
        scope: Id<Scope>,
        vis: Visibility,
        kind: VarKind,
        pat: &ast::VariantPat,
    ) -> Pat {
        let Some(name) = pat.name else {
            return self.error_pat(pat.span);
        };

        let span = pat.span;

        let payload = pat
            .pat
            .as_ref()
            .map(|pat| self.pat(scope, vis, kind, pat))
            .map(Box::new);

        let payload_ty = payload.as_deref().map(Pat::ty);

        let ty = Ty::Union(UnionTy {
            variants: vec![Variant {
                name,
                payload: payload_ty,
            }],
        });

        Pat::Variant(VariantPat {
            name,
            payload,
            ty,
            span,
        })
    }

    fn duplicate_variable_binding(&mut self, span: Span, name: &str) -> Pat {
        let diagnostic = Diagnostic::error(format!("redefinition of variable `{}`", name))
            .with_label(span, "found here");

        self.emitter.emit(diagnostic);
        self.error_pat(span)
    }

    fn error_pat(&mut self, span: Span) -> Pat {
        let ty = Ty::Error;
        Pat::Error(ErrorPat { ty, span })
    }
}
