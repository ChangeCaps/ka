use std::io::{self, Write};

use crate::mir::{Constant, Constructor, Entry, Expr};

pub fn write(writer: impl Write, entry: &Entry) -> io::Result<()> {
    let mut writer = Writer::new(writer);

    for (i, global) in entry.globals.iter().enumerate() {
        write!(writer.writer, "global{i} = ")?;
        writer.expr(0, global)?;

        writeln!(writer.writer)?;
        writeln!(writer.writer)?;
    }

    writer.expr(0, &entry.output)
}

struct Writer<W> {
    writer: W,
}

impl<W> Writer<W>
where
    W: Write,
{
    fn new(writer: W) -> Self {
        Self { writer }
    }

    fn expr(&mut self, indent: usize, expr: &Expr) -> io::Result<()> {
        match expr {
            Expr::Local(local) => {
                write!(self.writer, "local{}", local.index)?;
            }

            Expr::Global(global) => {
                write!(self.writer, "global{}", global.index)?;
            }

            Expr::Extern(r#extern) => {
                write!(self.writer, "extern[{}]", r#extern.id)?;
            }

            Expr::Constant(constant) => match constant {
                Constant::Nat(x) => write!(self.writer, "{x}t")?,
                Constant::Int(x) => write!(self.writer, "{x}i")?,
                Constant::Real(x) => write!(self.writer, "{x}r")?,

                Constant::Str(s) => {
                    let s = s.replace('\n', "\\n").replace('\t', "\\t");

                    write!(self.writer, "\"{s}\"")?;
                }

                Constant::Bool(b) => write!(self.writer, "{b}")?,
            },

            Expr::Construct(constructor) => match constructor {
                Constructor::Pure(expr) => {
                    write!(self.writer, "pure ")?;
                    self.expr(indent, expr)?;
                }

                Constructor::Tuple(exprs) => {
                    write!(self.writer, "{{")?;

                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            write!(self.writer, ", ")?;
                        }

                        self.expr(indent, expr)?;
                    }

                    write!(self.writer, "}}")?;
                }

                Constructor::Variant(index, expr) => {
                    write!(self.writer, ":{index} ")?;
                    self.expr(indent, expr)?;
                }
            },

            Expr::Let(r#let) => {
                write!(self.writer, "let local{} = ", r#let.local.index)?;
                self.expr(indent, &r#let.input)?;
                write!(self.writer, " in")?;
                self.line(indent + 1)?;
                self.expr(indent + 1, &r#let.output)?;
            }

            Expr::Bind(bind) => {
                write!(self.writer, "bind local{} <- ", bind.local.index)?;
                self.expr(indent, &bind.input)?;
                write!(self.writer, " ")?;
                self.captures(indent, &bind.captures)?;
                self.line(indent + 1)?;
                self.expr(indent + 1, &bind.output)?;
            }

            Expr::Lambda(lambda) => {
                write!(self.writer, "\\local{}. ", lambda.input.index)?;
                self.captures(indent, &lambda.captures)?;
                self.line(indent + 1)?;
                self.expr(indent + 1, &lambda.output)?;
            }

            Expr::Payload(variant) => {
                write!(self.writer, "(*")?;
                self.expr(indent, variant)?;
                write!(self.writer, ")")?;
            }

            Expr::Field(expr, index) => {
                self.expr(indent, expr)?;
                write!(self.writer, ".{index}")?;
            }

            Expr::Call(lambda, input) => {
                write!(self.writer, "(")?;
                self.expr(indent, lambda)?;
                write!(self.writer, " ")?;
                self.expr(indent, input)?;
                write!(self.writer, ")")?;
            }

            Expr::Is(input, variant) => {
                self.expr(indent, input)?;
                write!(self.writer, " is :{variant}")?;
            }

            Expr::If(condition, then, otherwise) => {
                write!(self.writer, "if ")?;
                self.expr(indent, condition)?;
                write!(self.writer, " then ")?;
                self.line(indent + 1)?;
                self.expr(indent + 1, then)?;
                self.line(indent)?;
                write!(self.writer, "else ")?;
                self.line(indent + 1)?;
                self.expr(indent + 1, otherwise)?;
            }

            Expr::Intrinsic(intrinsic, exprs) => {
                write!(self.writer, "({intrinsic:?}")?;

                for expr in exprs.iter() {
                    write!(self.writer, " ")?;
                    self.expr(indent, expr)?;
                }

                write!(self.writer, ")")?;
            }
        }

        Ok(())
    }

    fn line(&mut self, indent: usize) -> io::Result<()> {
        write!(self.writer, "\n{}", " ".repeat(indent * 2))
    }

    fn captures(&mut self, indent: usize, captures: &[Expr]) -> io::Result<()> {
        write!(self.writer, "[")?;

        for (i, expr) in captures.iter().enumerate() {
            if i > 0 {
                write!(self.writer, ", ")?;
            }

            self.expr(indent, expr)?;
        }

        write!(self.writer, "]")?;

        Ok(())
    }
}
