use std::{
    fs,
    io::{self, Write},
};

use owo_colors::{AnsiColors, OwoColorize};

use crate::diagnostic::{Diagnostic, Files, Label, Severity};

pub struct Writer<'a, W> {
    writer: W,
    files: &'a Files,
}

impl<'a, W> Writer<'a, W>
where
    W: Write,
{
    pub fn new(writer: W, files: &'a Files) -> Self {
        Self { writer, files }
    }

    pub fn write_report<'b>(
        &mut self,
        diagnostics: impl IntoIterator<Item = &'b Diagnostic>,
    ) -> io::Result<()> {
        let mut errors = 0;
        let mut warnings = 0;

        for diagnostic in diagnostics {
            match diagnostic.severity() {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }

            self.write(diagnostic)?
        }

        if errors > 0 {
            writeln!(
                self.writer,
                "{}{} could to compile program due to {} previous error{}",
                "error".red().bold(),
                ":".bold(),
                errors,
                if errors > 1 { "s" } else { "" },
            )?;
        } else if warnings > 0 {
            writeln!(
                self.writer,
                "{}{} compiled program with {} warning{}",
                "warning".yellow().bold(),
                ":".bold(),
                warnings,
                if warnings > 1 { "s" } else { "" },
            )?;
        }

        Ok(())
    }

    pub fn write(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        let color = match diagnostic.severity() {
            Severity::Error => AnsiColors::Red,
            Severity::Warning => AnsiColors::Yellow,
        };

        self.write_header(diagnostic)?;

        for label in diagnostic.labels() {
            self.write_label(color, label)?;
        }

        if let Some(note) = diagnostic.note() {
            self.write_note(2, color, note)?;
        }

        writeln!(self.writer)?;

        Ok(())
    }

    fn write_header(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        match diagnostic.severity() {
            Severity::Error => write!(self.writer, "{}", "error".red().bold())?,
            Severity::Warning => write!(self.writer, "{}", "warning".yellow().bold())?,
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
                let end = span.start as usize;
                let start = start.min(end);

                column = source[start..end].chars().count() + 1;
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

    fn write_note(&mut self, indent: usize, _color: AnsiColors, note: &str) -> io::Result<()> {
        writeln!(self.writer, "{}{}", " ".repeat(indent), "|".blue().bold())?;

        for (i, line) in note.lines().enumerate() {
            if i == 0 {
                write!(
                    self.writer,
                    "{}{} {} ",
                    " ".repeat(indent),
                    "=".blue().bold(),
                    "note:".bold(),
                )?;
            } else if !line.is_empty() {
                write!(self.writer, "{}{} ", " ".repeat(indent), "~".blue().bold())?;
            } else {
                write!(self.writer, "{}{} ", " ".repeat(indent), "|".blue().bold())?;
            }

            writeln!(self.writer, "{}", line)?;
        }

        Ok(())
    }
}
