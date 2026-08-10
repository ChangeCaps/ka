use crate::{
    ast::{BindPat, ParenPat, Pat, TuplePat, VariantPat, WildPat},
    diagnostic::Span,
    lex::Token,
    parse::Parser,
};

pub fn is_pat(token: Token) -> bool {
    matches!(
        token,
        Token::Ident(..) | Token::Colon | Token::Under | Token::OpenParen
    )
}

pub fn pats(parser: &mut Parser) -> Vec<Pat> {
    let mut args = Vec::new();

    while is_pat(parser.peek()) {
        args.push(pat(parser));
    }

    args
}

pub fn pat(parser: &mut Parser) -> Pat {
    tuple(parser)
}

fn tuple(parser: &mut Parser) -> Pat {
    let first = term(parser);

    if !parser.is(Token::Comma) {
        return first;
    }

    let mut fields = vec![first];

    while parser.take(Token::Comma) {
        fields.push(term(parser));
    }

    let span = fields.iter().map(Pat::span).reduce(Span::join).unwrap();

    Pat::Tuple(TuplePat { fields, span })
}

fn term(parser: &mut Parser) -> Pat {
    match parser.peek() {
        Token::Ident(name) => bind(parser, name),
        Token::Under => wild(parser),
        Token::Colon => variant(parser),

        Token::OpenParen => paren(parser),

        _ => {
            let span = parser.expected("pattern");
            Pat::Error(span)
        }
    }
}

fn paren(parser: &mut Parser) -> Pat {
    let start = parser.expect(Token::OpenParen);

    let pat = pat(parser);
    let pat = Box::new(pat);

    let end = parser.expect(Token::CloseParen);
    let span = start.join(end);

    Pat::Paren(ParenPat { pat, span })
}

fn wild(parser: &mut Parser) -> Pat {
    let span = parser.expect(Token::Under);

    Pat::Wild(WildPat { span })
}

fn bind(parser: &mut Parser, name: &'static str) -> Pat {
    let span = parser.consume();

    Pat::Bind(BindPat { name, span })
}

fn variant(parser: &mut Parser) -> Pat {
    let start = parser.expect(Token::Colon);
    let mut span = start.join(parser.span());

    let name = parser.expect_ident();
    let pat = is_pat(parser.peek()).then(|| Box::new(term(parser)));

    if let Some(ref pat) = pat {
        span = span.join(pat.span());
    }

    Pat::Variant(VariantPat { name, pat, span })
}
