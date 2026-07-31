use crate::{
    ast::{Def, LetDef, Pat, Ty, TypeDef},
    lex::Token,
    parse::{self, Parser},
};

pub fn file(parser: &mut Parser) -> Vec<Def> {
    let mut defs = Vec::new();

    parser.take_all(Token::Newline);

    while !parser.is(Token::Eof) {
        defs.push(def(parser));
        parser.take_all(Token::Newline);
    }

    defs
}

pub fn is_def(token: Token) -> bool {
    matches!(token, Token::Is | Token::Let | Token::Type)
}

pub fn def(parser: &mut Parser) -> Def {
    match parser.peek() {
        Token::Type => r#type(parser),
        Token::Is => is(parser),
        Token::Let => r#let(parser, None),

        _ => {
            let span = parser.expected("definition");
            Def::Error(span)
        }
    }
}

fn r#type(parser: &mut Parser) -> Def {
    parser.expect(Token::Type);
    let name = parser.expect_ident();
    let args = type_args(parser);

    parser.expect(Token::Eq);

    let ty = parse::ty(parser);

    Def::Type(TypeDef { name, args, ty })
}

fn type_args(parser: &mut Parser) -> Vec<Option<&'static str>> {
    let mut args = Vec::new();

    while parser.take(Token::Quote) {
        args.push(parser.expect_ident());
    }

    args
}

fn is(parser: &mut Parser) -> Def {
    parser.expect(Token::Is);
    let ty = parse::ty(parser);

    parser.take_all(Token::Newline);

    r#let(parser, Some(ty))
}

fn r#let(parser: &mut Parser, ty: Option<Ty>) -> Def {
    let span = parser.expect(Token::Let);

    let pat = parse::pat(parser);
    let args = let_args(parser);

    parser.expect(Token::Eq);

    let expr = parse::expr(parser);

    Def::Let(LetDef {
        ty,
        pat,
        args,
        expr,
        span,
    })
}

fn let_args(parser: &mut Parser) -> Vec<Pat> {
    let mut args = Vec::new();

    while parse::is_pat(parser.peek()) {
        args.push(parse::pat(parser));
    }

    args
}
