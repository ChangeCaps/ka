use std::{collections::HashMap, iter};

use crate::{
    diagnostic::Span,
    ir::{Pat, Ty, UnionTy, Variant, lower::Lowerer},
};

impl Lowerer<'_> {
    pub(super) fn match_input_type<'a>(
        &mut self,
        column: &Column,
        pats: impl Iterator<Item = &'a Pat>,
        span: Span,
    ) -> Ty {
        let ty = self.add_inferred_type();

        match column {
            Column::Wild => {
                for pat in pats {
                    self.unify(&ty, &pat.ty(), pat.span());
                }
            }

            Column::Str => {
                for pat in pats {
                    self.unify(&Ty::Str, &pat.ty(), pat.span());
                }

                return Ty::Str;
            }

            Column::List(item, list) => {
                let mut items = Vec::new();
                let mut lists = Vec::new();

                for mut pat in pats {
                    while let Pat::Cons(cons) = pat {
                        items.push(cons.first.as_ref());
                        pat = &cons.rest;
                    }

                    lists.push(pat);
                }

                let item = self.match_input_type(item, items.into_iter(), span);
                self.unify(&ty, &Ty::list(item), span);

                let list = self.match_input_type(list, lists.into_iter(), span);
                self.unify(&ty, &list, span);
            }

            Column::Tuple(columns) => {
                let mut fields = vec![Vec::new(); columns.len()];

                for pat in pats {
                    if let Pat::Tuple(pats) = pat {
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

                let tuple = Ty::Tuple(fields);
                self.unify(&ty, &tuple, span);
            }

            Column::Union(union) => {
                let mut variant_pats: HashMap<&str, Vec<&Pat>> = HashMap::new();

                for pat in pats {
                    if let Pat::Variant(pat) = pat {
                        let pats = variant_pats.entry(pat.name).or_default();

                        if let Some(ref pat) = pat.payload {
                            pats.push(pat);
                        }
                    } else {
                        self.unify(&ty, &pat.ty(), pat.span());
                    }
                }

                let variants = union
                    .variants
                    .iter()
                    .map(|variant| {
                        let payload = variant.payload.as_ref().map(|column| {
                            self.match_input_type(
                                column,
                                variant_pats.remove(variant.name).unwrap().into_iter(),
                                span,
                            )
                        });

                        Variant {
                            name: variant.name,
                            payload,
                        }
                    })
                    .collect::<Vec<_>>();

                if union.is_open {
                    for variant in variants {
                        self.constrain_variant(&ty, variant.name, variant.payload.as_ref(), span);
                    }
                } else {
                    let union = Ty::Union(UnionTy { variants });
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

    pub fn specialize(&self, kind: ConstructorKind) -> Self {
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
            .any(|row| row.first().is_some_and(Pattern::is_wild))
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
                        ConstructorKind::Str(..) => Column::Str,

                        ConstructorKind::Cons => {
                            let item = stack.pop().unwrap();
                            let list = stack.pop().unwrap();

                            Column::List(Box::new(item), Box::new(list))
                        }

                        ConstructorKind::Empty => {
                            Column::List(Box::new(Column::Wild), Box::new(Column::Wild))
                        }

                        ConstructorKind::Tuple(count) => {
                            let fields = stack.drain(stack.len() - count..).rev();
                            Column::Tuple(fields.collect())
                        }

                        ConstructorKind::Variant(name, payload) => {
                            let payload = payload.then(|| stack.pop().unwrap());

                            Column::Union(ColumnUnion {
                                variants: vec![ColumnVariant { name, payload }],
                                is_open: self.is_wild(),
                            })
                        }
                    };

                    stack.push(column);
                    stack
                })
                .reduce(|stack_a, stack_b| {
                    stack_a
                        .into_iter()
                        .zip(stack_b)
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
            return self.rows.iter().all(|row| !row.patterns.is_empty());
        };

        match pat {
            Pattern::Constructor(cons) => {
                let matrix = self.specialize(cons.kind);
                let row = row.specialize(cons.kind).unwrap();
                let columns = columns.specialize(cons.kind);

                matrix.is_useful_recurse(&columns, &row)
            }

            Pattern::Wild => {
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

    pub(super) fn unexhausted_pats(&self, column: &Column) -> Vec<Pattern> {
        let columns = Columns::new(column.clone());

        self.unexhausted_pats_recurse(&columns)
            .into_iter()
            .map(|mut stack| stack.pop().unwrap())
            .collect()
    }

    fn unexhausted_pats_recurse(&self, columns: &Columns) -> Vec<Vec<Pattern>> {
        let Some(column) = columns.first() else {
            return if self.rows.iter().all(|row| !row.patterns.is_empty()) {
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
                        stack.push(Pattern::Wild);
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
                            stack.push(Pattern::Constructor(Constructor { kind, fields }));
                            stack
                        })
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Row {
    patterns: Vec<Pattern>,
}

impl Row {
    pub(super) fn new(pat: Pattern) -> Self {
        Self {
            patterns: vec![pat],
        }
    }

    fn first(&self) -> Option<&Pattern> {
        self.patterns.last()
    }

    fn tail(&self) -> Option<Self> {
        let (_, tail) = self.patterns.split_last()?;
        Some(Self {
            patterns: tail.into(),
        })
    }

    fn specialize(&self, kind: ConstructorKind) -> Option<Self> {
        let mut row = self.clone();

        match self.first()? {
            Pattern::Wild => {
                row.patterns.pop();
                row.patterns
                    .extend(iter::repeat_n(Pattern::Wild, kind.arity()));
            }

            Pattern::Constructor(cons) if cons.kind == kind => {
                row.patterns.pop();
                row.patterns.extend(cons.fields.iter().rev().cloned());
            }

            Pattern::Constructor(..) => return None,
        }

        Some(row)
    }
}

#[derive(Clone, Debug)]
pub(super) enum Pattern {
    Wild,
    Constructor(Constructor),
}

#[derive(Clone, Debug)]
pub(super) struct Constructor {
    kind: ConstructorKind,
    fields: Vec<Pattern>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ConstructorKind {
    Cons,
    Empty,
    Str(&'static str),
    Tuple(usize),
    Variant(&'static str, bool),
}

impl Pattern {
    pub(super) fn new(pat: &Pat) -> Self {
        match pat {
            Pat::Wild(..) | Pat::Bind(..) | Pat::Error(..) => Self::Wild,

            Pat::Str(pat) => Self::Constructor(Constructor {
                kind: ConstructorKind::Str(pat.string),
                fields: Vec::new(),
            }),

            Pat::Cons(pat) => Self::Constructor(Constructor {
                kind: ConstructorKind::Cons,
                fields: vec![Self::new(&pat.first), Self::new(&pat.rest)],
            }),

            Pat::Empty(..) => Self::Constructor(Constructor {
                kind: ConstructorKind::Empty,
                fields: vec![],
            }),

            Pat::Variant(pat) => Self::Constructor(Constructor {
                kind: ConstructorKind::Variant(pat.name, pat.payload.is_some()),
                fields: pat.payload.as_deref().map(Self::new).into_iter().collect(),
            }),

            Pat::Tuple(pat) => Self::Constructor(Constructor {
                kind: ConstructorKind::Tuple(pat.fields.len()),
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
            Pattern::Wild => String::from("_"),
            Pattern::Constructor(cons) => match cons.kind {
                ConstructorKind::Str(s) => format!("\"{s}\""),

                ConstructorKind::Cons => {
                    let first = cons.fields[0].format_recurse(1);
                    let rest = cons.fields[1].format_recurse(1);

                    format!("{first} :: {rest}")
                }

                ConstructorKind::Empty => String::from("[]"),

                ConstructorKind::Tuple(_) => {
                    let f = cons
                        .fields
                        .iter()
                        .map(|pat| pat.format_recurse(1))
                        .collect::<Vec<_>>()
                        .join(", ");

                    if precedence > 0 { format!("({f})") } else { f }
                }

                ConstructorKind::Variant(name, _) => {
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

impl ConstructorKind {
    fn arity(&self) -> usize {
        match self {
            ConstructorKind::Cons => 2,
            ConstructorKind::Empty => 0,
            ConstructorKind::Str(..) => 0,
            ConstructorKind::Tuple(arity) => *arity,
            ConstructorKind::Variant(_, payload) => *payload as usize,
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

    fn specialize(&self, kind: ConstructorKind) -> Self {
        let first = self.first().unwrap();

        let mut columns = self.clone();
        columns.columns.pop();

        match first {
            Column::Wild => {
                columns
                    .columns
                    .extend(iter::repeat_n(Column::Wild, kind.arity()));
            }

            Column::Str => {}

            Column::List(item, list) => match kind {
                ConstructorKind::Cons => {
                    columns.columns.push(list.as_ref().clone());
                    columns.columns.push(item.as_ref().clone());
                }

                ConstructorKind::Empty => {}

                _ => unreachable!(),
            },

            Column::Tuple(fields) => {
                columns.columns.extend(fields.iter().rev().cloned());
            }

            Column::Union(union) => {
                let ConstructorKind::Variant(name, ..) = kind else {
                    panic!();
                };

                if !name.is_empty() {
                    let variant = union
                        .variants
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
    Str,
    List(Box<Column>, Box<Column>),
    Tuple(Vec<Column>),
    Union(ColumnUnion),
}

#[derive(Clone, Debug)]
pub(super) struct ColumnUnion {
    variants: Vec<ColumnVariant>,
    is_open: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ColumnVariant {
    name: &'static str,
    payload: Option<Column>,
}

impl Column {
    pub(super) fn from_pat(pat: &Pattern) -> Self {
        match pat {
            Pattern::Wild => Self::Wild,
            Pattern::Constructor(cons) => Self::from_cons(cons),
        }
    }

    fn from_cons(cons: &Constructor) -> Self {
        match cons.kind {
            ConstructorKind::Cons => {
                let first = Self::from_pat(&cons.fields[0]);
                let rest = Self::from_pat(&cons.fields[1]);

                Self::List(Box::new(first), Box::new(Self::Wild))
                    .merge(&rest)
                    .unwrap()
            }

            ConstructorKind::Empty => Self::List(Box::new(Self::Wild), Box::new(Self::Wild)),

            ConstructorKind::Str(..) => Self::Str,

            ConstructorKind::Tuple(_) => {
                let fields = cons.fields.iter().map(Self::from_pat).collect();
                Self::Tuple(fields)
            }

            ConstructorKind::Variant(name, _) => {
                let payload = cons.fields.first().map(Self::from_pat);
                Self::Union(ColumnUnion {
                    variants: vec![ColumnVariant { name, payload }],
                    is_open: false,
                })
            }
        }
    }

    pub(super) fn merge(&self, other: &Self) -> Result<Self, ()> {
        match (self, other) {
            (_, Self::Wild) => Ok(self.clone()),
            (Self::Wild, _) => Ok(other.clone()),

            (Self::Str, Self::Str) => Ok(self.clone()),

            (Self::List(this_item, this_list), Self::List(other_item, other_list)) => {
                let item = this_item.merge(other_item)?;
                let list = this_list.merge(other_list)?;

                Ok(Self::List(Box::new(item), Box::new(list)))
            }

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

            (Self::Union(this), Self::Union(other)) => {
                let mut variants = this.variants.clone();

                for other_variant in other.variants.iter() {
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

                Ok(Self::Union(ColumnUnion {
                    variants,
                    is_open: this.is_open & other.is_open,
                }))
            }

            (_, _) => Err(()),
        }
    }

    fn constructors(&self) -> Option<Vec<ConstructorKind>> {
        match self {
            Self::Wild | Self::Union(ColumnUnion { is_open: true, .. }) | Self::Str => None,
            Self::List(..) => Some(vec![ConstructorKind::Cons, ConstructorKind::Empty]),
            Self::Tuple(fields) => Some(vec![ConstructorKind::Tuple(fields.len())]),
            Self::Union(union) => {
                let constructors = union
                    .variants
                    .iter()
                    .map(|variant| {
                        ConstructorKind::Variant(variant.name, variant.payload.is_some())
                    })
                    .collect();

                Some(constructors)
            }
        }
    }
}
