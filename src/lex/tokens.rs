use std::sync::Arc;

use crate::{
    diagnostic::{Diagnostic, Emitter, FileId, Span},
    intern::Interner,
    lex::Token,
};

#[derive(Clone, Debug)]
pub struct Tokens {
    tokens: Arc<[(Token, Span)]>,
    file: FileId,
    eof: u32,
}

impl Tokens {
    pub fn lex(
        emitter: &mut dyn Emitter,
        interner: &mut Interner,
        file: FileId,
        input: &str,
    ) -> Self {
        let mut lexer = Lexer::new(emitter, interner, file, input);

        // lex the initial indent
        lexer.advance_newlines();

        // advance the lexer to the end of input
        while lexer.advance() {}

        // finish the lexer, creating the token stream
        lexer.finish()
    }

    pub fn get(&self, index: usize) -> (Token, Span) {
        match self.tokens.get(index) {
            Some(pair) => *pair,
            None => (Token::Eof, self.eof_span()),
        }
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Token, Span)> {
        self.tokens.iter().copied()
    }

    fn eof_span(&self) -> Span {
        Span::new(self.file, self.eof, self.eof)
    }
}

struct Lexer<'a> {
    emitter: &'a mut dyn Emitter,
    interner: &'a mut Interner,

    file: FileId,
    input: &'a str,

    indents: Vec<&'a str>,

    offset: usize,
    tokens: Vec<(Token, Span)>,
}

impl<'a> Lexer<'a> {
    fn new(
        emitter: &'a mut dyn Emitter,
        interner: &'a mut Interner,
        file: FileId,
        input: &'a str,
    ) -> Self {
        Self {
            emitter,
            interner,
            file,
            input,
            indents: Vec::new(),
            offset: 0,
            tokens: Vec::new(),
        }
    }

    fn finish(mut self) -> Tokens {
        let eof = self.input.len() as u32;
        let span = Span::new(self.file, eof, eof);

        self.advance_dedents(span, self.indents.len());

        Tokens {
            tokens: self.tokens.into(),
            file: self.file,
            eof,
        }
    }

    fn remaining(&self) -> &str {
        &self.input[self.offset..]
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.remaining().chars().nth(n)
    }

    fn peek(&self) -> Option<char> {
        self.peek_nth(0)
    }

    fn consume(&mut self) -> usize {
        match self.peek() {
            Some(c) => {
                let len = c.len_utf8();
                self.offset += len;

                len
            }

            None => 0,
        }
    }

    fn consume_n(&mut self, n: usize) -> usize {
        (0..n).map(|_| self.consume()).sum()
    }

    fn consume_while(&mut self, f: impl Fn(char) -> bool) -> &'a str {
        let start = self.offset;
        let mut len = 0;

        while self.peek().is_some_and(&f) {
            len += self.consume();
        }

        &self.input[start..start + len]
    }

    fn spanned<T>(&mut self, mut f: impl FnMut(&mut Self) -> T) -> (T, Span) {
        let start = self.offset;
        let output = f(self);
        let end = self.offset;
        let span = Span::new(self.file, start as u32, end as u32);

        (output, span)
    }

    fn advance(&mut self) -> bool {
        let Some(c) = self.peek() else {
            return false;
        };

        let (Some(token), span) = self.spanned(|l| l.consume_token(c)) else {
            self.advance_invalid(c);
            return true;
        };

        self.tokens.push((token, span));
        self.advance_newlines();

        true
    }

    fn advance_newlines(&mut self) {
        while self.peek().is_some_and(Self::is_newline) {
            let (_, span) = self.spanned(|l| l.consume());
            self.tokens.push((Token::Newline, span));

            self.advance_indents();
        }
    }

    fn advance_invalid(&mut self, c: char) {
        let (_, span) = self.spanned(|l| l.consume());

        self.emitter.emit(
            Diagnostic::error(format!("unexpected character `{c}`")).with_label(span, "found here"),
        );

        self.tokens.push((Token::Invalid(c), span));
    }

    fn advance_indents(&mut self) {
        let (mut indent, span) = self.spanned(|l| l.consume_while(Self::is_indent));

        if self.peek().is_some_and(Self::is_newline) {
            return;
        }

        let mut indents = self.indents.iter();

        loop {
            if indent.is_empty() {
                self.advance_dedents(span, indents.len());
                break;
            }

            let Some(expected) = indents.next() else {
                break;
            };

            let Some(rest) = indent.strip_prefix(expected) else {
                self.emitter
                    .emit(Diagnostic::error("invalid indentation").with_label(span, "found here"));

                return;
            };

            indent = rest;
        }

        if !indent.is_empty() {
            self.indents.push(indent);
            self.tokens.push((Token::Indent, span))
        }
    }

    fn advance_dedents(&mut self, span: Span, n: usize) {
        for _ in 0..n {
            self.tokens.push((Token::Dedent, span));
            self.indents
                .pop()
                .expect("there should never be more dedents than indents");
        }
    }

    fn consume_token(&mut self, c: char) -> Option<Token> {
        if Self::is_whitespace(c) {
            self.consume_whitespace();

            return Some(Token::Whitespace);
        }

        if Self::is_ident_start(c) {
            let token = self.consume_ident_or_keyword();
            return Some(token);
        }

        if Self::is_number_start(c) {
            let token = self.consume_number();
            return Some(token);
        }

        if Self::is_string_delimiter(c) {
            let token = self.consume_string();
            return Some(token);
        }

        if let Some(token) = self.try_consume_symbol(c) {
            return Some(token);
        }

        None
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(Self::is_whitespace);
    }

    fn consume_ident(&mut self) -> &'a str {
        self.consume_while(Self::is_ident_continue)
    }

    fn consume_ident_or_keyword(&mut self) -> Token {
        let ident = self.consume_ident();

        match Self::match_keyword(ident) {
            Some(token) => token,
            None => {
                let ident = self.interner.intern(ident);
                Token::Ident(ident)
            }
        }
    }

    fn consume_number(&mut self) -> Token {
        let number = self.consume_while(|c| c.is_ascii_digit());
        let number = number.parse::<f64>().expect("should parse valid numbers");
        Token::Number(number)
    }

    fn consume_string(&mut self) -> Token {
        self.consume();

        let string = self.consume_while(|c| !Self::is_string_delimiter(c));

        let string = string
            .replace("\\n", "\n")
            .replace("\\r", "\r")
            .replace("\\t", "\t")
            .replace("\\0", "\0")
            .replace("\\\\", "\\")
            .replace("\\\"", "\"");

        let string = self.interner.intern(string);

        self.consume();

        Token::String(string)
    }

    fn try_consume_symbol(&mut self, c: char) -> Option<Token> {
        if let Some(snd) = self.peek_nth(1)
            && let Some(token) = Self::match_two_character_symbol(c, snd)
        {
            self.consume_n(2);
            Some(token)
        } else if let Some(token) = Self::match_one_character_symbol(c) {
            self.consume_n(1);
            Some(token)
        } else {
            None
        }
    }

    fn match_keyword(s: &str) -> Option<Token> {
        Some(match s {
            "as" => Token::As,
            "do" => Token::Do,
            "extern" => Token::Extern,
            "import" => Token::Import,
            "int" => Token::Int,
            "is" => Token::Is,
            "let" => Token::Let,
            "match" => Token::Match,
            "nat" => Token::Nat,
            "num" => Token::Num,
            "str" => Token::Str,
            "type" => Token::Type,
            "_" => Token::Under,
            _ => return None,
        })
    }

    fn match_one_character_symbol(c: char) -> Option<Token> {
        Some(match c {
            '=' => Token::Eq,
            ':' => Token::Colon,
            ';' => Token::Semi,
            '.' => Token::Dot,
            ',' => Token::Comma,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '>' => Token::Gt,
            '<' => Token::Lt,
            '|' => Token::Pipe,
            '!' => Token::Bang,

            '\\' => Token::Back,
            '\'' => Token::Quote,

            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '{' => Token::OpenBrace,
            '}' => Token::CloseBrace,
            '[' => Token::OpenBracket,
            ']' => Token::CloseBracket,

            _ => return None,
        })
    }

    fn match_two_character_symbol(fst: char, snd: char) -> Option<Token> {
        Some(match (fst, snd) {
            (':', ':') => Token::ColonColon,
            ('-', '>') => Token::RightArrow,
            ('<', '-') => Token::LeftArrow,
            ('|', '>') => Token::PipeGt,
            ('=', '=') => Token::EqEq,
            ('!', '=') => Token::BangEq,
            ('>', '=') => Token::GtEq,
            ('<', '=') => Token::LtEq,
            _ => return None,
        })
    }

    fn is_indent(c: char) -> bool {
        c == ' ' || c == '\t'
    }

    fn is_newline(c: char) -> bool {
        c == '\n'
    }

    fn is_whitespace(c: char) -> bool {
        c.is_whitespace() && !Self::is_newline(c)
    }

    fn is_ident_start(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    fn is_ident_continue(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '-' || c == '\''
    }

    fn is_string_delimiter(c: char) -> bool {
        c == '"'
    }

    fn is_number_start(c: char) -> bool {
        c.is_ascii_digit()
    }
}
