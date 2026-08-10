use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{
    arena::Id,
    diagnostic::{Diagnostic, Span},
    ir::{AliasTy, Bound, GenericTy, Numeric, RecordTy, Ty, TyField, UnionTy, Variant},
    lower::Lowerer,
};

impl Lowerer<'_> {
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

    pub(super) fn constrain_field(&mut self, target: &Ty, name: &'static str, ty: &Ty, span: Span) {
        if self.try_constrain_field(target, name, ty).is_err() {
            let diagnostic = Diagnostic::error(format!(
                "type `{}` does not have field `{}: {}`",
                self.format_ty(target),
                name,
                self.format_ty(ty),
            ))
            .with_label(span, "here");

            self.emitter.emit(diagnostic);
        }
    }

    pub(super) fn constrain_variant(
        &mut self,
        target: &Ty,
        name: &'static str,
        payload: Option<&Ty>,
        span: Span,
    ) {
        if self.try_constrain_variant(target, name, payload).is_err() {
            let diagnostic = Diagnostic::error(format!(
                "type `{}` does not have variant `{}{}`",
                self.format_ty(target),
                name,
                payload.map_or(String::new(), |ty| format!(" {}", self.format_ty(ty))),
            ))
            .with_label(span, "here");

            self.emitter.emit(diagnostic);
        }
    }

    fn try_constrain_numeric(&mut self, target: &Ty, bound: Numeric) -> Result<(), ()> {
        if let Some(target) = self.subst_shallow(target).cloned() {
            return self.try_constrain_numeric(&target, bound);
        }

        if let Ty::Error = target {
            return Ok(());
        }

        if let Ty::Alias(target) = target {
            let target = self.instantiate_alias(target);
            self.try_constrain_numeric(&target, bound)?;
            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bound::None = self.bounds[id]
        {
            self.bounds[id] = Bound::Numeric(bound);
            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bound::Numeric(ref mut target) = self.bounds[id]
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

    fn try_constrain_field(&mut self, target: &Ty, name: &'static str, ty: &Ty) -> Result<(), ()> {
        if let Some(target) = self.subst_shallow(target).cloned() {
            return self.try_constrain_field(&target, name, ty);
        }

        if let Ty::Error = target {
            return Ok(());
        }

        if let Ty::Alias(target) = target {
            let target = self.instantiate_alias(target);
            self.try_constrain_field(&target, name, ty)?;
            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bound::None = self.bounds[id]
        {
            self.bounds[id] = Bound::Record(RecordTy {
                fields: vec![TyField {
                    name,
                    ty: ty.clone(),
                }],
            });

            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bound::Record(ref target) = self.bounds[id]
            && let Some(target) = target.get(name)
        {
            let target = target.clone();
            return self.try_unify(&target, ty);
        }

        if let &Ty::Infer(id) = target
            && let Bound::Record(ref mut target) = self.bounds[id]
            && target.get(name).is_none()
        {
            target.fields.push(TyField {
                name,
                ty: ty.clone(),
            });

            return Ok(());
        }

        if let Ty::Record(target) = target
            && let Some(target) = target.get(name)
        {
            return self.try_unify(target, ty);
        }

        Err(())
    }

    fn try_constrain_variant(
        &mut self,
        target: &Ty,
        name: &'static str,
        payload: Option<&Ty>,
    ) -> Result<(), ()> {
        if let Some(target) = self.subst_shallow(target).cloned() {
            return self.try_constrain_variant(&target, name, payload);
        }

        if let Ty::Error = target {
            return Ok(());
        }

        if let Ty::Alias(target) = target {
            let target = self.instantiate_alias(target);
            self.try_constrain_variant(&target, name, payload)?;
            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bound::None = self.bounds[id]
        {
            self.bounds[id] = Bound::Union(UnionTy {
                variants: vec![Variant {
                    name,
                    payload: payload.cloned(),
                }],
            });

            return Ok(());
        }

        if let &Ty::Infer(id) = target
            && let Bound::Union(ref target) = self.bounds[id]
            && let Some(target) = target.get(name)
        {
            match (target, payload) {
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
            && let Bound::Union(ref mut target) = self.bounds[id]
            && target.get(name).is_none()
        {
            let payload = payload.cloned();
            target.variants.push(Variant { name, payload });
            return Ok(());
        }

        if let Ty::Union(target) = target
            && let Some(target) = target.get(name)
        {
            match (target, payload) {
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

        Err(())
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
        if lhs == rhs {
            return Ok(());
        }

        let mut state = DefaultHasher::new();
        lhs.hash(&mut state);
        rhs.hash(&mut state);

        let hash = state.finish();

        if !self.cache.insert(hash) {
            return Ok(());
        }

        if let Some(lhs) = self.subst_shallow(lhs).cloned() {
            return self.try_unify(&lhs, rhs);
        } else if let Some(rhs) = self.subst_shallow(rhs).cloned() {
            return self.try_unify(lhs, &rhs);
        }

        match (lhs, rhs) {
            (Ty::Infer(id), ty) => self.try_unify_infer_ty(*id, ty),
            (ty, Ty::Infer(id)) => self.try_unify_infer_ty(*id, ty),

            (lhs, rhs) => self.try_unify_ty_ty(lhs, rhs),
        }
    }

    fn try_unify_infer_ty(&mut self, id: Id<Bound>, ty: &Ty) -> Result<(), ()> {
        match self.bounds[id] {
            Bound::Numeric(bound) => self.try_constrain_numeric(ty, bound)?,

            Bound::Record(ref record) => {
                for field in record.fields.clone() {
                    self.try_constrain_field(ty, field.name, &field.ty)?;
                }
            }

            Bound::Union(ref union) => {
                for variant in union.variants.clone() {
                    self.try_constrain_variant(ty, variant.name, variant.payload.as_ref())?;
                }
            }

            Bound::None => {}
        }

        self.subst.insert(id, ty.clone());

        Ok(())
    }

    fn try_unify_ty_ty(&mut self, lhs: &Ty, rhs: &Ty) -> Result<(), ()> {
        match (lhs, rhs) {
            (Ty::Infer(..), _) | (_, Ty::Infer(..)) => unreachable!(),

            (Ty::Error, _) | (_, Ty::Error) => {}

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

            (Ty::Record(lhs_record), Ty::Record(rhs_record)) => {
                for lhs in &lhs_record.fields {
                    self.try_constrain_field(rhs, lhs.name, &lhs.ty)?;
                }

                for rhs in &rhs_record.fields {
                    self.try_constrain_field(lhs, rhs.name, &rhs.ty)?;
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

        let mut ty = self.aliases[ty.alias].ty.clone();

        Self::map_ty(&mut ty, |ty| {
            if let Ty::Infer(bound) = ty
                && let Some(subst) = map.get(bound)
            {
                *ty = subst.clone();
            }
        });

        ty
    }

    fn subst_shallow(&self, ty: &Ty) -> Option<&Ty> {
        match ty {
            Ty::Infer(id) => self.subst.get(id),
            _ => None,
        }
    }

    pub(super) fn instantiate_generics(&mut self, mut ty: Ty) -> Ty {
        fn recurse(lowerer: &mut Lowerer<'_>, ty: &mut Ty, map: &mut HashMap<Id<Bound>, Ty>) {
            Lowerer::map_ty(ty, |ty| {
                while let Ty::Infer(bound) | Ty::Generic(GenericTy { bound, .. }) = ty
                    && let Some(new) = map.get(bound).cloned()
                {
                    *ty = new;
                }

                if let Ty::Infer(bound) = *ty
                    && let Some(mut subst) = lowerer.subst.get(&bound).cloned()
                {
                    let new = lowerer.bounds.add(Bound::None);
                    map.insert(bound, Ty::Infer(new));

                    recurse(lowerer, &mut subst, map);
                    lowerer.subst.insert(new, subst);

                    *ty = Ty::Infer(new);

                    return;
                }

                let Ty::Generic(generic) = ty else {
                    return;
                };

                let new = lowerer.bounds.reserve();
                map.insert(generic.bound, Ty::Infer(new));

                let mut bounds = lowerer.bounds[generic.bound].clone();

                match bounds {
                    Bound::Numeric(..) | Bound::None => {}

                    Bound::Record(ref mut ty) => {
                        for field in &mut ty.fields {
                            recurse(lowerer, &mut field.ty, map);
                        }
                    }

                    Bound::Union(ref mut ty) => {
                        for variant in &mut ty.variants {
                            if let Some(ref mut payload) = variant.payload {
                                recurse(lowerer, payload, map);
                            }
                        }
                    }
                }

                lowerer.bounds.insert(new, bounds);
                *ty = Ty::Infer(new);
            });
        }

        let mut map = HashMap::new();
        recurse(self, &mut ty, &mut map);
        ty
    }

    pub(super) fn instantiate_inferred(&mut self, ty: Ty) -> Ty {
        self.instantiate_inferred_with(ty, HashMap::new())
    }

    fn instantiate_inferred_with(&mut self, mut ty: Ty, mut map: HashMap<Id<Bound>, Ty>) -> Ty {
        fn recurse(lowerer: &mut Lowerer<'_>, ty: &mut Ty, map: &mut HashMap<Id<Bound>, Ty>) {
            Lowerer::map_ty(ty, |ty| {
                let (Ty::Infer(bound) | Ty::Generic(GenericTy { bound, .. })) = *ty else {
                    return;
                };

                if let Some(new) = map.get(&bound).cloned() {
                    *ty = new;
                    return;
                }

                let new = lowerer.bounds.reserve();
                map.insert(bound, Ty::Infer(new));

                if let Some(mut subst) = lowerer.subst.get(&bound).cloned() {
                    recurse(lowerer, &mut subst, map);
                    lowerer.subst.insert(new, subst);
                }

                let mut bounds = lowerer.bounds[bound].clone();

                match bounds {
                    Bound::Numeric(..) | Bound::None => {}

                    Bound::Record(ref mut ty) => {
                        for field in &mut ty.fields {
                            recurse(lowerer, &mut field.ty, map);
                        }
                    }

                    Bound::Union(ref mut ty) => {
                        for variant in &mut ty.variants {
                            if let Some(ref mut payload) = variant.payload {
                                recurse(lowerer, payload, map);
                            }
                        }
                    }
                }

                lowerer.bounds.insert(new, bounds);
                *ty = Ty::Infer(new);
            })
        }

        recurse(self, &mut ty, &mut map);

        ty
    }

    fn map_ty(ty: &mut Ty, mut f: impl FnMut(&mut Ty)) {
        fn recurse_record(ty: &mut RecordTy, f: &mut dyn FnMut(&mut Ty)) {
            for field in &mut ty.fields {
                recurse(&mut field.ty, f);
            }
        }

        fn recurse_union(ty: &mut UnionTy, f: &mut dyn FnMut(&mut Ty)) {
            for variant in &mut ty.variants {
                if let Some(ref mut ty) = variant.payload {
                    recurse(ty, f);
                }
            }
        }

        fn recurse(ty: &mut Ty, f: &mut dyn FnMut(&mut Ty)) {
            f(ty);

            match ty {
                Ty::Infer(..) | Ty::Generic(..) | Ty::Numeric(..) | Ty::Str | Ty::Error => {}

                Ty::Tuple(fields) => {
                    for field in fields {
                        recurse(field, f);
                    }
                }

                Ty::Lambda(ty) => {
                    recurse(&mut ty.input, f);
                    recurse(&mut ty.output, f);
                }

                Ty::Alias(ty) => {
                    for arg in &mut ty.args {
                        recurse(arg, f);
                    }
                }

                Ty::Record(ty) => recurse_record(ty, f),
                Ty::Union(ty) => recurse_union(ty, f),
                Ty::Monad(ty) => recurse(ty, f),
            }
        }

        recurse(ty, &mut f);
    }
}
