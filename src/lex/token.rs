use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    Ident(&'static str),
    Number(f64),

    // special
    Invalid(char),
    Whitespace,
    Newline,
    Indent,
    Dedent,
    Eof,

    // keywords
    Is,
    Let,
    Match,
    Num,
    Type,

    // one-character symbols
    Eq,
    Colon,
    Semi,
    Dot,
    Comma,
    Pipe,
    Bang,
    Back,
    Quote,

    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,

    // two-character symbols
    Arrow,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident) => write!(f, "{ident}"),
            Self::Number(number) => write!(f, "{number}"),

            // special
            Self::Invalid(c) => write!(f, "`{c}`"),
            Self::Whitespace => write!(f, "whitespace"),
            Self::Newline => write!(f, "newline"),
            Self::Indent => write!(f, "indent"),
            Self::Dedent => write!(f, "dedent"),
            Self::Eof => write!(f, "end of file"),

            // keywords
            Self::Is => write!(f, "is"),
            Self::Let => write!(f, "let"),
            Self::Match => write!(f, "match"),
            Self::Num => write!(f, "num"),
            Self::Type => write!(f, "type"),

            // one-character symbols
            Self::Eq => write!(f, "="),
            Self::Colon => write!(f, ":"),
            Self::Semi => write!(f, ";"),
            Self::Dot => write!(f, "."),
            Self::Comma => write!(f, ","),
            Self::Pipe => write!(f, "|"),
            Self::Bang => write!(f, "!"),
            Self::Back => write!(f, "\\"),
            Self::Quote => write!(f, "'"),
            Self::OpenParen => write!(f, "("),
            Self::CloseParen => write!(f, ")"),
            Self::OpenBrace => write!(f, "{{"),
            Self::CloseBrace => write!(f, "}}"),
            Self::OpenBracket => write!(f, "["),
            Self::CloseBracket => write!(f, "]"),

            // two-character symbols
            Self::Arrow => write!(f, "->"),
        }
    }
}
