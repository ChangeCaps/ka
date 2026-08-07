use crate::{
    arena::Id,
    ast,
    diagnostic::{Diagnostic, Span},
    ir,
    lower::Lowerer,
};

impl Lowerer<'_> {
    pub(super) fn pat(
        &mut self,
        scope: Id<ir::Scope>,
        kind: ir::VarKind,
        pat: &ast::Pat,
    ) -> ir::Pat {
        match pat {
            ast::Pat::Paren(pat) => self.paren_pat(scope, kind, pat),
            ast::Pat::Wild(pat) => self.wild_pat(scope, kind, pat),
            ast::Pat::Bind(pat) => self.bind_pat(scope, kind, pat),
            ast::Pat::Tuple(pat) => self.tuple_pat(scope, kind, pat),
            ast::Pat::Tag(pat) => self.tag_pat(scope, kind, pat),
            ast::Pat::Error(span) => self.error_pat(*span),
        }
    }

    fn paren_pat(
        &mut self,
        scope: Id<ir::Scope>,
        kind: ir::VarKind,
        pat: &ast::ParenPat,
    ) -> ir::Pat {
        self.pat(scope, kind, &pat.pat)
    }

    fn wild_pat(
        &mut self,
        _scope: Id<ir::Scope>,
        _kind: ir::VarKind,
        pat: &ast::WildPat,
    ) -> ir::Pat {
        let ty = self.add_inferred_type();
        let span = pat.span;
        ir::Pat::Wild(ir::WildPat { ty, span })
    }

    fn bind_pat(&mut self, scope: Id<ir::Scope>, kind: ir::VarKind, pat: &ast::BindPat) -> ir::Pat {
        if self.find_var(scope, pat.name).is_some()
            && matches!(self.scopes[scope].kind, ir::ScopeKind::Const(..))
        {
            return self.duplicate_variable_binding(pat.span, pat.name);
        }

        let ty = self.add_inferred_type();
        let var = self.vars.add(ir::Var {
            kind,
            name: pat.name,
            ty: ty.clone(),
        });

        self.scopes[scope].vars.push(var);

        let span = pat.span;
        ir::Pat::Bind(ir::BindPat { var, ty, span })
    }

    fn tuple_pat(
        &mut self,
        scope: Id<ir::Scope>,
        kind: ir::VarKind,
        pat: &ast::TuplePat,
    ) -> ir::Pat {
        let fields = pat
            .fields
            .iter()
            .map(|field| self.pat(scope, kind, field))
            .collect::<Vec<_>>();

        let field_tys = fields.iter().map(|field| field.ty()).collect();
        let ty = ir::Ty::Tuple(field_tys);

        let span = pat.span;
        ir::Pat::Tuple(ir::TuplePat { fields, ty, span })
    }

    fn tag_pat(&mut self, scope: Id<ir::Scope>, kind: ir::VarKind, pat: &ast::TagPat) -> ir::Pat {
        let Some(name) = pat.name else {
            return self.error_pat(pat.span);
        };

        let span = pat.span;

        let pat = pat
            .pat
            .as_ref()
            .map(|pat| self.pat(scope, kind, pat))
            .map(Box::new);

        let ty = pat.as_ref().map(|pat| pat.ty());

        let ty = ir::Ty::Union(ir::UnionTy {
            variants: vec![ir::Variant { name, ty }],
        });

        ir::Pat::Tag(ir::TagPat {
            name,
            pat,
            ty,
            span,
        })
    }

    fn duplicate_variable_binding(&mut self, span: Span, name: &str) -> ir::Pat {
        let diagnostic = Diagnostic::error(format!("redefinition of variable `{}`", name))
            .with_label(span, "found here");

        self.emitter.emit(diagnostic);
        self.error_pat(span)
    }

    fn error_pat(&mut self, span: Span) -> ir::Pat {
        let ty = self.add_inferred_type();
        ir::Pat::Error(ir::ErrorPat { ty, span })
    }
}
