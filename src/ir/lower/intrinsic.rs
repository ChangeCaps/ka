use crate::{
    diagnostic::Span,
    ir::{
        BindPat, Bound, Expr, GenericTy, Global, Intrinsic, IntrinsicExpr, LambdaExpr, Pat,
        ScopeKind, TuplePat, Ty, UnionTy, Var, VarExpr, VarKind, Variant, Visible, lower::Lowerer,
    },
};

impl Lowerer<'_> {
    pub(super) fn add_intrinsics(&mut self) {
        let scope = self.add_scope(ScopeKind::Module, None);
        self.imports.insert("intrinsic", scope);

        self.add_dynamic();

        self.add_format_nat();
        self.add_format_int();
        self.add_format_real();

        self.add_hash_str();
        self.add_hash_nat();
        self.add_hash_int();
        self.add_hash_real();

        self.add_nat_xor();

        self.add_string_length();
        self.add_string_prepend();
        self.add_string_splitat();
        self.add_string_find();
    }

    fn add_dynamic(&mut self) {
        const NAT_VARIANT: Variant = Variant {
            name: "nat'",
            payload: Some(Ty::NAT),
        };

        const INT_VARIANT: Variant = Variant {
            name: "int'",
            payload: Some(Ty::INT),
        };

        const REAL_VARIANT: Variant = Variant {
            name: "real'",
            payload: Some(Ty::REAL),
        };

        const STR_VARIANT: Variant = Variant {
            name: "str'",
            payload: Some(Ty::Str),
        };

        const LAMBDA_VARIANT: Variant = Variant {
            name: "lambda",
            payload: None,
        };

        const ACTION_VARIANT: Variant = Variant {
            name: "action",
            payload: None,
        };

        let input = Ty::Generic(GenericTy {
            name: "a",
            bound: self.bounds.add(Bound::None),
        });

        let ty = self.add_inferred_type();
        let tuple = self.add_inferred_type();
        let record = self.add_inferred_type();

        self.unify(
            &record,
            &Ty::Union(UnionTy {
                variants: vec![
                    Variant {
                        name: "none",
                        payload: None,
                    },
                    Variant {
                        name: "some",
                        payload: Some(Ty::Tuple(vec![
                            Ty::Tuple(vec![Ty::Str, ty.clone()]),
                            record.clone(),
                        ])),
                    },
                ],
            }),
            Span::DUMMY,
        );

        self.unify(
            &tuple,
            &Ty::Union(UnionTy {
                variants: vec![
                    Variant {
                        name: "none",
                        payload: None,
                    },
                    Variant {
                        name: "some",
                        payload: Some(Ty::Tuple(vec![ty.clone(), tuple.clone()])),
                    },
                ],
            }),
            Span::DUMMY,
        );

        self.unify(
            &ty,
            &Ty::Union(UnionTy {
                variants: vec![
                    NAT_VARIANT,
                    INT_VARIANT,
                    REAL_VARIANT,
                    STR_VARIANT,
                    LAMBDA_VARIANT,
                    ACTION_VARIANT,
                    Variant {
                        name: "record",
                        payload: Some(record),
                    },
                    Variant {
                        name: "tuple",
                        payload: Some(tuple),
                    },
                    Variant {
                        name: "variant",
                        payload: Some(Ty::Tuple(vec![Ty::Str, ty.clone()])),
                    },
                ],
            }),
            Span::DUMMY,
        );

        self.add_intrinsic("dynamic", Intrinsic::Dynamic, [input], ty);
    }

    fn add_format_nat(&mut self) {
        self.add_intrinsic("format-nat", Intrinsic::FormatNat, [Ty::NAT], Ty::Str);
    }

    fn add_format_int(&mut self) {
        self.add_intrinsic("format-int", Intrinsic::FormatInt, [Ty::INT], Ty::Str);
    }

    fn add_format_real(&mut self) {
        self.add_intrinsic("format-real", Intrinsic::FormatReal, [Ty::REAL], Ty::Str);
    }

    fn add_nat_xor(&mut self) {
        self.add_intrinsic("nat-xor", Intrinsic::NatXor, [Ty::NAT, Ty::NAT], Ty::NAT);
    }

    fn add_hash_str(&mut self) {
        self.add_intrinsic("hash-str", Intrinsic::HashStr, [Ty::Str], Ty::NAT);
    }

    fn add_hash_nat(&mut self) {
        self.add_intrinsic("hash-nat", Intrinsic::HashNat, [Ty::NAT], Ty::NAT);
    }

    fn add_hash_int(&mut self) {
        self.add_intrinsic("hash-int", Intrinsic::HashStr, [Ty::INT], Ty::NAT);
    }

    fn add_hash_real(&mut self) {
        self.add_intrinsic("hash-real", Intrinsic::HashReal, [Ty::REAL], Ty::NAT);
    }

    fn add_string_length(&mut self) {
        self.add_intrinsic("str-length", Intrinsic::StrLength, [Ty::Str], Ty::NAT);
    }

    fn add_string_prepend(&mut self) {
        self.add_intrinsic(
            "str-prepend",
            Intrinsic::StrPrepend,
            [Ty::Str, Ty::Str],
            Ty::Str,
        );
    }

    fn add_string_splitat(&mut self) {
        self.add_intrinsic(
            "str-split-at",
            Intrinsic::StrSplitAt,
            [Ty::Str, Ty::NAT],
            Ty::Tuple(vec![Ty::Str, Ty::Str]),
        );
    }

    fn add_string_find(&mut self) {
        self.add_intrinsic(
            "str-find",
            Intrinsic::StrFind,
            [Ty::Str, Ty::Str],
            Ty::option(Ty::NAT),
        );
    }

    fn add_intrinsic<const N: usize>(
        &mut self,
        name: &'static str,
        intrinsic: Intrinsic,
        inputs: [Ty; N],
        output: Ty,
    ) {
        let scope = self.imports["intrinsic"];

        let input_ty = match N {
            0 => unreachable!(),
            1 => inputs[0].clone(),
            _ => Ty::Tuple(inputs.to_vec()),
        };

        let ty = Ty::lambda(input_ty.clone(), output.clone());

        let mut pats = Vec::new();
        let mut exprs = Vec::new();

        // add the input variables and create input patterns and expressions
        for input in inputs {
            let var = self.vars.add(Var {
                kind: VarKind::Local,
                name: "_",
                ty: input.clone(),
                span: Span::DUMMY,
            });

            let pat = Pat::Bind(BindPat {
                var,
                ty: input.clone(),
                span: Span::DUMMY,
            });

            let expr = Expr::Var(VarExpr {
                var,
                ty: input.clone(),
            });

            pats.push(pat);
            exprs.push(expr);
        }

        let input_pat = match N {
            1 => pats.pop().unwrap(),
            _ => Pat::Tuple(TuplePat {
                fields: pats,
                ty: input_ty,
                span: Span::DUMMY,
            }),
        };

        // create a lambda expression that evaluates the intrinsic on the input
        let expr = Expr::Lambda(LambdaExpr {
            scope: self.add_scope(ScopeKind::Lambda, scope),
            input: input_pat,
            expr: Box::new(Expr::Intrinsic(IntrinsicExpr {
                intrinsic,
                inputs: exprs,
                ty: output,
            })),
            ty: ty.clone(),
        });

        // add the lambda as a global in the intrinsic module
        let global = self.globals.reserve();
        let var = self.vars.add(Var {
            kind: VarKind::Global(global),
            name,
            ty: ty.clone(),
            span: Span::DUMMY,
        });

        let pat = Pat::Bind(BindPat {
            var,
            ty,
            span: Span::DUMMY,
        });

        self.globals.insert(global, Global { pat, expr });
        self.scopes[scope].vars.push(Visible::global(var));
    }
}
