use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    Ident(&'static str),
    String(&'static str),
    Number(f64),

    // special
    Invalid(char),
    Whitespace,
    Newline,
    Indent,
    Dedent,
    Eof,

    // keywords
    And,
    As,
    Do,
    Extern,
    Import,
    Int,
    Is,
    Let,
    Match,
    Nat,
    Num,
    Or,
    Str,
    Type,
    With,

    // one-character symbols
    Eq,
    Colon,
    Semi,
    Dot,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Gt,
    Lt,
    Pipe,
    Bang,
    Under,
    Back,
    Quote,

    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,

    // two-character symbols
    ColonColon,
    RightArrow,
    LeftArrow,
    PipeGt,
    EqEq,
    BangEq,
    GtEq,
    LtEq,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident) => write!(f, "{ident}"),
            Self::String(string) => write!(f, "\"{string}\""),
            Self::Number(number) => write!(f, "{number}"),

            // special
            Self::Invalid(c) => write!(f, "`{c}`"),
            Self::Whitespace => write!(f, "whitespace"),
            Self::Newline => write!(f, "newline"),
            Self::Indent => write!(f, "indent"),
            Self::Dedent => write!(f, "dedent"),
            Self::Eof => write!(f, "end of file"),

            // keywords
            Self::And => write!(f, "and"),
            Self::As => write!(f, "as"),
            Self::Do => write!(f, "do"),
            Self::Extern => write!(f, "extern"),
            Self::Import => write!(f, "import"),
            Self::Int => write!(f, "int"),
            Self::Is => write!(f, "is"),
            Self::Let => write!(f, "let"),
            Self::Match => write!(f, "match"),
            Self::Nat => write!(f, "nat"),
            Self::Num => write!(f, "num"),
            Self::Or => write!(f, "or"),
            Self::Str => write!(f, "str"),
            Self::Type => write!(f, "type"),
            Self::With => write!(f, "with"),

            // one-character symbols
            Self::Eq => write!(f, "="),
            Self::Colon => write!(f, ":"),
            Self::Semi => write!(f, ";"),
            Self::Dot => write!(f, "."),
            Self::Comma => write!(f, ","),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Gt => write!(f, ">"),
            Self::Lt => write!(f, "<"),
            Self::Pipe => write!(f, "|"),
            Self::Bang => write!(f, "!"),
            Self::Under => write!(f, "_"),
            Self::Back => write!(f, "\\"),
            Self::Quote => write!(f, "'"),
            Self::OpenParen => write!(f, "("),
            Self::CloseParen => write!(f, ")"),
            Self::OpenBrace => write!(f, "{{"),
            Self::CloseBrace => write!(f, "}}"),
            Self::OpenBracket => write!(f, "["),
            Self::CloseBracket => write!(f, "]"),

            // two-character symbols
            Self::ColonColon => write!(f, "::"),
            Self::RightArrow => write!(f, "->"),
            Self::LeftArrow => write!(f, "<-"),
            Self::PipeGt => write!(f, "|>"),
            Self::EqEq => write!(f, "=="),
            Self::BangEq => write!(f, "!="),
            Self::GtEq => write!(f, ">="),
            Self::LtEq => write!(f, "<="),
        }
    }
}
