use std::fmt;

use crate::{
    diagnostic::{Diagnostic, Emitter, Span},
    lex::{Token, Tokens},
};

pub struct Parser<'a> {
    emitter: &'a mut dyn Emitter,
    tokens: &'a Tokens,
    index: usize,
}

impl<'a> Parser<'a> {
    pub fn new(emitter: &'a mut dyn Emitter, tokens: &'a Tokens) -> Self {
        Self {
            emitter,
            tokens,
            index: 0,
        }
    }

    fn get_nth(&self, mut n: usize) -> (Token, Span) {
        let mut index = self.index;

        loop {
            let (token, span) = self.tokens.get(index);
            index += 1;

            if token == Token::Whitespace {
                continue;
            } else if n == 0 {
                break (token, span);
            } else {
                n -= 1;
            }
        }
    }

    pub fn peek_nth(&self, n: usize) -> Token {
        let (token, _span) = self.get_nth(n);
        token
    }

    pub fn peek(&self) -> Token {
        self.peek_nth(0)
    }

    pub fn is(&self, token: Token) -> bool {
        self.peek() == token
    }

    pub fn span_nth(&self, n: usize) -> Span {
        let (_token, span) = self.get_nth(n);
        span
    }

    pub fn span(&self) -> Span {
        self.span_nth(0)
    }

    pub fn consume(&mut self) -> Span {
        let span = self.span();

        while matches!(self.tokens.get(self.index), (Token::Whitespace, _)) {
            self.index += 1;
        }

        if !matches!(self.tokens.get(self.index), (Token::Eof, _)) {
            self.index += 1;
        }

        span
    }

    pub fn take_all(&mut self, token: Token) {
        while self.is(token) {
            self.consume();
        }
    }

    pub fn take(&mut self, expected: Token) -> bool {
        if self.is(expected) {
            self.consume();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, expected: Token) -> Span {
        if self.is(expected) {
            self.consume()
        } else {
            self.expected(expected)
        }
    }

    pub fn expect_ident(&mut self) -> Option<&'static str> {
        match self.peek() {
            Token::Ident(ident) => {
                self.consume();
                Some(ident)
            }

            _ => {
                self.expected("identifier");
                None
            }
        }
    }

    pub fn expected(&mut self, expected: impl fmt::Display) -> Span {
        let actual = self.peek();
        let span = self.consume();

        self.emitter.emit(
            Diagnostic::error(format!("expected `{}` but found `{}`", expected, actual))
                .with_label(span, "here"),
        );

        span
    }
}
