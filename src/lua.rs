use std::collections::HashMap;

use crate::{
    arena::Id,
    ir::{BinOp, Expr, Intrinsic, Pat, Program, Value, Var, VarKind},
};

pub fn codegen(program: &Program, main: Id<Var>) -> String {
    let mut codegen = Codegen::new(program);
    let mut output = String::from(include_str!("prelude.lua"));

    for (id, global) in program.globals.iter() {
        let var = format!("global[{}]", id.index());
        codegen.pat(&global.pat, var);
    }

    for (id, global) in program.globals.iter() {
        let expr = codegen.expr(&global.expr);
        output += &format!("global[{}] = {}", id.index(), expr);
        output += " ";
    }

    output += &format!("{}()", codegen.vars[&main]);

    output
}

struct Codegen<'a> {
    program: &'a Program,
    locals: usize,
    vars: HashMap<Id<Var>, String>,
    stmts: Vec<String>,
}

impl<'a> Codegen<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            program,
            locals: 0,
            vars: HashMap::new(),
            stmts: Vec::new(),
        }
    }

    fn add_local(&mut self) -> String {
        let local = format!("local{}", self.locals);
        self.locals += 1;
        local
    }

    fn push_scope(&mut self) -> usize {
        self.stmts.len()
    }

    fn pop_scope(&mut self, len: usize) -> String {
        self.stmts.drain(len..).collect::<Vec<_>>().join(" ")
    }

    fn expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Value(expr) => match expr.value {
                Value::Num(x) => format!("{x}"),
                Value::Str(ref s) => {
                    let s = Self::escape(s);
                    format!("\"{s}\"")
                }
            },

            Expr::Var(expr) => match self.program.vars[expr.var].kind {
                VarKind::Extern(id) => {
                    let id = self.program.externs[id].id;
                    format!("extern['{id}']")
                }

                VarKind::Global(..) | VarKind::Local => self.vars[&expr.var].clone(),
            },

            Expr::Let(expr) => {
                let input = self.expr(&expr.input);

                let local = self.add_local();
                self.stmts.push(format!("local {local} = {input}"));
                self.pat(&expr.pat, local);

                self.expr(&expr.output)
            }

            Expr::Bind(expr) => {
                let input = self.expr(&expr.input);

                let len = self.push_scope();

                let local = self.add_local();
                self.stmts.push(format!("local {local} = {input}"));
                self.pat(&expr.pat, local);

                let output = self.expr(&expr.output);

                let stmts = self.pop_scope(len);

                format!("function() {stmts} return {output} end")
            }

            Expr::Pure(expr) => {
                let input = self.expr(&expr.input);
                format!("function() return {input} end")
            }

            Expr::Call(expr) => {
                let lambda = self.expr(&expr.lambda);
                let input = self.expr(&expr.input);

                format!("({lambda})({input})")
            }

            Expr::With(expr) => {
                let input = self.expr(&expr.input);

                let local = self.add_local();
                self.stmts.push(format!("local {local} = copy({input})"));

                for field in &expr.fields {
                    let expr = self.expr(&field.expr);
                    (self.stmts).push(format!("{local}.{} = {}", field.name, expr));
                }

                local
            }

            Expr::Field(expr) => {
                let input = self.expr(&expr.input);
                format!("{input}.{}", expr.name)
            }

            Expr::Lambda(expr) => {
                let input = self.add_local();
                self.pat(&expr.input, input.clone());

                let len = self.push_scope();
                let output = self.expr(&expr.output);
                let stmts = self.pop_scope(len);

                format!("function({input}) {stmts} return {output} end")
            }

            Expr::Variant(expr) => {
                if expr.name == "true" && expr.payload.is_none() {
                    return String::from("true");
                } else if expr.name == "false" && expr.payload.is_none() {
                    return String::from("false");
                }

                match expr.payload {
                    Some(ref payload) => {
                        let payload = self.expr(payload);

                        format!(
                            "{{ ['variant'] = \"{}\", ['payload'] = {} }}",
                            expr.name, payload,
                        )
                    }

                    None => format!("{{ ['variant'] = \"{}\" }}", expr.name),
                }
            }

            Expr::Record(expr) => {
                let fields = expr
                    .fields
                    .iter()
                    .map(|field| {
                        let expr = self.expr(&field.expr);
                        format!("{} = {}", field.name, expr)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{{ {fields} }}")
            }

            Expr::Unary(expr) => todo!(),

            Expr::Binary(expr) => {
                let lhs = self.expr(&expr.lhs);
                let rhs = self.expr(&expr.rhs);

                match expr.op {
                    BinOp::Add => format!("({lhs} + {rhs})"),
                    BinOp::Sub => format!("({lhs} - {rhs})"),
                    BinOp::Mul => format!("({lhs} * {rhs})"),
                    BinOp::Div => format!("({lhs} / {rhs})"),
                    BinOp::Gt => format!("({lhs} > {rhs})"),
                    BinOp::Lt => format!("({lhs} < {rhs})"),
                    BinOp::Ge => format!("({lhs} >= {rhs})"),
                    BinOp::Le => format!("({lhs} <= {rhs})"),

                    BinOp::And => format!("({lhs} and {rhs})"),
                    BinOp::Or => format!("({lhs} or {rhs})"),

                    BinOp::Eq => format!("eq({lhs}, {rhs})"),
                    BinOp::Ne => format!("ne({lhs}, {rhs})"),
                }
            }

            Expr::Tuple(expr) => {
                let fields = expr
                    .fields
                    .iter()
                    .map(|field| self.expr(field))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{{ {fields} }}")
            }

            Expr::Match(expr) => {
                let input = self.expr(&expr.input);

                let local = self.add_local();
                self.stmts.push(format!("local {local} = {input}"));

                let output = self.add_local();
                self.stmts.push(format!("local {output}"));

                for (i, arm) in expr.arms.iter().enumerate() {
                    self.pat(&arm.pat, local.clone());

                    let len = self.push_scope();
                    let body = self.expr(&arm.expr);
                    self.stmts.push(format!("{output} = {body}"));
                    let body = self.pop_scope(len);

                    if i == 0 {
                        let check = self.check(&arm.pat, &local);
                        self.stmts.push(format!("if {} then {}", check, body));
                    } else if i == expr.arms.len() - 1 {
                        self.stmts.push(format!("else {} end", body));
                    } else {
                        let check = self.check(&arm.pat, &local);
                        self.stmts.push(format!("elseif {} then {}", check, body));
                    }
                }

                output
            }

            Expr::Intrinsic(expr) => {
                let mut inputs = expr.inputs.iter().map(|input| self.expr(input));

                match expr.intrinsic {
                    Intrinsic::Dynamic => {
                        let input = inputs.next().unwrap();
                        format!("dynamic({input})")
                    }

                    Intrinsic::FormatNat | Intrinsic::FormatInt | Intrinsic::FormatReal => {
                        let input = inputs.next().unwrap();
                        format!("tostring({input})")
                    }

                    Intrinsic::HashStr => {
                        let input = inputs.next().unwrap();
                        format!("hashstr({input})")
                    }

                    Intrinsic::HashNat | Intrinsic::HashInt | Intrinsic::HashReal => {
                        let input = inputs.next().unwrap();
                        format!("hashnum({input})")
                    }

                    Intrinsic::NatXor => {
                        let lhs = inputs.next().unwrap();
                        let rhs = inputs.next().unwrap();

                        format!("{lhs} ^ {rhs}")
                    }

                    Intrinsic::StrLength => {
                        let input = inputs.next().unwrap();

                        format!("strlength({input})")
                    }

                    Intrinsic::StrPrepend => {
                        let a = inputs.next().unwrap();
                        let b = inputs.next().unwrap();

                        format!("{a} .. {b}")
                    }

                    Intrinsic::StrSplitAt => {
                        let s = inputs.next().unwrap();
                        let i = inputs.next().unwrap();

                        format!("strsplitat({s}, {i})")
                    }

                    Intrinsic::StrFind => {
                        let haystack = inputs.next().unwrap();
                        let needle = inputs.next().unwrap();

                        format!("strfind({haystack}, {needle})")
                    }
                }
            }

            Expr::Error(..) => panic!(),
        }
    }

    fn pat(&mut self, pat: &Pat, input: String) {
        match pat {
            Pat::Wild(..) | Pat::Str(..) => {}

            Pat::Bind(pat) => {
                self.vars.insert(pat.var, input);
            }

            Pat::Variant(pat) => {
                if let Some(ref payload) = pat.payload {
                    let input = format!("{input}['payload']");
                    self.pat(payload, input);
                }
            }

            Pat::Tuple(pat) => {
                for (i, field) in pat.fields.iter().enumerate() {
                    let input = format!("{input}[{}]", i + 1);
                    self.pat(field, input);
                }
            }

            Pat::Error(..) => panic!(),
        }
    }

    fn check(&mut self, pat: &Pat, input: &str) -> String {
        match pat {
            Pat::Wild(..) | Pat::Bind(..) => String::from("true"),

            Pat::Str(pat) => {
                let s = Self::escape(pat.string);
                format!("{} == \"{}\"", input, s)
            }

            Pat::Variant(pat) => {
                if pat.name == "true" && pat.payload.is_none() {
                    return input.into();
                } else if pat.name == "false" && pat.payload.is_none() {
                    return format!("not {input}");
                }

                match pat.payload {
                    Some(ref payload) => {
                        let payload = self.check(payload, &format!("{input}['payload']"));
                        format!("{}['variant'] == \"{}\" and {}", input, pat.name, payload)
                    }

                    None => format!("{}['variant'] == \"{}\"", input, pat.name),
                }
            }

            Pat::Tuple(pat) => pat
                .fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let input = format!("{input}[{}]", i + 1);
                    self.check(field, &input)
                })
                .collect::<Vec<_>>()
                .join(" and "),

            Pat::Error(..) => panic!(),
        }
    }

    fn escape(s: &str) -> String {
        s.replace("\n", "\\n")
            .replace("\t", "\\t")
            .replace("\r", "\\r")
            .replace("\0", "\\0")
            .replace("\"", "\\\"")
            .replace("\\", "\\\\")
    }
}
