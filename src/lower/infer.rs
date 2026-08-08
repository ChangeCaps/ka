use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    iter,
};

use crate::{
    arena::Id,
    diagnostic::{Diagnostic, Span},
    ir,
    lower::Lowerer,
};

impl Lowerer<'_> {
    pub(super) fn add_inferred_type(&mut self) -> ir::Ty {
        let id = self.bounds.add(ir::Bounds::None);
        ir::Ty::Infer(id)
    }

    pub(super) fn constrain_tag(
        &mut self,
        target: &ir::Ty,
        name: &'static str,
        ty: Option<&ir::Ty>,
        span: Span,
    ) {
        if let Some(target) = self.subst_shallow(target).cloned() {
            return self.constrain_tag(&target, name, ty, span);
        }

        if let &ir::Ty::Infer(id) = target
            && let ir::Bounds::None = self.bounds[id]
        {
            self.bounds[id] = ir::Bounds::Union(ir::UnionTy {
                variants: vec![ir::Variant {
                    name,
                    ty: ty.cloned(),
                }],
            });
        }

        if let &ir::Ty::Infer(id) = target
            && let ir::Bounds::Union(ref target) = self.bounds[id]
            && let Some(target) = target.get(name)
        {
            match (target, ty) {
                (Some(target), Some(ty)) => {
                    let target = target.clone();
                    self.unify(&target, ty, span);
                    return;
                }

                (None, None) => {
                    return;
                }

                _ => {}
            }
        }

        if let &ir::Ty::Infer(id) = target
            && let ir::Bounds::Union(ref mut target) = self.bounds[id]
            && target.get(name).is_none()
        {
            let ty = ty.cloned();
            target.variants.push(ir::Variant { name, ty });
            return;
        }

        if let ir::Ty::Union(target) = target
            && let Some(target) = target.get(name)
        {
            match (target, ty) {
                (Some(target), Some(ty)) => {
                    self.unify(target, ty, span);
                    return;
                }

                (None, None) => {
                    return;
                }

                _ => {}
            }
        }

        if let ir::Ty::Alias(target) = target {
            let target = self.instantiate_alias(target);
            self.constrain_tag(&target, name, ty, span);
            return;
        }

        let diagnostic = Diagnostic::error(format!(
            "constrain::tag, `{}`, {}, {:?}",
            self.format_ty(target),
            name,
            ty,
        ))
        .with_label(span, "here");

        self.emitter.emit(diagnostic);
    }

    pub(super) fn instantiate(&mut self, ty: ir::Ty) -> ir::Ty {
        self.instantiate_with(ty, HashMap::new())
    }

    pub(super) fn instantiate_with(
        &mut self,
        mut ty: ir::Ty,
        mut map: HashMap<Id<ir::Bounds>, ir::Ty>,
    ) -> ir::Ty {
        fn recurse_record(
            lowerer: &mut Lowerer<'_>,
            ty: &mut ir::RecordTy,
            map: &mut HashMap<Id<ir::Bounds>, ir::Ty>,
        ) {
            for field in &mut ty.fields {
                recurse(lowerer, &mut field.ty, map);
            }
        }

        fn recurse_union(
            lowerer: &mut Lowerer<'_>,
            ty: &mut ir::UnionTy,
            map: &mut HashMap<Id<ir::Bounds>, ir::Ty>,
        ) {
            for variant in &mut ty.variants {
                if let Some(ref mut ty) = variant.ty {
                    recurse(lowerer, ty, map);
                }
            }
        }

        fn recurse(
            lowerer: &mut Lowerer<'_>,
            ty: &mut ir::Ty,
            map: &mut HashMap<Id<ir::Bounds>, ir::Ty>,
        ) {
            match ty {
                ir::Ty::Infer(id) => {
                    if let Some(subst) = lowerer.subst.get(id) {
                        *ty = subst.clone();
                    } else if let Some(new) = map.get(id).cloned() {
                        *ty = new;
                    } else {
                        let new = lowerer.bounds.reserve();
                        map.insert(*id, ir::Ty::Infer(new));

                        let mut bounds = lowerer.bounds[*id].clone();

                        match bounds {
                            ir::Bounds::Record(ref mut ty) => recurse_record(lowerer, ty, map),
                            ir::Bounds::Union(ref mut ty) => recurse_union(lowerer, ty, map),
                            ir::Bounds::None => {}
                        }

                        lowerer.bounds.insert(new, bounds);
                        *id = new;
                    }
                }

                ir::Ty::Nat | ir::Ty::Int | ir::Ty::Num | ir::Ty::Str => {}

                ir::Ty::Tuple(fields) => {
                    for field in fields {
                        recurse(lowerer, field, map);
                    }
                }

                ir::Ty::Lambda(ty) => {
                    recurse(lowerer, &mut ty.input, map);
                    recurse(lowerer, &mut ty.output, map);
                }

                ir::Ty::Alias(ty) => {
                    for arg in &mut ty.args {
                        recurse(lowerer, arg, map);
                    }
                }

                ir::Ty::Record(ty) => recurse_record(lowerer, ty, map),
                ir::Ty::Union(ty) => recurse_union(lowerer, ty, map),
                ir::Ty::Monad(ty) => recurse(lowerer, ty, map),
            }
        }

        recurse(self, &mut ty, &mut map);

        ty
    }

    pub(super) fn format_ty(&self, ty: &ir::Ty) -> String {
        fn recurse_record(
            lowerer: &Lowerer<'_>,
            ty: &ir::RecordTy,
            infos: &HashMap<Id<ir::Bounds>, InferTyInfo>,
        ) -> String {
            let fields = ty
                .fields
                .iter()
                .map(|field| {
                    let ty = recurse(lowerer, &field.ty, infos, 0);
                    format!("{}: {}", field.name, ty)
                })
                .collect::<Vec<_>>()
                .join(", ");

            if fields.is_empty() {
                String::from("{}")
            } else {
                format!("{{ {fields} }}")
            }
        }

        fn recurse_union(
            lowerer: &Lowerer<'_>,
            ty: &ir::UnionTy,
            infos: &HashMap<Id<ir::Bounds>, InferTyInfo>,
        ) -> String {
            ty.variants
                .iter()
                .map(|variant| {
                    let f = format!(":{}", variant.name);

                    match variant.ty {
                        Some(ref ty) => {
                            let ty = recurse(lowerer, ty, infos, 4);

                            format!("{f} {ty}")
                        }

                        None => f,
                    }
                })
                .collect::<Vec<_>>()
                .join(" | ")
        }

        fn recurse(
            lowerer: &Lowerer<'_>,
            ty: &ir::Ty,
            infos: &HashMap<Id<ir::Bounds>, InferTyInfo>,
            precedence: u8,
        ) -> String {
            match ty {
                ir::Ty::Infer(id) => {
                    if let Some(info) = infos.get(id) {
                        format!("'{}", info.name)
                    } else {
                        let subst = lowerer.subst.get(id).unwrap();
                        recurse(lowerer, subst, infos, precedence)
                    }
                }

                ir::Ty::Nat => String::from("nat"),
                ir::Ty::Int => String::from("int"),
                ir::Ty::Num => String::from("num"),
                ir::Ty::Str => String::from("str"),

                ir::Ty::Tuple(fields) => {
                    let f = fields
                        .iter()
                        .map(|field| recurse(lowerer, field, infos, 0))
                        .collect::<Vec<_>>()
                        .join(", ");

                    if precedence > 0 { format!("({f})") } else { f }
                }

                ir::Ty::Record(ty) => recurse_record(lowerer, ty, infos),

                ir::Ty::Union(ty) => {
                    let f = recurse_union(lowerer, ty, infos);

                    if precedence > 4 { format!("({f})") } else { f }
                }

                ir::Ty::Alias(ty) => {
                    let args = ty.args.iter().map(|ty| recurse(lowerer, ty, infos, 5));
                    let f = iter::once(lowerer.aliases[ty.alias].name.to_string())
                        .chain(args)
                        .collect::<Vec<_>>()
                        .join(" ");

                    if precedence > 4 { format!("({f})") } else { f }
                }

                ir::Ty::Lambda(ty) => {
                    let input = recurse(lowerer, &ty.input, infos, 2);
                    let output = recurse(lowerer, &ty.output, infos, 1);

                    let f = format!("{} -> {}", input, output);

                    if precedence > 1 { format!("({f})") } else { f }
                }

                ir::Ty::Monad(ty) => {
                    let ty = recurse(lowerer, ty, infos, 0);
                    let f = format!("!{ty}");

                    if precedence > 1 { format!("({f})") } else { f }
                }
            }
        }

        let infos = self.enumerate_infer_ty(ty);

        let bounds = infos
            .iter()
            .filter_map(|(id, info)| {
                let bound = if info.recursive
                    && let Some(ty) = self.subst.get(id)
                {
                    recurse(self, ty, &infos, 0)
                } else {
                    match self.bounds[*id] {
                        ir::Bounds::Record(ref ty) => recurse_record(self, ty, &infos),
                        ir::Bounds::Union(ref ty) => recurse_union(self, ty, &infos),
                        ir::Bounds::None => return None,
                    }
                };

                Some(format!("'{}: {bound}", info.name))
            })
            .collect::<Vec<_>>()
            .join(" and ");

        let f = recurse(self, ty, &infos, 0);

        if !bounds.is_empty() {
            format!("{f} where {bounds}")
        } else {
            f
        }
    }

    pub(super) fn enumerate_infer_ty(&self, ty: &ir::Ty) -> HashMap<Id<ir::Bounds>, InferTyInfo> {
        fn recurse_record(
            lowerer: &Lowerer<'_>,
            ty: &ir::RecordTy,
            seen: &mut Vec<Id<ir::Bounds>>,
            infos: &mut HashMap<Id<ir::Bounds>, InferTyInfo>,
        ) {
            for field in &ty.fields {
                recurse(lowerer, &field.ty, seen, infos);
            }
        }

        fn recurse_union(
            lowerer: &Lowerer<'_>,
            ty: &ir::UnionTy,
            seen: &mut Vec<Id<ir::Bounds>>,
            infos: &mut HashMap<Id<ir::Bounds>, InferTyInfo>,
        ) {
            for variant in &ty.variants {
                if let Some(ref ty) = variant.ty {
                    recurse(lowerer, ty, seen, infos);
                }
            }
        }

        fn recurse_infer(
            lowerer: &Lowerer<'_>,
            mut id: Id<ir::Bounds>,
            seen: &mut Vec<Id<ir::Bounds>>,
            infos: &mut HashMap<Id<ir::Bounds>, InferTyInfo>,
        ) {
            let n = infos.len();

            while let Some(ir::Ty::Infer(subst)) = lowerer.subst.get(&id) {
                id = *subst;
            }

            if seen.contains(&id) {
                let info = infos.entry(id).or_insert_with(|| InferTyInfo::new(n));
                info.recursive = true;
                return;
            }

            seen.push(id);

            if let Some(ty) = lowerer.subst.get(&id) {
                recurse(lowerer, ty, seen, infos);
                seen.pop();
                return;
            }

            let info = infos.entry(id).or_insert_with(|| InferTyInfo::new(n));
            info.occurences += 1;

            match lowerer.bounds[id] {
                ir::Bounds::Record(ref ty) => recurse_record(lowerer, ty, seen, infos),
                ir::Bounds::Union(ref ty) => recurse_union(lowerer, ty, seen, infos),
                ir::Bounds::None => {}
            }

            seen.pop();
        }

        fn recurse(
            lowerer: &Lowerer<'_>,
            ty: &ir::Ty,
            seen: &mut Vec<Id<ir::Bounds>>,
            infos: &mut HashMap<Id<ir::Bounds>, InferTyInfo>,
        ) {
            match ty {
                ir::Ty::Infer(id) => recurse_infer(lowerer, *id, seen, infos),

                ir::Ty::Nat | ir::Ty::Int | ir::Ty::Num | ir::Ty::Str => {}

                ir::Ty::Tuple(fields) => {
                    for field in fields {
                        recurse(lowerer, field, seen, infos);
                    }
                }

                ir::Ty::Lambda(ty) => {
                    recurse(lowerer, &ty.input, seen, infos);
                    recurse(lowerer, &ty.output, seen, infos);
                }

                ir::Ty::Alias(ty) => {
                    for arg in &ty.args {
                        recurse(lowerer, arg, seen, infos);
                    }
                }

                ir::Ty::Record(ty) => recurse_record(lowerer, ty, seen, infos),
                ir::Ty::Union(ty) => recurse_union(lowerer, ty, seen, infos),
                ir::Ty::Monad(ty) => recurse(lowerer, ty, seen, infos),
            }
        }

        let mut seen = Vec::new();
        let mut info = HashMap::new();
        recurse(self, ty, &mut seen, &mut info);
        info
    }

    pub(super) fn unify(&mut self, lhs: &ir::Ty, rhs: &ir::Ty, span: Span) {
        if let Some(lhs) = self.subst_shallow(lhs).cloned() {
            return self.unify(&lhs, rhs, span);
        } else if let Some(rhs) = self.subst_shallow(rhs).cloned() {
            return self.unify(lhs, &rhs, span);
        }

        let mut state = DefaultHasher::new();
        lhs.hash(&mut state);
        rhs.hash(&mut state);

        let hash = state.finish();

        if !self.cache.insert(hash) {
            return;
        }

        if lhs == rhs {
            return;
        }

        match (lhs, rhs) {
            (ir::Ty::Infer(id), ty) => self.unify_infer_ty(*id, ty, span),
            (ty, ir::Ty::Infer(id)) => self.unify_infer_ty(*id, ty, span),

            (lhs, rhs) => self.unify_ty_ty(lhs, rhs, span),
        }
    }

    fn unify_infer_ty(&mut self, id: Id<ir::Bounds>, ty: &ir::Ty, span: Span) {
        match self.bounds[id] {
            ir::Bounds::Record(..) => todo!(),

            ir::Bounds::Union(ref target) => {
                for variant in target.variants.clone() {
                    self.constrain_tag(ty, variant.name, variant.ty.as_ref(), span);
                }
            }

            ir::Bounds::None => {}
        }

        self.subst.insert(id, ty.clone());
    }

    fn unify_ty_ty(&mut self, lhs: &ir::Ty, rhs: &ir::Ty, span: Span) {
        match (lhs, rhs) {
            (ir::Ty::Num, ir::Ty::Num) => {}

            (ir::Ty::Lambda(lhs), ir::Ty::Lambda(rhs)) => {
                self.unify(&lhs.input, &rhs.input, span);
                self.unify(&lhs.output, &rhs.output, span);
            }

            (ir::Ty::Monad(lhs), ir::Ty::Monad(rhs)) => {
                self.unify(lhs, rhs, span);
            }

            (ir::Ty::Tuple(lhs), ir::Ty::Tuple(rhs)) if lhs.len() == rhs.len() => {
                for (lhs, rhs) in lhs.iter().zip(rhs) {
                    self.unify(lhs, rhs, span);
                }
            }

            (ir::Ty::Union(lhs_union), ir::Ty::Union(rhs_union)) => {
                for lhs in &lhs_union.variants {
                    self.constrain_tag(rhs, lhs.name, lhs.ty.as_ref(), span);
                }

                for rhs in &rhs_union.variants {
                    self.constrain_tag(lhs, rhs.name, rhs.ty.as_ref(), span);
                }
            }

            (ir::Ty::Alias(lhs), ir::Ty::Alias(rhs)) if lhs.alias == rhs.alias => {
                for (lhs, rhs) in lhs.args.iter().zip(&rhs.args) {
                    self.unify(lhs, rhs, span);
                }
            }

            (ir::Ty::Alias(lhs), rhs) => {
                let lhs = self.instantiate_alias(lhs);
                self.unify(&lhs, rhs, span);
            }

            (lhs, ir::Ty::Alias(rhs)) => {
                let rhs = self.instantiate_alias(rhs);
                self.unify(lhs, &rhs, span);
            }

            (lhs, rhs) => {
                let diagnostic = Diagnostic::error(format!(
                    "expected type `{}` but found `{}`",
                    self.format_ty(lhs),
                    self.format_ty(rhs)
                ))
                .with_label(span, "required here");

                self.emitter.emit(diagnostic);
            }
        }
    }

    fn instantiate_alias(&mut self, ty: &ir::AliasTy) -> ir::Ty {
        let map = self.aliases[ty.alias]
            .params
            .iter()
            .copied()
            .zip(ty.args.iter().cloned())
            .collect::<HashMap<_, _>>();

        let ty = self.aliases[ty.alias].ty.clone();
        self.instantiate_with(ty, map)
    }

    fn subst_shallow(&self, ty: &ir::Ty) -> Option<&ir::Ty> {
        match ty {
            ir::Ty::Infer(id) => self.subst.get(id),
            _ => None,
        }
    }
}

pub(super) struct InferTyInfo {
    pub recursive: bool,
    pub occurences: usize,
    pub name: String,
}

impl InferTyInfo {
    fn new(n: usize) -> Self {
        Self {
            recursive: false,
            occurences: 0,
            name: Self::generate_name(n),
        }
    }

    fn generate_name(index: usize) -> String {
        let letters = "abcdefghijklmnopqrstuvwxyz";

        let mut name = String::new();
        let mut i = index + 1;

        while i > 0 {
            let letter_index = (i - 1) % letters.len();
            name.push(letters.chars().nth(letter_index).unwrap());
            i = (i - 1) / letters.len();
        }

        name.chars().rev().collect()
    }
}
