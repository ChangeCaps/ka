use crate::{
    diagnostic::Span,
    ir::{
        BindPat, Expr, Global, Intrinsic, IntrinsicExpr, LambdaExpr, Pat, ScopeKind, TuplePat, Ty,
        Var, VarExpr, VarKind,
    },
    lower::Lowerer,
};

impl Lowerer<'_> {
    pub(super) fn add_intrinsics(&mut self) {
        let scope = self.add_scope(ScopeKind::Module, None);
        self.imports.insert("intrinsic", scope);

        self.add_hash();

        self.add_string_length();
        self.add_string_format();
        self.add_string_prepend();
        self.add_string_splitat();
        self.add_string_find();
    }

    fn add_hash(&mut self) {
        let input = self.add_inferred_type();

        self.add_intrinsic("hash", Intrinsic::Hash, [input], Ty::NAT);
    }

    fn add_string_length(&mut self) {
        self.add_intrinsic("string-length", Intrinsic::StringLength, [Ty::Str], Ty::NAT);
    }

    fn add_string_format(&mut self) {
        let input = self.add_inferred_type();

        self.add_intrinsic("string-format", Intrinsic::StringFormat, [input], Ty::Str);
    }

    fn add_string_prepend(&mut self) {
        self.add_intrinsic(
            "string-prepend",
            Intrinsic::StringPrepend,
            [Ty::Str, Ty::Str],
            Ty::Str,
        );
    }

    fn add_string_splitat(&mut self) {
        self.add_intrinsic(
            "string-split-at",
            Intrinsic::StringSplitAt,
            [Ty::Str, Ty::NAT],
            Ty::option(Ty::Tuple(vec![Ty::Str, Ty::Str])),
        );
    }

    fn add_string_find(&mut self) {
        self.add_intrinsic(
            "string-find",
            Intrinsic::StringFind,
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

        self.scopes[scope].vars.push(var);
    }
}
