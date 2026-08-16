use std::{collections::HashMap, iter};

use crate::{
    arena::Id,
    ast,
    diagnostic::Diagnostic,
    ir::{
        AliasTy, Bound, GenericTy, RecordTy, Scope, Ty, TyField, UnionTy, Variant, lower::Lowerer,
    },
};

pub(super) enum Generics<'a> {
    Static(&'a [Generic]),
    Dynamic(Vec<Generic>),
}

pub(super) struct Generic {
    pub name: &'static str,
    pub ty: Ty,
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
    pub(super) fn add_inferred_type(&mut self) -> Ty {
        let id = self.bounds.add(Bound::None);
        Ty::Infer(id)
    }

    pub(super) fn ty(&mut self, scope: Id<Scope>, generics: &mut Generics, ty: &ast::Ty) -> Ty {
        match ty {
            ast::Ty::Nat => Ty::NAT,
            ast::Ty::Int => Ty::INT,
            ast::Ty::Real => Ty::REAL,
            ast::Ty::Str => Ty::Str,

            ast::Ty::Paren(ty) => self.ty(scope, generics, &ty.ty),

            ast::Ty::Lambda(ty) => {
                let input = self.ty(scope, generics, &ty.input);
                let output = self.ty(scope, generics, &ty.output);

                Ty::lambda(input, output)
            }

            ast::Ty::Tuple(ty) => {
                let fields = ty
                    .fields
                    .iter()
                    .map(|ty| self.ty(scope, generics, ty))
                    .collect();

                Ty::Tuple(fields)
            }

            ast::Ty::Record(ty) => self.record_ty(scope, generics, ty),

            ast::Ty::Monad(ty) => {
                let ty = self.ty(scope, generics, &ty.ty);
                Ty::Monad(Box::new(ty))
            }

            ast::Ty::Generic(ty) => self.generic_ty(scope, generics, ty),
            ast::Ty::Union(ty) => self.union_ty(scope, generics, ty),
            ast::Ty::Alias(ty) => self.alias_ty(scope, generics, ty),

            ast::Ty::Error(..) => Ty::Error,
        }
    }

    fn record_ty(&mut self, scope: Id<Scope>, generics: &mut Generics, ty: &ast::RecordTy) -> Ty {
        let mut fields: Vec<TyField> = Vec::new();

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

            fields.push(TyField { name, ty });
        }

        Ty::Record(RecordTy { fields })
    }

    fn generic_ty(
        &mut self,
        _scope: Id<Scope>,
        generics: &mut Generics,
        ty: &ast::GenericTy,
    ) -> Ty {
        if let Some(generic) = generics.as_slice().iter().find(|g| g.name == ty.name) {
            return generic.ty.clone();
        }

        match generics {
            Generics::Static(_) => {
                let diagnostic = Diagnostic::error(format!("invalid generic `'{}`", ty.name))
                    .with_label(ty.span, "found here");

                self.emitter.emit(diagnostic);
                Ty::Error
            }

            Generics::Dynamic(generics) => {
                let bounds = self.bounds.add(Bound::None);
                let generic = Ty::Generic(GenericTy {
                    name: ty.name,
                    bound: bounds,
                });

                generics.push(Generic {
                    name: ty.name,
                    ty: generic.clone(),
                });

                generic.clone()
            }
        }
    }

    fn alias_ty(&mut self, scope: Id<Scope>, generics: &mut Generics, ty: &ast::AliasTy) -> Ty {
        let Some(alias) = self.resolve_alias(scope, ty.import, ty.name) else {
            let diagnostic = Diagnostic::error(format!("type alias `{}` not defined", ty.name))
                .with_label(ty.span, "found here");

            self.emitter.emit(diagnostic);
            return Ty::Error;
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
            return Ty::Error;
        }

        Ty::Alias(AliasTy { alias, args })
    }

    fn union_ty(&mut self, scope: Id<Scope>, generics: &mut Generics, ty: &ast::UnionTy) -> Ty {
        let mut variants: Vec<Variant> = Vec::new();

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

            let payload = variant
                .payload
                .as_ref()
                .map(|ty| self.ty(scope, generics, ty));

            variants.push(Variant { name, payload });
        }

        Ty::Union(UnionTy { variants })
    }

    pub(super) fn format_ty(&self, ty: &Ty) -> String {
        fn recurse_record(
            lowerer: &Lowerer<'_>,
            ty: &RecordTy,
            infos: &HashMap<Id<Bound>, BoundsInfo>,
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
            ty: &UnionTy,
            infos: &HashMap<Id<Bound>, BoundsInfo>,
        ) -> String {
            ty.variants
                .iter()
                .map(|variant| {
                    let f = format!(":{}", variant.name);

                    match variant.payload {
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
            ty: &Ty,
            infos: &HashMap<Id<Bound>, BoundsInfo>,
            precedence: u8,
        ) -> String {
            match ty {
                Ty::Numeric(numeric) => String::from(numeric.as_str()),
                Ty::Str => String::from("str"),
                Ty::Error => String::from("_"),

                Ty::Infer(bounds) => {
                    if let Some(info) = infos.get(bounds) {
                        format!("'{}", info.name)
                    } else {
                        let subst = lowerer.subst.get(bounds).unwrap();
                        recurse(lowerer, subst, infos, precedence)
                    }
                }

                Ty::Generic(generic) => {
                    format!("'{}", generic.name)
                }

                Ty::Tuple(fields) => {
                    let f = fields
                        .iter()
                        .map(|field| recurse(lowerer, field, infos, 0))
                        .collect::<Vec<_>>()
                        .join(", ");

                    if precedence > 0 { format!("({f})") } else { f }
                }

                Ty::Record(ty) => recurse_record(lowerer, ty, infos),

                Ty::Union(ty) => {
                    let f = recurse_union(lowerer, ty, infos);

                    if precedence > 4 { format!("({f})") } else { f }
                }

                Ty::Alias(ty) => {
                    let args = ty.args.iter().map(|ty| recurse(lowerer, ty, infos, 5));
                    let f = iter::once(lowerer.aliases[ty.alias].name.to_string())
                        .chain(args)
                        .collect::<Vec<_>>()
                        .join(" ");

                    if precedence > 4 { format!("({f})") } else { f }
                }

                Ty::Lambda(ty) => {
                    let input = recurse(lowerer, &ty.input, infos, 2);
                    let output = recurse(lowerer, &ty.output, infos, 1);

                    let f = format!("{} -> {}", input, output);

                    if precedence > 1 { format!("({f})") } else { f }
                }

                Ty::Monad(ty) => {
                    let ty = recurse(lowerer, ty, infos, 2);
                    let f = format!("!{ty}");

                    if precedence > 3 { format!("({f})") } else { f }
                }
            }
        }

        let infos = self.enumerate_ty_bounds(ty);

        let bounds = infos
            .iter()
            .filter_map(|(id, info)| {
                let bound = if info.recursive
                    && let Some(ty) = self.subst.get(id)
                {
                    recurse(self, ty, &infos, 0)
                } else {
                    match self.bounds[*id] {
                        Bound::Numeric(bound) => String::from(bound.as_str()),
                        Bound::Record(ref ty) => recurse_record(self, ty, &infos),
                        Bound::Union(ref ty) => recurse_union(self, ty, &infos),
                        Bound::None => return None,
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

    pub(super) fn enumerate_ty_bounds(&self, ty: &Ty) -> HashMap<Id<Bound>, BoundsInfo> {
        fn recurse_record(
            lowerer: &Lowerer<'_>,
            ty: &RecordTy,
            seen: &mut Vec<Id<Bound>>,
            infos: &mut HashMap<Id<Bound>, BoundsInfo>,
        ) {
            for field in &ty.fields {
                recurse(lowerer, &field.ty, seen, infos);
            }
        }

        fn recurse_union(
            lowerer: &Lowerer<'_>,
            ty: &UnionTy,
            seen: &mut Vec<Id<Bound>>,
            infos: &mut HashMap<Id<Bound>, BoundsInfo>,
        ) {
            for variant in &ty.variants {
                if let Some(ref ty) = variant.payload {
                    recurse(lowerer, ty, seen, infos);
                }
            }
        }

        fn recurse_infer(
            lowerer: &Lowerer<'_>,
            mut id: Id<Bound>,
            seen: &mut Vec<Id<Bound>>,
            infos: &mut HashMap<Id<Bound>, BoundsInfo>,
        ) {
            let n = infos.len();

            while let Some(Ty::Infer(subst)) = lowerer.subst.get(&id) {
                id = *subst;
            }

            if seen.contains(&id) {
                let info = infos.entry(id).or_insert_with(|| BoundsInfo::new(n));
                info.recursive = true;
                return;
            }

            seen.push(id);

            if let Some(ty) = lowerer.subst.get(&id) {
                recurse(lowerer, ty, seen, infos);
                seen.pop();
                return;
            }

            let info = infos.entry(id).or_insert_with(|| BoundsInfo::new(n));
            info.occurences += 1;

            match lowerer.bounds[id] {
                Bound::Numeric(..) => {}
                Bound::Record(ref ty) => recurse_record(lowerer, ty, seen, infos),
                Bound::Union(ref ty) => recurse_union(lowerer, ty, seen, infos),
                Bound::None => {}
            }

            seen.pop();
        }

        fn recurse(
            lowerer: &Lowerer<'_>,
            ty: &Ty,
            seen: &mut Vec<Id<Bound>>,
            infos: &mut HashMap<Id<Bound>, BoundsInfo>,
        ) {
            match ty {
                Ty::Infer(bounds) => recurse_infer(lowerer, *bounds, seen, infos),

                Ty::Numeric(..) | Ty::Generic(..) | Ty::Str | Ty::Error => {}

                Ty::Tuple(fields) => {
                    for field in fields {
                        recurse(lowerer, field, seen, infos);
                    }
                }

                Ty::Lambda(ty) => {
                    recurse(lowerer, &ty.input, seen, infos);
                    recurse(lowerer, &ty.output, seen, infos);
                }

                Ty::Alias(ty) => {
                    for arg in &ty.args {
                        recurse(lowerer, arg, seen, infos);
                    }
                }

                Ty::Record(ty) => recurse_record(lowerer, ty, seen, infos),
                Ty::Union(ty) => recurse_union(lowerer, ty, seen, infos),
                Ty::Monad(ty) => recurse(lowerer, ty, seen, infos),
            }
        }

        let mut seen = Vec::new();
        let mut info = HashMap::new();
        recurse(self, ty, &mut seen, &mut info);
        info
    }
}

pub(super) struct BoundsInfo {
    pub recursive: bool,
    pub occurences: usize,
    pub name: String,
}

impl BoundsInfo {
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
