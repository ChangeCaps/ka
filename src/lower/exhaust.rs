use std::{collections::HashMap, iter};

use crate::{diagnostic::Span, ir, lower::Lowerer};

impl Lowerer<'_> {
    pub(super) fn match_input_type<'a>(
        &mut self,
        column: &Column,
        pats: impl Iterator<Item = &'a ir::Pat>,
        span: Span,
    ) -> ir::Ty {
        let ty = self.add_inferred_type();

        match column {
            Column::Wild => {
                for pat in pats {
                    self.unify(&ty, &pat.ty(), pat.span())
                }
            }

            Column::Tuple(columns) => {
                let mut fields = vec![Vec::new(); columns.len()];

                for pat in pats {
                    if let ir::Pat::Tuple(pats) = pat {
                        for (field, pat) in fields.iter_mut().zip(&pats.fields) {
                            field.push(pat);
                        }
                    } else {
                        self.unify(&ty, &pat.ty(), pat.span());
                    }
                }

                let fields = fields
                    .into_iter()
                    .zip(columns)
                    .map(|(pats, column)| self.match_input_type(column, pats.into_iter(), span))
                    .collect::<Vec<_>>();

                let tuple = ir::Ty::Tuple(fields);
                self.unify(&ty, &tuple, span);
            }

            Column::Union(variants, is_open) => {
                let mut variant_pats: HashMap<&str, Vec<&ir::Pat>> = HashMap::new();

                for pat in pats {
                    if let ir::Pat::Variant(pat) = pat {
                        let pats = variant_pats.entry(pat.name).or_default();

                        if let Some(ref pat) = pat.pat {
                            pats.push(pat);
                        }
                    } else {
                        self.unify(&ty, &pat.ty(), pat.span());
                    }
                }

                let variants = variants
                    .iter()
                    .map(|variant| {
                        let payload = variant.payload.as_ref().map(|column| {
                            self.match_input_type(
                                column,
                                variant_pats.remove(variant.name).unwrap().into_iter(),
                                span,
                            )
                        });

                        ir::Variant {
                            name: variant.name,
                            ty: payload,
                        }
                    })
                    .collect::<Vec<_>>();

                if *is_open {
                    for variant in variants {
                        self.constrain_tag(&ty, variant.name, variant.ty.as_ref(), span);
                    }
                } else {
                    let union = ir::Ty::Union(ir::UnionTy { variants });
                    self.unify(&ty, &union, span);
                }
            }
        }

        ty
    }
}

#[derive(Clone, Debug)]
pub(super) struct Matrix {
    rows: Vec<Row>,
}

impl Matrix {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn push(&mut self, row: Row) {
        self.rows.push(row);
    }

    pub fn tail(&self) -> Self {
        let rows = self.rows.iter().filter_map(Row::tail).collect();
        Self { rows }
    }

    pub fn specialize(&self, kind: ConsKind) -> Self {
        let rows = self
            .rows
            .iter()
            .filter_map(|row| row.specialize(kind))
            .collect();

        Self { rows }
    }

    pub fn default(&self) -> Self {
        let rows = self
            .rows
            .iter()
            .filter_map(|row| {
                row.first()
                    .is_some_and(|pat| pat.is_wild())
                    .then(|| row.tail())
                    .flatten()
            })
            .collect();

        Self { rows }
    }

    pub fn is_wild(&self) -> bool {
        self.rows
            .iter()
            .any(|row| row.first().is_some_and(Pat::is_wild))
    }

    pub(super) fn open_column(&self, column: Column) -> Column {
        let columns = Columns::new(column);
        let mut columns = self.open_column_recurse(&columns);
        columns.pop().unwrap()
    }

    fn open_column_recurse(&self, columns: &Columns) -> Vec<Column> {
        let Some(column) = columns.first() else {
            return Vec::new();
        };

        match column.constructors() {
            Some(constructors) => constructors
                .into_iter()
                .map(|kind| {
                    let matrix = self.specialize(kind);
                    let columns = columns.specialize(kind);

                    let mut stack = matrix.open_column_recurse(&columns);

                    let column = match kind {
                        ConsKind::Tuple(count) => {
                            let fields = stack.drain(stack.len() - count..).rev();
                            Column::Tuple(fields.collect())
                        }

                        ConsKind::Variant(name, payload) => {
                            let payload = payload.then(|| stack.pop().unwrap());
                            let open = self.is_wild();
                            Column::Union(vec![Variant { name, payload }], open)
                        }
                    };

                    stack.push(column);
                    stack
                })
                .reduce(|columns_a, columns_b| {
                    columns_a
                        .into_iter()
                        .zip(columns_b)
                        .map(|(a, b)| a.merge(&b).unwrap())
                        .collect()
                })
                .unwrap(),

            None => {
                let matrix = self.tail();
                let columns = columns.tail().unwrap();

                let mut stack = matrix.open_column_recurse(&columns);
                stack.push(Column::Wild);
                stack
            }
        }
    }

    pub(super) fn is_useful(&self, column: &Column, row: &Row) -> bool {
        let columns = Columns::new(column.clone());
        self.is_useful_recurse(&columns, row)
    }

    fn is_useful_recurse(&self, columns: &Columns, row: &Row) -> bool {
        let Some(pat) = row.first() else {
            return self.rows.iter().all(|row| !row.pats.is_empty());
        };

        match pat {
            Pat::Cons(cons) => {
                let matrix = self.specialize(cons.kind);
                let row = row.specialize(cons.kind).unwrap();
                let columns = columns.specialize(cons.kind);

                matrix.is_useful_recurse(&columns, &row)
            }

            Pat::Wild => {
                let column = columns.first().unwrap();

                match column.constructors() {
                    None => {
                        let matrix = self.default();
                        let row = row.tail().unwrap();
                        let columns = columns.tail().unwrap();

                        matrix.is_useful_recurse(&columns, &row)
                    }

                    Some(constructors) => constructors.into_iter().any(|kind| {
                        let matrix = self.specialize(kind);
                        let row = row.specialize(kind).unwrap();
                        let columns = columns.specialize(kind);

                        matrix.is_useful_recurse(&columns, &row)
                    }),
                }
            }
        }
    }

    pub(super) fn unexhausted_pats(&self, column: &Column) -> Vec<Pat> {
        let columns = Columns::new(column.clone());

        self.unexhausted_pats_recurse(&columns)
            .into_iter()
            .map(|mut stack| stack.pop().unwrap())
            .collect()
    }

    fn unexhausted_pats_recurse(&self, columns: &Columns) -> Vec<Vec<Pat>> {
        let Some(column) = columns.first() else {
            return if self.rows.iter().all(|row| !row.pats.is_empty()) {
                vec![Vec::new()]
            } else {
                Vec::new()
            };
        };

        match column.constructors() {
            None => {
                let matrix = self.default();
                let columns = columns.tail().unwrap();

                matrix
                    .unexhausted_pats_recurse(&columns)
                    .into_iter()
                    .map(|mut stack| {
                        stack.push(Pat::Wild);
                        stack
                    })
                    .collect()
            }

            Some(constructors) => constructors
                .into_iter()
                .flat_map(|kind| {
                    let matrix = self.specialize(kind);
                    let columns = columns.specialize(kind);

                    matrix
                        .unexhausted_pats_recurse(&columns)
                        .into_iter()
                        .map(move |mut stack| {
                            let fields = stack.drain(stack.len() - kind.arity()..).rev().collect();
                            stack.push(Pat::Cons(Cons { kind, fields }));
                            stack
                        })
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Row {
    pats: Vec<Pat>,
}

impl Row {
    pub(super) fn new(pat: Pat) -> Self {
        Self { pats: vec![pat] }
    }

    fn first(&self) -> Option<&Pat> {
        self.pats.last()
    }

    fn tail(&self) -> Option<Self> {
        let (_, tail) = self.pats.split_last()?;
        Some(Self { pats: tail.into() })
    }

    fn specialize(&self, kind: ConsKind) -> Option<Self> {
        let mut row = self.clone();

        match self.first()? {
            Pat::Wild => {
                row.pats.pop();
                row.pats.extend(iter::repeat_n(Pat::Wild, kind.arity()));
            }

            Pat::Cons(cons) if cons.kind == kind => {
                row.pats.pop();
                row.pats.extend(cons.fields.iter().rev().cloned());
            }

            Pat::Cons(..) => return None,
        }

        Some(row)
    }
}

#[derive(Clone, Debug)]
pub(super) enum Pat {
    Wild,
    Cons(Cons),
}

#[derive(Clone, Debug)]
pub(super) struct Cons {
    kind: ConsKind,
    fields: Vec<Pat>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ConsKind {
    Tuple(usize),
    Variant(&'static str, bool),
}

impl Pat {
    pub(super) fn new(pat: &ir::Pat) -> Self {
        match pat {
            ir::Pat::Wild(..) | ir::Pat::Bind(..) | ir::Pat::Error(..) => Self::Wild,

            ir::Pat::Variant(pat) => Self::Cons(Cons {
                kind: ConsKind::Variant(pat.name, pat.pat.is_some()),
                fields: pat.pat.as_deref().map(Self::new).into_iter().collect(),
            }),

            ir::Pat::Tuple(pat) => Self::Cons(Cons {
                kind: ConsKind::Tuple(pat.fields.len()),
                fields: pat.fields.iter().map(Self::new).collect(),
            }),
        }
    }

    fn is_wild(&self) -> bool {
        matches!(self, Self::Wild)
    }

    pub(super) fn format(&self) -> String {
        self.format_recurse(0)
    }

    fn format_recurse(&self, precedence: u8) -> String {
        match self {
            Pat::Wild => String::from("_"),
            Pat::Cons(cons) => match cons.kind {
                ConsKind::Tuple(_) => {
                    let f = cons
                        .fields
                        .iter()
                        .map(|pat| pat.format_recurse(1))
                        .collect::<Vec<_>>()
                        .join(", ");

                    if precedence > 0 { format!("({f})") } else { f }
                }

                ConsKind::Variant(name, _) => {
                    let mut f = format!(":{name}");

                    if let Some(payload) = cons.fields.first() {
                        f += &format!(" {}", payload.format_recurse(2));
                    }

                    if precedence > 2 { format!("({f})") } else { f }
                }
            },
        }
    }
}

impl ConsKind {
    fn arity(&self) -> usize {
        match self {
            ConsKind::Tuple(arity) => *arity,
            ConsKind::Variant(_, payload) => *payload as usize,
        }
    }
}

#[derive(Clone, Debug)]
struct Columns {
    columns: Vec<Column>,
}

impl Columns {
    fn new(column: Column) -> Self {
        Self {
            columns: vec![column],
        }
    }

    fn first(&self) -> Option<&Column> {
        self.columns.last()
    }

    fn tail(&self) -> Option<Self> {
        let (_, tail) = self.columns.split_last()?;

        Some(Self {
            columns: tail.into(),
        })
    }

    fn specialize(&self, kind: ConsKind) -> Self {
        let first = self.first().unwrap();

        let mut columns = self.clone();
        columns.columns.pop();

        match first {
            Column::Wild => {
                columns
                    .columns
                    .extend(iter::repeat_n(Column::Wild, kind.arity()));
            }

            Column::Tuple(fields) => {
                columns.columns.extend(fields.iter().rev().cloned());
            }

            Column::Union(variants, ..) => {
                let ConsKind::Variant(name, ..) = kind else {
                    panic!();
                };

                if !name.is_empty() {
                    let variant = variants
                        .iter()
                        .find(|variant| variant.name == name)
                        .unwrap();

                    if let Some(ref payload) = variant.payload {
                        columns.columns.push(payload.clone());
                    }
                }
            }
        }

        columns
    }
}

#[derive(Clone, Debug)]
pub(super) enum Column {
    Wild,
    Tuple(Vec<Column>),
    Union(Vec<Variant>, bool),
}

#[derive(Clone, Debug)]
pub(super) struct Variant {
    name: &'static str,
    payload: Option<Column>,
}

impl Column {
    pub(super) fn from_pat(pat: &Pat) -> Self {
        match pat {
            Pat::Wild => Self::Wild,
            Pat::Cons(cons) => Self::from_cons(cons),
        }
    }

    fn from_cons(cons: &Cons) -> Self {
        match cons.kind {
            ConsKind::Tuple(_) => {
                let fields = cons.fields.iter().map(Self::from_pat).collect();
                Self::Tuple(fields)
            }

            ConsKind::Variant(name, _) => {
                let payload = cons.fields.first().map(Self::from_pat);
                Self::Union(vec![Variant { name, payload }], false)
            }
        }
    }

    pub(super) fn merge(&self, other: &Self) -> Result<Self, ()> {
        match (self, other) {
            (_, Self::Wild) => Ok(self.clone()),
            (Self::Wild, _) => Ok(other.clone()),

            (Self::Tuple(this), Self::Tuple(other)) => {
                if this.len() != other.len() {
                    return Err(());
                }

                let fields = this
                    .iter()
                    .cloned()
                    .zip(other)
                    .map(|(this, other)| this.merge(other))
                    .collect::<Result<_, _>>()?;

                Ok(Self::Tuple(fields))
            }

            (Self::Union(variants, is_open), Self::Union(other_variants, other_is_open)) => {
                let mut variants = variants.clone();

                for other_variant in other_variants {
                    let Some(variant) = variants
                        .iter_mut()
                        .find(|variant| variant.name == other_variant.name)
                    else {
                        variants.push(other_variant.clone());
                        continue;
                    };

                    match (variant.payload.as_mut(), other_variant.payload.as_ref()) {
                        (Some(this), Some(other)) => {
                            *this = this.merge(other)?;
                        }

                        (None, None) => {}

                        (_, _) => return Err(()),
                    }
                }

                Ok(Self::Union(variants, is_open & other_is_open))
            }

            (_, _) => Err(()),
        }
    }

    fn constructors(&self) -> Option<Vec<ConsKind>> {
        match self {
            Column::Wild | Column::Union(_, true) => None,
            Column::Tuple(fields) => Some(vec![ConsKind::Tuple(fields.len())]),
            Column::Union(variants, false) => {
                let constructors = variants
                    .iter()
                    .map(|variant| ConsKind::Variant(variant.name, variant.payload.is_some()))
                    .collect();

                Some(constructors)
            }
        }
    }
}
