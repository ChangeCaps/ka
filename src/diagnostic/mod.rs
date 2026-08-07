mod emitter;
mod files;
mod span;
mod writer;

use std::fmt;

pub use emitter::*;
pub use files::*;
pub use span::*;
pub use writer::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    severity: Severity,
    message: String,
    labels: Vec<Label>,
}

impl Diagnostic {
    pub fn new(severity: Severity, message: impl ToString) -> Self {
        Self {
            severity,
            message: message.to_string(),
            labels: Vec::new(),
        }
    }

    pub fn error(message: impl ToString) -> Self {
        Self::new(Severity::Error, message)
    }

    pub fn with_label(mut self, span: Span, message: impl ToString) -> Self {
        self.labels.push(Label {
            message: message.to_string(),
            span,
        });
        self
    }

    pub fn with_span(self, span: Span) -> Self {
        self.with_label(span, String::new())
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn labels(&self) -> &[Label] {
        &self.labels
    }
}

#[derive(Clone, Debug)]
pub struct Label {
    message: String,
    span: Span,
}

impl Label {
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn span(&self) -> Span {
        self.span
    }
}
