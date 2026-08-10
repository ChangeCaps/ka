use crate::{
    ast::{AliasDef, Def, ExternDef, ImportDef, LetDef, Pat, Ty},
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
    matches!(
        token,
        Token::Is | Token::Let | Token::Type | Token::Extern | Token::Import
    )
}

pub fn def(parser: &mut Parser) -> Def {
    match parser.peek() {
        Token::Extern => r#extern(parser),
        Token::Import => import(parser),
        Token::Type => alias(parser),
        Token::Is => is(parser),
        Token::Let => r#let(parser, None),

        _ => {
            let span = parser.expected("definition");
            Def::Error(span)
        }
    }
}

fn r#extern(parser: &mut Parser) -> Def {
    let span = parser.expect(Token::Extern);

    let Some(id) = parser.expect_string() else {
        return Def::Error(span);
    };

    let Some(name) = parser.expect_ident() else {
        return Def::Error(span);
    };

    parser.expect(Token::Is);

    let ty = parse::ty(parser);

    Def::Extern(ExternDef { id, name, ty, span })
}

fn import(parser: &mut Parser) -> Def {
    parser.expect(Token::Import);

    let span = parser.span();
    let Some(path) = parser.expect_string() else {
        return Def::Error(span);
    };

    let name = parser
        .take(Token::As)
        .then(|| parser.expect_ident())
        .flatten();

    Def::Import(ImportDef { path, name, span })
}

fn alias(parser: &mut Parser) -> Def {
    let span = parser.expect(Token::Type);

    let Some(name) = parser.expect_ident() else {
        return Def::Error(span);
    };

    let params = alias_params(parser);

    parser.expect(Token::Eq);

    let ty = parse::ty(parser);

    Def::Alias(AliasDef {
        name,
        params,
        ty,
        span,
    })
}

fn alias_params(parser: &mut Parser) -> Vec<Option<&'static str>> {
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
    let params = let_params(parser);

    let is_bind = parser.take(Token::LeftArrow);

    if !is_bind {
        parser.expect(Token::Eq);
    }

    let expr = parse::expr(parser);

    Def::Let(LetDef {
        ty,
        pat,
        params,
        is_bind,
        expr,
        span,
    })
}

fn let_params(parser: &mut Parser) -> Vec<Pat> {
    let mut args = Vec::new();

    while parse::is_pat(parser.peek()) {
        args.push(parse::pat(parser));
    }

    args
}
