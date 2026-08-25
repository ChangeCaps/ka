use std::io::{self, Write};

use crate::{
    arena::Id,
    ir::{Expr, Pat, Program, Value, Var},
};

pub struct Writer<'a, W> {
    program: &'a Program,
    writer: W,
}

impl<'a, W> Writer<'a, W>
where
    W: Write,
{
    pub fn new(writer: W, program: &'a Program) -> Self {
        Self { program, writer }
    }

    pub fn write(&mut self) -> io::Result<()> {
        for global in self.program.globals.values() {
            write!(self.writer, "global ")?;
            self.pat(&global.pat)?;
            write!(self.writer, " =")?;

            self.line(1)?;
            self.expr(1, &global.expr)?;

            writeln!(self.writer)?;
            writeln!(self.writer)?;
        }

        Ok(())
    }

    fn line(&mut self, indent: usize) -> io::Result<()> {
        write!(self.writer, "\n{}", " ".repeat(indent * 2))
    }

    fn expr(&mut self, indent: usize, expr: &Expr) -> io::Result<()> {
        match expr {
            Expr::Value(expr) => match expr.value {
                Value::Num(x) => write!(self.writer, "{x}")?,
                Value::Str(ref s) => {
                    let s = s.replace('\n', "\\n");

                    write!(self.writer, "\"{s}\"")?;
                }
            },

            Expr::Var(expr) => {
                let var = &self.program.vars[expr.var];
                write!(self.writer, "{}", var.name)?;
            }

            Expr::Let(expr) => {
                write!(self.writer, "let ")?;
                self.pat(&expr.pat)?;
                write!(self.writer, " = ")?;
                self.expr(indent, &expr.input)?;
                write!(self.writer, " in")?;
                self.line(indent + 1)?;
                self.expr(indent + 1, &expr.output)?;
            }

            Expr::Bind(expr) => {
                write!(self.writer, "bind ")?;

                self.pat(&expr.pat)?;

                write!(self.writer, " = ")?;

                self.expr(indent, &expr.input)?;

                write!(self.writer, " in ")?;

                self.captures(&self.program.scopes[expr.scope].captures)?;

                self.line(indent + 1)?;
                self.expr(indent + 1, &expr.output)?;
            }

            Expr::Pure(expr) => {
                write!(self.writer, "pure ")?;
                self.expr(indent, &expr.input)?;
            }

            Expr::Call(expr) => {
                write!(self.writer, "(")?;
                self.expr(indent, &expr.lambda)?;
                write!(self.writer, " ")?;
                self.expr(indent, &expr.input)?;
                write!(self.writer, ")")?;
            }

            Expr::With(..) => todo!(),

            Expr::Field(expr) => {
                self.expr(indent, &expr.input)?;
                write!(self.writer, ".{}", expr.name)?;
            }

            Expr::Lambda(expr) => {
                write!(self.writer, "\\")?;
                self.pat(&expr.input)?;
                write!(self.writer, ". ")?;
                self.captures(&self.program.scopes[expr.scope].captures)?;

                self.line(indent + 1)?;
                self.expr(indent + 1, &expr.output)?;
            }

            Expr::Variant(expr) => {
                write!(self.writer, ":{}", expr.name)?;

                if let Some(ref expr) = expr.payload {
                    write!(self.writer, " ")?;
                    self.expr(indent, expr)?;
                }
            }

            Expr::Record(..) => todo!(),

            Expr::Unary(..) => todo!(),

            Expr::Binary(..) => todo!(),

            Expr::Tuple(expr) => {
                write!(self.writer, "(")?;

                for (i, field) in expr.fields.iter().enumerate() {
                    self.expr(indent, field)?;

                    if i < expr.fields.len() - 1 {
                        write!(self.writer, ", ")?;
                    }
                }

                write!(self.writer, ")")?;
            }

            Expr::Match(expr) => {
                write!(self.writer, "match ")?;
                self.expr(indent, &expr.input)?;

                for arm in &expr.arms {
                    self.line(indent + 1)?;
                    self.pat(&arm.pat)?;

                    write!(self.writer, " ->")?;

                    self.line(indent + 2)?;
                    self.expr(indent + 2, &arm.expr)?;
                }
            }

            Expr::Intrinsic(..) => todo!(),

            Expr::Error(..) => write!(self.writer, "{{error}}")?,
        }

        Ok(())
    }

    fn captures(&mut self, captures: &[Id<Var>]) -> io::Result<()> {
        write!(self.writer, "[")?;

        for (i, var) in captures.iter().copied().enumerate() {
            let var = &self.program.vars[var];
            write!(self.writer, "{}", var.name)?;

            if i < captures.len() - 1 {
                write!(self.writer, ", ")?;
            }
        }

        write!(self.writer, "]")?;

        Ok(())
    }

    fn pat(&mut self, pat: &Pat) -> io::Result<()> {
        match pat {
            Pat::Wild(..) => write!(self.writer, "_")?,

            Pat::Bind(pat) => {
                let var = &self.program.vars[pat.var];
                write!(self.writer, "{}", var.name)?;
            }

            Pat::Str(pat) => {
                write!(self.writer, "\"{}\"", pat.string)?;
            }

            Pat::Variant(pat) => {
                write!(self.writer, ":{}", pat.name)?;

                if let Some(ref pat) = pat.payload {
                    write!(self.writer, " ")?;
                    self.pat(pat)?;
                }
            }

            Pat::Tuple(pat) => {
                for (i, field) in pat.fields.iter().enumerate() {
                    self.pat(field)?;

                    if i < pat.fields.len() - 1 {
                        write!(self.writer, ", ")?;
                    }
                }
            }

            Pat::Error(..) => write!(self.writer, "{{error}}")?,
        }

        Ok(())
    }
}
