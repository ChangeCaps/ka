use crate::{
    ast::{AliasDef, Def, ExternDef, GlobalDef, ImportDef, ModuleDef, Ty},
    lex::Token,
    parse::{self, Parser},
};

pub fn file(parser: &mut Parser) -> Vec<ModuleDef> {
    let mut defs = Vec::new();

    parser.take_all(Token::Newline);

    while !parser.is(Token::Eof) {
        defs.push(module_def(parser));
        parser.take_all(Token::Newline);
    }

    defs
}

pub fn is_def(token: Token) -> bool {
    matches!(token, Token::Type | Token::Extern | Token::Import)
}

pub fn module_def(parser: &mut Parser) -> ModuleDef {
    match parser.peek() {
        token if is_def(token) => ModuleDef::Def(def(parser)),
        token if parse::is_pat(token) || token == Token::Is => global(parser),

        _ => {
            let span = parser.expected("definition");
            ModuleDef::Def(Def::Error(span))
        }
    }
}

fn global(parser: &mut Parser) -> ModuleDef {
    let ty = is(parser);

    let pat = parse::pat(parser);
    let params = parse::pats(parser);

    let is_bind = parser.take(Token::LeftArrow);

    if !is_bind {
        parser.expect(Token::Eq);
    }

    let expr = parse::expr(parser);
    let span = pat.span();

    ModuleDef::Global(GlobalDef {
        ty,
        pat,
        params,
        expr,
        span,
    })
}

pub fn def(parser: &mut Parser) -> Def {
    match parser.peek() {
        Token::Extern => r#extern(parser),
        Token::Import => import(parser),
        Token::Type => alias(parser),

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

pub fn is(parser: &mut Parser) -> Option<Ty> {
    parser.take(Token::Is).then(|| {
        let ty = parse::ty(parser);

        parser.take_all(Token::Newline);

        ty
    })
}
