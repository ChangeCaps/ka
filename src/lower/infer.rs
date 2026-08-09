use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{
    arena::Id,
    diagnostic::{Diagnostic, Span},
    ir::{AliasTy, Bounds, GenericTy, Numeric, RecordTy, Ty, UnionTy, Variant},
    lower::Lowerer,
};

impl Lowerer<'_> {
    pub(super) fn add_inferred_type(&mut self) -> Ty {
        let id = self.bounds.add(Bounds::None);
        Ty::Infer(id)
    }

    pub(super) fn constrain_numeric(&mut self, target: &Ty, bound: Numeric, span: Span) {
        if self.try_constrain_numeric(target, bound).is_err() {
            let diagnostic = Diagnostic::error(format!(
                "`{}` is not `{}`",
                self.format_ty(target),
                bound.as_str(),
            ))
            .with_label(span, "required here");

            self.emitter.emit(diagnostic);
        }
    }

    pub(super) fn try_constrain_numeric(&mut self, target: &Ty, bound: Numeric) -> Result<(), ()> {
        if let Some(target) = self.subst_shallow(target).cloned() {
            return self.try_constrain_numeric(&target, bound);
        }

        if let &Ty::Infer(id) = target
            && let Bounds::None = self.bounds[id]
        {
            self.bounds[id] = Bounds::Numeric(bound);
            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bounds::Numeric(ref mut target) = self.bounds[id]
            && *target >= bound
        {
            *target = bound;
            return Ok(());
        }

        if let Ty::Numeric(target) = target
            && *target <= bound
        {
            return Ok(());
        }

        Err(())
    }

    pub(super) fn constrain_variant(
        &mut self,
        target: &Ty,
        name: &'static str,
        ty: Option<&Ty>,
        span: Span,
    ) {
        if self.try_constrain_variant(target, name, ty).is_err() {
            let diagnostic = Diagnostic::error(format!(
                "constrain::variant, `{}`, {}, {:?}",
                self.format_ty(target),
                name,
                ty,
            ))
            .with_label(span, "here");

            self.emitter.emit(diagnostic);
        }
    }

    fn try_constrain_variant(
        &mut self,
        target: &Ty,
        name: &'static str,
        ty: Option<&Ty>,
    ) -> Result<(), ()> {
        if let Some(target) = self.subst_shallow(target).cloned() {
            return self.try_constrain_variant(&target, name, ty);
        }

        if let &Ty::Infer(id) = target
            && let Bounds::None = self.bounds[id]
        {
            self.bounds[id] = Bounds::Union(UnionTy {
                variants: vec![Variant {
                    name,
                    payload: ty.cloned(),
                }],
            });

            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bounds::Union(ref target) = self.bounds[id]
            && let Some(target) = target.get(name)
        {
            match (target, ty) {
                (Some(target), Some(ty)) => {
                    let target = target.clone();
                    self.try_unify(&target, ty)?;
                    return Ok(());
                }

                (None, None) => {
                    return Ok(());
                }

                _ => {}
            }
        }

        if let &Ty::Infer(id) = target
            && let Bounds::Union(ref mut target) = self.bounds[id]
            && target.get(name).is_none()
        {
            let ty = ty.cloned();
            target.variants.push(Variant { name, payload: ty });
            return Ok(());
        }

        if let Ty::Union(target) = target
            && let Some(target) = target.get(name)
        {
            match (target, ty) {
                (Some(target), Some(ty)) => {
                    self.try_unify(target, ty)?;
                    return Ok(());
                }

                (None, None) => {
                    return Ok(());
                }

                _ => {}
            }
        }

        if let Ty::Alias(target) = target {
            let target = self.instantiate_alias(target);
            self.try_constrain_variant(&target, name, ty)?;
            return Ok(());
        }

        Err(())
    }

    pub(super) fn instantiate(&mut self, ty: Ty) -> Ty {
        self.instantiate_with(ty, HashMap::new())
    }

    pub(super) fn instantiate_with(&mut self, mut ty: Ty, mut map: HashMap<Id<Bounds>, Ty>) -> Ty {
        fn recurse_record(
            lowerer: &mut Lowerer<'_>,
            ty: &mut RecordTy,
            map: &mut HashMap<Id<Bounds>, Ty>,
        ) {
            for field in &mut ty.fields {
                recurse(lowerer, &mut field.ty, map);
            }
        }

        fn recurse_union(
            lowerer: &mut Lowerer<'_>,
            ty: &mut UnionTy,
            map: &mut HashMap<Id<Bounds>, Ty>,
        ) {
            for variant in &mut ty.variants {
                if let Some(ref mut ty) = variant.payload {
                    recurse(lowerer, ty, map);
                }
            }
        }

        fn recurse(lowerer: &mut Lowerer<'_>, ty: &mut Ty, map: &mut HashMap<Id<Bounds>, Ty>) {
            match ty {
                Ty::Infer(bounds) | Ty::Generic(GenericTy { bounds, .. }) => {
                    if let Some(new) = map.get(bounds).cloned() {
                        *ty = new;
                    } else {
                        let new = lowerer.bounds.reserve();
                        map.insert(*bounds, Ty::Infer(new));

                        if let Some(mut subst) = lowerer.subst.get(bounds).cloned() {
                            recurse(lowerer, &mut subst, map);
                            lowerer.subst.insert(new, subst);
                        }

                        let mut bounds = lowerer.bounds[*bounds].clone();

                        match bounds {
                            Bounds::Numeric(..) => {}
                            Bounds::Record(ref mut ty) => recurse_record(lowerer, ty, map),
                            Bounds::Union(ref mut ty) => recurse_union(lowerer, ty, map),
                            Bounds::None => {}
                        }

                        lowerer.bounds.insert(new, bounds);
                        *ty = Ty::Infer(new);
                    }
                }

                Ty::Numeric(..) | Ty::Str => {}

                Ty::Tuple(fields) => {
                    for field in fields {
                        recurse(lowerer, field, map);
                    }
                }

                Ty::Lambda(ty) => {
                    recurse(lowerer, &mut ty.input, map);
                    recurse(lowerer, &mut ty.output, map);
                }

                Ty::Alias(ty) => {
                    for arg in &mut ty.args {
                        recurse(lowerer, arg, map);
                    }
                }

                Ty::Record(ty) => recurse_record(lowerer, ty, map),
                Ty::Union(ty) => recurse_union(lowerer, ty, map),
                Ty::Monad(ty) => recurse(lowerer, ty, map),
            }
        }

        recurse(self, &mut ty, &mut map);

        ty
    }

    pub(super) fn unify(&mut self, lhs: &Ty, rhs: &Ty, span: Span) {
        if self.try_unify(lhs, rhs).is_err() {
            let diagnostic = Diagnostic::error(format!(
                "expected type `{}` but found `{}`",
                self.format_ty(lhs),
                self.format_ty(rhs)
            ))
            .with_label(span, "required here");

            self.emitter.emit(diagnostic);
        }
    }

    fn try_unify(&mut self, lhs: &Ty, rhs: &Ty) -> Result<(), ()> {
        if let Some(lhs) = self.subst_shallow(lhs).cloned() {
            return self.try_unify(&lhs, rhs);
        } else if let Some(rhs) = self.subst_shallow(rhs).cloned() {
            return self.try_unify(lhs, &rhs);
        }

        let mut state = DefaultHasher::new();
        lhs.hash(&mut state);
        rhs.hash(&mut state);

        let hash = state.finish();

        if !self.cache.insert(hash) {
            return Ok(());
        }

        if lhs == rhs {
            return Ok(());
        }

        match (lhs, rhs) {
            (Ty::Infer(id), ty) => self.try_unify_infer_ty(*id, ty),
            (ty, Ty::Infer(id)) => self.try_unify_infer_ty(*id, ty),

            (lhs, rhs) => self.try_unify_ty_ty(lhs, rhs),
        }
    }

    fn try_unify_infer_ty(&mut self, id: Id<Bounds>, ty: &Ty) -> Result<(), ()> {
        match self.bounds[id] {
            Bounds::Numeric(bound) => self.try_constrain_numeric(ty, bound)?,

            Bounds::Record(..) => todo!(),

            Bounds::Union(ref target) => {
                for variant in target.variants.clone() {
                    self.try_constrain_variant(ty, variant.name, variant.payload.as_ref())?;
                }
            }

            Bounds::None => {}
        }

        self.subst.insert(id, ty.clone());

        Ok(())
    }

    fn try_unify_ty_ty(&mut self, lhs: &Ty, rhs: &Ty) -> Result<(), ()> {
        match (lhs, rhs) {
            (Ty::Infer(..), _) | (_, Ty::Infer(..)) => unreachable!(),

            (Ty::Generic(lhs), Ty::Generic(rhs)) if lhs == rhs => {}
            (Ty::Numeric(lhs), Ty::Numeric(rhs)) if lhs == rhs => {}
            (Ty::Str, Ty::Str) => {}

            (Ty::Lambda(lhs), Ty::Lambda(rhs)) => {
                self.try_unify(&lhs.input, &rhs.input)?;
                self.try_unify(&lhs.output, &rhs.output)?;
            }

            (Ty::Monad(lhs), Ty::Monad(rhs)) => {
                self.try_unify(lhs, rhs)?;
            }

            (Ty::Tuple(lhs), Ty::Tuple(rhs)) if lhs.len() == rhs.len() => {
                for (lhs, rhs) in lhs.iter().zip(rhs) {
                    self.try_unify(lhs, rhs)?;
                }
            }

            (Ty::Union(lhs_union), Ty::Union(rhs_union)) => {
                for lhs in &lhs_union.variants {
                    self.try_constrain_variant(rhs, lhs.name, lhs.payload.as_ref())?;
                }

                for rhs in &rhs_union.variants {
                    self.try_constrain_variant(lhs, rhs.name, rhs.payload.as_ref())?;
                }
            }

            (Ty::Alias(lhs), Ty::Alias(rhs)) if lhs.alias == rhs.alias => {
                for (lhs, rhs) in lhs.args.iter().zip(&rhs.args) {
                    self.try_unify(lhs, rhs)?;
                }
            }

            (Ty::Alias(lhs), rhs) => {
                let lhs = self.instantiate_alias(lhs);
                self.try_unify(&lhs, rhs)?;
            }

            (lhs, Ty::Alias(rhs)) => {
                let rhs = self.instantiate_alias(rhs);
                self.try_unify(lhs, &rhs)?;
            }

            (_, _) => return Err(()),
        }

        Ok(())
    }

    fn instantiate_alias(&mut self, ty: &AliasTy) -> Ty {
        let map = self.aliases[ty.alias]
            .params
            .iter()
            .copied()
            .zip(ty.args.iter().cloned())
            .collect::<HashMap<_, _>>();

        let ty = self.aliases[ty.alias].ty.clone();
        self.instantiate_with(ty, map)
    }

    fn subst_shallow(&self, ty: &Ty) -> Option<&Ty> {
        match ty {
            Ty::Infer(id) => self.subst.get(id),
            _ => None,
        }
    }
}
