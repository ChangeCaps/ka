use crate::{
    ast::{GenericTy, LambdaTy, MonadTy, NamedTy, ParenTy, TupleTy, Ty, UnionTy, Variant},
    lex::Token,
    parse::Parser,
};

pub fn is_ty(token: Token) -> bool {
    matches!(
        token,
        Token::Ident(..)
            | Token::Num
            | Token::Quote
            | Token::Colon
            | Token::Bang
            | Token::OpenParen
    )
}

pub fn ty(parser: &mut Parser) -> Ty {
    tuple(parser)
}

fn tuple(parser: &mut Parser) -> Ty {
    let first = lambda(parser);

    if !parser.is(Token::Comma) {
        return first;
    }

    let mut fields = vec![first];

    while parser.take(Token::Comma) {
        fields.push(lambda(parser));
    }

    Ty::Tuple(TupleTy { fields })
}

fn lambda(parser: &mut Parser) -> Ty {
    let input = union(parser);

    if !parser.take(Token::Arrow) {
        return input;
    }

    let output = lambda(parser);

    Ty::Lambda(LambdaTy {
        input: Box::new(input),
        output: Box::new(output),
    })
}

fn union(parser: &mut Parser) -> Ty {
    if !parser.is(Token::Colon) {
        return named(parser);
    }

    let variants = variants(parser);

    Ty::Union(UnionTy { variants })
}

fn variants(parser: &mut Parser) -> Vec<Variant> {
    let mut variants = Vec::new();

    while parser.is(Token::Colon) {
        variants.push(variant(parser));

        if !parser.take(Token::Pipe) {
            break;
        }
    }

    variants
}

fn variant(parser: &mut Parser) -> Variant {
    parser.expect(Token::Colon);

    let name = parser.expect_ident();
    let ty = is_ty(parser.peek()).then(|| union(parser));

    Variant { name, ty }
}

fn named(parser: &mut Parser) -> Ty {
    let Token::Ident(name) = parser.peek() else {
        return term(parser);
    };

    parser.consume();
    let args = named_args(parser);

    Ty::Named(NamedTy { name, args })
}

fn named_args(parser: &mut Parser) -> Vec<Ty> {
    let mut args = Vec::new();

    while is_ty(parser.peek()) {
        args.push(term(parser));
    }

    args
}

fn term(parser: &mut Parser) -> Ty {
    match parser.peek() {
        Token::Num => num(parser),
        Token::Quote => generic(parser),
        Token::Bang => monad(parser),

        Token::OpenParen => paren(parser),

        _ => {
            let span = parser.expected("type");
            Ty::Error(span)
        }
    }
}

fn paren(parser: &mut Parser) -> Ty {
    parser.expect(Token::OpenParen);

    let ty = ty(parser);
    let ty = Box::new(ty);

    parser.expect(Token::CloseParen);

    Ty::Paren(ParenTy { ty })
}

fn num(parser: &mut Parser) -> Ty {
    parser.expect(Token::Num);

    Ty::Num
}

fn generic(parser: &mut Parser) -> Ty {
    parser.expect(Token::Quote);

    let span = parser.span();
    match parser.expect_ident() {
        Some(name) => Ty::Generic(GenericTy { name }),
        None => Ty::Error(span),
    }
}

fn monad(parser: &mut Parser) -> Ty {
    parser.expect(Token::Bang);

    let ty = union(parser);
    let ty = Box::new(ty);

    Ty::Monad(MonadTy { ty })
}
