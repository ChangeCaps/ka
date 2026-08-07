use std::{
    fs,
    io::{self, Write},
};

use owo_colors::{AnsiColors, OwoColorize};

use crate::diagnostic::{Diagnostic, Files, Label, Severity};

pub struct DiagnosticWriter<'a, W> {
    writer: W,
    files: &'a Files,
}

impl<'a, W> DiagnosticWriter<'a, W>
where
    W: Write,
{
    pub fn new(writer: W, files: &'a Files) -> Self {
        Self { writer, files }
    }

    pub fn write(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        let color = match diagnostic.severity() {
            Severity::Error => AnsiColors::Red,
        };

        self.write_header(diagnostic)?;

        for label in diagnostic.labels() {
            self.write_label(color, label)?;
        }

        writeln!(self.writer)?;

        Ok(())
    }

    fn write_header(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        match diagnostic.severity() {
            Severity::Error => write!(self.writer, "{}", "error".red().bold())?,
        }

        writeln!(
            self.writer,
            "{} {}",
            ":".bold(),
            diagnostic.message().bold(),
        )?;

        Ok(())
    }

    fn write_label(&mut self, color: AnsiColors, label: &Label) -> io::Result<()> {
        let span = label.span();

        let file = self.files.get(span.file).unwrap();
        let source = fs::read_to_string(&file.path)?;

        let mut line = 1;
        let mut column = 1;

        let mut start = 0;
        let mut end = 0;

        for l in source.split('\n') {
            end += l.len();

            if end > span.start as usize {
                column = source[start..span.start as usize].chars().count() + 1;
                break;
            }

            line += 1;
            end += '\n'.len_utf8();
            end = end.min(source.len());
            start = end;
        }

        let line_number = line.to_string();
        let line_number_spaces = line_number.len() + 1;

        writeln!(
            self.writer,
            "{}{} {}:{}:{}",
            " ".repeat(line_number_spaces - 1),
            "-->".blue().bold(),
            file.name,
            line,
            column,
        )?;

        writeln!(
            self.writer,
            "{}{}",
            " ".repeat(line_number_spaces),
            "|".blue().bold(),
        )?;

        writeln!(
            self.writer,
            "{} {} {}",
            line_number.blue().bold(),
            "|".blue().bold(),
            &source[start..end],
        )?;

        let highlight_end = end.min(span.end as usize);
        let highlight_count = source[span.start as usize..highlight_end].chars().count();

        writeln!(
            self.writer,
            "{}{}{}{} {}",
            " ".repeat(line_number_spaces),
            "|".blue().bold(),
            " ".repeat(column),
            "^".repeat(highlight_count).color(color).bold(),
            label.message().color(color).bold(),
        )?;

        Ok(())
    }
}
