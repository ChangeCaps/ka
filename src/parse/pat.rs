use crate::{
    ast::{BindPat, ParenPat, Pat, TagPat, TuplePat},
    lex::Token,
    parse::Parser,
};

pub fn is_pat(token: Token) -> bool {
    matches!(token, Token::Ident(..) | Token::Colon | Token::OpenParen)
}

pub fn pat(parser: &mut Parser) -> Pat {
    tuple(parser)
}

fn tuple(parser: &mut Parser) -> Pat {
    let first = term(parser);

    if !parser.is(Token::Comma) {
        return first;
    }

    let mut pats = vec![first];

    while parser.take(Token::Comma) {
        pats.push(term(parser));
    }

    Pat::Tuple(TuplePat { pats })
}

fn term(parser: &mut Parser) -> Pat {
    match parser.peek() {
        Token::Ident(name) => bind(parser, name),
        Token::Colon => tag(parser),

        Token::OpenParen => paren(parser),

        _ => {
            let span = parser.expected("pattern");
            Pat::Error(span)
        }
    }
}

fn paren(parser: &mut Parser) -> Pat {
    parser.expect(Token::OpenParen);

    let pat = pat(parser);
    let pat = Box::new(pat);

    parser.expect(Token::CloseParen);

    Pat::Paren(ParenPat { pat })
}

fn bind(parser: &mut Parser, name: &'static str) -> Pat {
    let span = parser.consume();

    Pat::Bind(BindPat { name, span })
}

fn tag(parser: &mut Parser) -> Pat {
    parser.expect(Token::Colon);

    let name = parser.expect_ident();
    let pat = is_pat(parser.peek()).then(|| Box::new(term(parser)));

    Pat::Tag(TagPat { name, pat })
}
