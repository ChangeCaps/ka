use crate::{
    ast::{
        AliasTy, GenericTy, LambdaTy, MonadTy, ParenTy, RecordTy, TupleTy, Ty, TyField, UnionTy,
        Variant,
    },
    lex::Token,
    parse::Parser,
};

pub fn is_ty(token: Token) -> bool {
    matches!(
        token,
        Token::Ident(..)
            | Token::Nat
            | Token::Int
            | Token::Num
            | Token::Str
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

    if !parser.take(Token::RightArrow) {
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
        return alias(parser);
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
    let start = parser.expect(Token::Colon);
    let span = start.join(parser.span());

    let name = parser.expect_ident();
    let ty = is_ty(parser.peek()).then(|| union(parser));

    Variant { name, ty, span }
}

fn alias(parser: &mut Parser) -> Ty {
    let Token::Ident(mut name) = parser.peek() else {
        return term(parser);
    };

    let mut import = None;
    let span = parser.consume();

    if parser.take(Token::ColonColon) {
        import = Some(name);

        let span = span.join(parser.span());
        let Some(actual_name) = parser.expect_ident() else {
            return Ty::Error(span);
        };

        name = actual_name;
    }

    let args = alias_args(parser);

    Ty::Alias(AliasTy {
        import,
        name,
        args,
        span,
    })
}

fn alias_args(parser: &mut Parser) -> Vec<Ty> {
    let mut args = Vec::new();

    while is_ty(parser.peek()) {
        args.push(term(parser));
    }

    args
}

fn term(parser: &mut Parser) -> Ty {
    match parser.peek() {
        Token::Nat => nat(parser),
        Token::Int => int(parser),
        Token::Num => num(parser),
        Token::Str => str(parser),

        Token::Quote => generic(parser),
        Token::Bang => monad(parser),

        Token::OpenParen => paren(parser),
        Token::OpenBrace => record(parser),

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

fn nat(parser: &mut Parser) -> Ty {
    parser.expect(Token::Nat);
    Ty::Nat
}

fn int(parser: &mut Parser) -> Ty {
    parser.expect(Token::Int);
    Ty::Int
}

fn num(parser: &mut Parser) -> Ty {
    parser.expect(Token::Num);
    Ty::Num
}

fn str(parser: &mut Parser) -> Ty {
    parser.expect(Token::Str);
    Ty::Str
}

fn generic(parser: &mut Parser) -> Ty {
    parser.expect(Token::Quote);

    let span = parser.span();
    match parser.expect_ident() {
        Some(name) => Ty::Generic(GenericTy { name, span }),
        None => Ty::Error(span),
    }
}

fn monad(parser: &mut Parser) -> Ty {
    parser.expect(Token::Bang);

    let ty = union(parser);
    let ty = Box::new(ty);

    Ty::Monad(MonadTy { ty })
}

fn record(parser: &mut Parser) -> Ty {
    parser.expect(Token::OpenBrace);

    let mut fields = Vec::new();

    if parser.is(Token::Newline) {
        parser.take_all(Token::Newline);
        parser.expect(Token::Indent);
        parser.take_all(Token::Newline);

        while !parser.is(Token::Dedent) && !parser.is(Token::Eof) {
            let field = field(parser);
            fields.push(field);

            parser.take_all(Token::Newline);
        }

        parser.expect(Token::Dedent);
    } else {
        while !parser.is(Token::CloseBrace) && !parser.is(Token::Eof) {
            let field = field(parser);
            fields.push(field);

            if !parser.take(Token::Semi) {
                break;
            }
        }
    }

    parser.expect(Token::CloseBrace);

    Ty::Record(RecordTy { fields })
}

fn field(parser: &mut Parser) -> TyField {
    let span = parser.span();
    let name = parser.expect_ident();

    parser.expect(Token::Colon);

    let ty = ty(parser);

    TyField { name, ty, span }
}
