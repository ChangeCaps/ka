use crate::{
    ast::{
        AliasTy, GenericTy, LambdaTy, MonadTy, ParenTy, RecordTy, TupleTy, Ty, TyField, UnionTy,
        Variant,
    },
    diagnostic::Span,
    lex::Token,
    parse::Parser,
};

pub fn is_ty(token: Token) -> bool {
    matches!(
        token,
        Token::Ident(..)
            | Token::Nat
            | Token::Int
            | Token::Real
            | Token::Str
            | Token::Quote
            | Token::Colon
            | Token::Bang
            | Token::OpenParen
            | Token::OpenBrace
    )
}

pub fn ty(parser: &mut Parser) -> Ty {
    match parser.peek() {
        Token::Newline => block_union(parser),
        _ => tuple(parser),
    }
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

fn block_union(parser: &mut Parser) -> Ty {
    parser.take_all(Token::Newline);
    parser.expect(Token::Indent);
    parser.take_all(Token::Newline);

    let mut variants = Vec::new();

    while !parser.is(Token::Dedent) && !parser.is(Token::Eof) {
        parser.expect(Token::Pipe);

        let variant = variant(parser);
        variants.push(variant);

        parser.take_all(Token::Newline);
    }

    parser.expect(Token::Dedent);

    Ty::Union(UnionTy { variants })
}

fn variants(parser: &mut Parser) -> Vec<Variant> {
    let mut variants = Vec::new();

    while parser.is(Token::Colon) {
        let variant = variant(parser);
        variants.push(variant);

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

    Variant {
        name,
        payload: ty,
        span,
    }
}

fn alias(parser: &mut Parser) -> Ty {
    let Some((import, name, span)) = name(parser) else {
        return term(parser);
    };

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
        let arg = match name(parser) {
            Some((import, name, span)) => Ty::Alias(AliasTy {
                import,
                name,
                args: Vec::new(),
                span,
            }),

            None => term(parser),
        };

        args.push(arg);
    }

    args
}

fn name(parser: &mut Parser) -> Option<(Option<&'static str>, &'static str, Span)> {
    let Token::Ident(name) = parser.peek() else {
        return None;
    };

    let span = parser.consume();

    if parser.take(Token::ColonColon) {
        let import = Some(name);
        let span = span.join(parser.span());
        let name = parser.expect_ident()?;

        Some((import, name, span))
    } else {
        Some((None, name, span))
    }
}

fn term(parser: &mut Parser) -> Ty {
    match parser.peek() {
        Token::Nat => nat(parser),
        Token::Int => int(parser),
        Token::Real => real(parser),
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

fn real(parser: &mut Parser) -> Ty {
    parser.expect(Token::Real);
    Ty::Real
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
