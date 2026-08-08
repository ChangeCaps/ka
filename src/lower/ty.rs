use crate::{arena::Id, ast, diagnostic::Diagnostic, ir, lower::Lowerer};

pub(super) enum Generics<'a> {
    Static(&'a [Generic]),
    Dynamic(Vec<Generic>),
}

pub(super) struct Generic {
    pub name: &'static str,
    pub ty: ir::Ty,
}

impl Generics<'_> {
    pub(super) fn dynamic() -> Self {
        Self::Dynamic(Vec::new())
    }

    fn as_slice(&self) -> &[Generic] {
        match self {
            Generics::Static(generics) => generics,
            Generics::Dynamic(generics) => generics,
        }
    }
}

impl Lowerer<'_> {
    pub(super) fn ty(
        &mut self,
        scope: Id<ir::Scope>,
        generics: &mut Generics,
        ty: &ast::Ty,
    ) -> ir::Ty {
        match ty {
            ast::Ty::Nat => ir::Ty::Nat,
            ast::Ty::Int => ir::Ty::Int,
            ast::Ty::Num => ir::Ty::Num,
            ast::Ty::Str => ir::Ty::Str,

            ast::Ty::Paren(ty) => self.ty(scope, generics, &ty.ty),

            ast::Ty::Lambda(ty) => {
                let input = self.ty(scope, generics, &ty.input);
                let output = self.ty(scope, generics, &ty.output);

                ir::Ty::lambda(input, output)
            }

            ast::Ty::Tuple(ty) => {
                let fields = ty
                    .fields
                    .iter()
                    .map(|ty| self.ty(scope, generics, ty))
                    .collect();

                ir::Ty::Tuple(fields)
            }

            ast::Ty::Record(ty) => self.record_ty(scope, generics, ty),

            ast::Ty::Monad(ty) => {
                let ty = self.ty(scope, generics, &ty.ty);
                ir::Ty::Monad(Box::new(ty))
            }

            ast::Ty::Generic(ty) => self.generic_ty(scope, generics, ty),
            ast::Ty::Union(ty) => self.union_ty(scope, generics, ty),
            ast::Ty::Alias(ty) => self.alias_ty(scope, generics, ty),

            ast::Ty::Error(..) => self.add_inferred_type(),
        }
    }

    fn record_ty(
        &mut self,
        scope: Id<ir::Scope>,
        generics: &mut Generics,
        ty: &ast::RecordTy,
    ) -> ir::Ty {
        let mut fields: Vec<ir::TyField> = Vec::new();

        for field in &ty.fields {
            let Some(name) = field.name else {
                continue;
            };

            let ty = self.ty(scope, generics, &field.ty);

            if fields.iter().any(|f| f.name == name) {
                let diagnostic = Diagnostic::error(format!("field `{}` already defined", name))
                    .with_label(field.span, "here");

                self.emitter.emit(diagnostic);
                continue;
            }

            fields.push(ir::TyField { name, ty });
        }

        ir::Ty::Record(ir::RecordTy { fields })
    }

    fn generic_ty(
        &mut self,
        _scope: Id<ir::Scope>,
        generics: &mut Generics,
        ty: &ast::GenericTy,
    ) -> ir::Ty {
        if let Some(generic) = generics.as_slice().iter().find(|g| g.name == ty.name) {
            return generic.ty.clone();
        }

        match generics {
            Generics::Static(_) => {
                let diagnostic = Diagnostic::error(format!("invalid generic `'{}`", ty.name))
                    .with_label(ty.span, "found here");

                self.emitter.emit(diagnostic);
                self.add_inferred_type()
            }

            Generics::Dynamic(generics) => {
                let infer = self.add_inferred_type();

                generics.push(Generic {
                    name: ty.name,
                    ty: infer.clone(),
                });

                infer
            }
        }
    }

    fn alias_ty(
        &mut self,
        scope: Id<ir::Scope>,
        generics: &mut Generics,
        ty: &ast::AliasTy,
    ) -> ir::Ty {
        let Some(alias) = self.resolve_alias(scope, ty.import, ty.name) else {
            let diagnostic = Diagnostic::error(format!("type alias `{}` not defined", ty.name))
                .with_label(ty.span, "found here");

            self.emitter.emit(diagnostic);
            return self.add_inferred_type();
        };

        let args = ty
            .args
            .iter()
            .map(|ty| self.ty(scope, generics, ty))
            .collect::<Vec<_>>();

        if self.aliases[alias].params.len() != args.len() {
            let diagnostic =
                Diagnostic::error(format!("wrong number of arguments `{}`", ty.args.len()))
                    .with_label(ty.span, "found here");

            self.emitter.emit(diagnostic);
            return self.add_inferred_type();
        }

        ir::Ty::Alias(ir::AliasTy { alias, args })
    }

    fn union_ty(
        &mut self,
        scope: Id<ir::Scope>,
        generics: &mut Generics,
        ty: &ast::UnionTy,
    ) -> ir::Ty {
        let mut variants: Vec<ir::Variant> = Vec::new();

        for variant in &ty.variants {
            let Some(name) = variant.name else {
                continue;
            };

            if variants.iter().any(|v| v.name == name) {
                let diagnostic =
                    Diagnostic::error(format!("duplicate definition of variant `:{}`", name))
                        .with_label(variant.span, "here");

                self.emitter.emit(diagnostic);

                continue;
            }

            let ty = variant.ty.as_ref().map(|ty| self.ty(scope, generics, ty));

            variants.push(ir::Variant { name, ty });
        }

        ir::Ty::Union(ir::UnionTy { variants })
    }
}
