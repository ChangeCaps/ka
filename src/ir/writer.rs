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
            self.write_pat(&global.pat)?;
            write!(self.writer, " =")?;

            self.write_line(1)?;
            self.write_expr(1, &global.expr)?;

            writeln!(self.writer)?;
            writeln!(self.writer)?;
        }

        Ok(())
    }

    fn write_line(&mut self, indent: usize) -> io::Result<()> {
        write!(self.writer, "\n{}", " ".repeat(indent * 2))
    }

    fn write_expr(&mut self, indent: usize, expr: &Expr) -> io::Result<()> {
        match expr {
            Expr::Value(expr) => match expr.value {
                Value::Num(x) => write!(self.writer, "{x}")?,
                Value::String(ref s) => {
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
                self.write_pat(&expr.pat)?;
                write!(self.writer, " = ")?;
                self.write_expr(indent, &expr.input)?;
                write!(self.writer, " in")?;
                self.write_line(indent + 1)?;
                self.write_expr(indent + 1, &expr.expr)?;
            }

            Expr::Bind(expr) => {
                write!(self.writer, "bind ")?;

                self.write_pat(&expr.pat)?;

                write!(self.writer, " = ")?;

                self.write_expr(indent, &expr.input)?;

                write!(self.writer, " in ")?;

                self.write_captures(&self.program.scopes[expr.scope].caps)?;

                self.write_line(indent + 1)?;
                self.write_expr(indent + 1, &expr.expr)?;
            }

            Expr::Pure(expr) => {
                write!(self.writer, "pure ")?;
                self.write_expr(indent, &expr.expr)?;
            }

            Expr::Call(expr) => {
                write!(self.writer, "(")?;
                self.write_expr(indent, &expr.lambda)?;
                write!(self.writer, " ")?;
                self.write_expr(indent, &expr.input)?;
                write!(self.writer, ")")?;
            }

            Expr::Lambda(expr) => {
                write!(self.writer, "\\")?;
                self.write_pat(&expr.input)?;
                write!(self.writer, ". ")?;
                self.write_captures(&self.program.scopes[expr.scope].caps)?;

                self.write_line(indent + 1)?;
                self.write_expr(indent + 1, &expr.expr)?;
            }

            Expr::Variant(expr) => {
                write!(self.writer, ":{}", expr.name)?;

                if let Some(ref expr) = expr.expr {
                    write!(self.writer, " ")?;
                    self.write_expr(indent, expr)?;
                }
            }

            Expr::Record(..) => todo!(),

            Expr::Binary(..) => todo!(),

            Expr::Tuple(expr) => {
                write!(self.writer, "(")?;

                for (i, field) in expr.fields.iter().enumerate() {
                    self.write_expr(indent, field)?;

                    if i < expr.fields.len() - 1 {
                        write!(self.writer, ", ")?;
                    }
                }

                write!(self.writer, ")")?;
            }

            Expr::Match(expr) => {
                write!(self.writer, "match ")?;
                self.write_expr(indent, &expr.expr)?;

                for arm in &expr.arms {
                    self.write_line(indent + 1)?;
                    self.write_pat(&arm.pat)?;

                    write!(self.writer, " ->")?;

                    self.write_line(indent + 2)?;
                    self.write_expr(indent + 2, &arm.expr)?;
                }
            }

            Expr::Error(..) => write!(self.writer, "{{error}}")?,
        }

        Ok(())
    }

    fn write_captures(&mut self, captures: &[Id<Var>]) -> io::Result<()> {
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

    fn write_pat(&mut self, pat: &Pat) -> io::Result<()> {
        match pat {
            Pat::Wild(..) => write!(self.writer, "_")?,

            Pat::Bind(pat) => {
                let var = &self.program.vars[pat.var];
                write!(self.writer, "{}", var.name)?;
            }

            Pat::Variant(pat) => {
                write!(self.writer, ":{}", pat.name)?;

                if let Some(ref pat) = pat.pat {
                    write!(self.writer, " ")?;
                    self.write_pat(pat)?;
                }
            }

            Pat::Tuple(pat) => {
                for (i, field) in pat.fields.iter().enumerate() {
                    self.write_pat(field)?;

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
