use crate::{
    ast::{
        Arm, BinOp, BinaryExpr, BlockExpr, CallExpr, DoExpr, DoKind, DoStmt, Expr, ExprField,
        FieldExpr, LambdaExpr, MatchExpr, NamedExpr, NumExpr, ParenExpr, RecordExpr, StrExpr,
        TupleExpr, VariantExpr, WithExpr,
    },
    lex::Token,
    parse::{self, Parser},
};

pub fn is_expr(token: Token) -> bool {
    matches!(
        token,
        Token::Ident(..)
            | Token::String(..)
            | Token::Number(..)
            | Token::Do
            | Token::Colon
            | Token::Back
            | Token::OpenParen
            | Token::OpenBrace
    )
}

pub fn expr(parser: &mut Parser) -> Expr {
    match parser.peek() {
        Token::Do => r#do(parser),
        Token::Match => r#match(parser),
        Token::Newline => block(parser),
        _ => pipe(parser),
    }
}

fn r#match(parser: &mut Parser) -> Expr {
    let span = parser.expect(Token::Match);

    let expr = expr(parser);
    let expr = Box::new(expr);
    let arms = arms(parser);

    Expr::Match(MatchExpr { expr, arms, span })
}

fn arms(parser: &mut Parser) -> Vec<Arm> {
    parser.expect(Token::Newline);
    parser.take_all(Token::Newline);

    parser.expect(Token::Indent);
    parser.take_all(Token::Newline);

    let mut arms = Vec::new();

    while !parser.is(Token::Dedent) && !parser.is(Token::Eof) {
        let arm = arm(parser);
        arms.push(arm);

        parser.take_all(Token::Newline);
    }

    parser.expect(Token::Dedent);

    arms
}

fn arm(parser: &mut Parser) -> Arm {
    let pat = parse::pat(parser);
    parser.expect(Token::RightArrow);
    let expr = expr(parser);

    Arm { pat, expr }
}

fn block(parser: &mut Parser) -> Expr {
    parser.expect(Token::Newline);
    parser.take_all(Token::Newline);

    parser.expect(Token::Indent);
    parser.take_all(Token::Newline);

    let mut defs = Vec::new();

    while parse::is_def(parser.peek()) {
        let def = parse::def(parser);
        defs.push(def);

        parser.take_all(Token::Newline);
    }

    let expr = expr(parser);
    let expr = Box::new(expr);
    let span = expr.span();

    parser.take_all(Token::Newline);
    parser.expect(Token::Dedent);

    Expr::Block(BlockExpr { defs, expr, span })
}

fn r#do(parser: &mut Parser) -> Expr {
    let span = parser.expect(Token::Do);

    if !parser.is(Token::Newline) {
        let expr = expr(parser);
        let kind = DoKind::Expr(Box::new(expr));

        return Expr::Do(DoExpr { kind, span });
    }

    parser.expect(Token::Newline);
    parser.take_all(Token::Newline);

    parser.expect(Token::Indent);
    parser.take_all(Token::Newline);

    let mut stmts = Vec::new();

    while !parser.is(Token::Dedent) && !parser.is(Token::Eof) {
        let stmt = stmt(parser);
        stmts.push(stmt);

        parser.take_all(Token::Newline);
    }

    parser.expect(Token::Dedent);

    let kind = DoKind::Block(stmts);
    Expr::Do(DoExpr { kind, span })
}

fn stmt(parser: &mut Parser) -> DoStmt {
    if parse::is_def(parser.peek()) {
        let def = parse::def(parser);
        DoStmt::Def(def)
    } else {
        let expr = expr(parser);
        DoStmt::Expr(expr)
    }
}

fn pipe(parser: &mut Parser) -> Expr {
    let mut input = tuple(parser);
    pipe_line(parser, &mut input);

    if is_pipe_block(parser) {
        pipe_block(parser, &mut input);
    }

    input
}

fn pipe_line(parser: &mut Parser, input: &mut Expr) {
    while parser.is(Token::PipeGt) {
        parser.expect(Token::PipeGt);

        let lambda = call(parser);

        let span = lambda.span().join(input.span());

        *input = Expr::Call(CallExpr {
            lambda: Box::new(lambda),
            input: Box::new(input.clone()),
            span,
        });
    }
}

fn pipe_block(parser: &mut Parser, input: &mut Expr) {
    parser.take_all(Token::Newline);
    parser.expect(Token::Indent);
    parser.take_all(Token::Newline);

    while parser.is(Token::PipeGt) {
        pipe_line(parser, input);
        parser.take_all(Token::Newline);
    }

    parser.expect(Token::Dedent);
}

fn is_pipe_block(parser: &Parser) -> bool {
    let mut n = 0;

    while parser.is_nth(n, Token::Newline) {
        n += 1;
    }

    if !parser.is_nth(n, Token::Indent) {
        return false;
    }

    n += 1;

    while parser.is_nth(n, Token::Newline) {
        n += 1;
    }

    parser.is_nth(n, Token::PipeGt)
}

fn tuple(parser: &mut Parser) -> Expr {
    let first = or(parser);

    if !parser.is(Token::Comma) {
        return first;
    }

    let mut span = first.span();
    let mut fields = vec![first];

    while parser.take(Token::Comma) {
        let field = or(parser);

        span = span.join(field.span());
        fields.push(field);
    }

    Expr::Tuple(TupleExpr { fields, span })
}

fn or(parser: &mut Parser) -> Expr {
    binary(parser, &[(Token::Or, BinOp::Or)], and)
}

fn and(parser: &mut Parser) -> Expr {
    binary(parser, &[(Token::And, BinOp::And)], eq_ne)
}

fn eq_ne(parser: &mut Parser) -> Expr {
    binary(
        parser,
        &[(Token::EqEq, BinOp::Eq), (Token::BangEq, BinOp::Ne)],
        lt_gt,
    )
}

fn lt_gt(parser: &mut Parser) -> Expr {
    binary(
        parser,
        &[
            (Token::Gt, BinOp::Gt),
            (Token::Lt, BinOp::Lt),
            (Token::GtEq, BinOp::GtEq),
            (Token::LtEq, BinOp::LtEq),
        ],
        add_sub,
    )
}

fn add_sub(parser: &mut Parser) -> Expr {
    binary(
        parser,
        &[(Token::Plus, BinOp::Add), (Token::Minus, BinOp::Sub)],
        mul_div,
    )
}

fn mul_div(parser: &mut Parser) -> Expr {
    binary(
        parser,
        &[(Token::Star, BinOp::Mul), (Token::Slash, BinOp::Div)],
        variant,
    )
}

fn binary(parser: &mut Parser, ops: &[(Token, BinOp)], prev: impl Fn(&mut Parser) -> Expr) -> Expr {
    let lhs = prev(parser);

    if let Some((_, op)) = ops.iter().copied().find(|(t, _)| parser.is(*t)) {
        parser.consume();
        let rhs = binary(parser, ops, prev);

        let lhs = Box::new(lhs);
        let rhs = Box::new(rhs);

        let span = lhs.span().join(rhs.span());
        Expr::Binary(BinaryExpr { op, lhs, rhs, span })
    } else {
        lhs
    }
}

fn variant(parser: &mut Parser) -> Expr {
    if !parser.is(Token::Colon) {
        return call(parser);
    }

    let span = parser.consume();
    let name = parser.expect_ident();

    let expr = is_expr(parser.peek()).then(|| call(parser)).map(Box::new);

    Expr::Variant(VariantExpr { name, expr, span })
}

fn call(parser: &mut Parser) -> Expr {
    let mut expr = with(parser);

    while is_expr(parser.peek()) {
        let lambda = Box::new(expr);
        let input = with(parser);
        let input = Box::new(input);

        let span = lambda.span().join(input.span());

        expr = Expr::Call(CallExpr {
            lambda,
            input,
            span,
        });
    }

    expr
}

fn with(parser: &mut Parser) -> Expr {
    let expr = field(parser);

    if !parser.take(Token::With) {
        return expr;
    }

    parser.expect(Token::OpenBrace);
    let fields = fields(parser);
    let end = parser.expect(Token::CloseBrace);

    let input = Box::new(expr);
    let span = input.span().join(end);

    Expr::With(WithExpr {
        input,
        fields,
        span,
    })
}

fn field(parser: &mut Parser) -> Expr {
    let mut expr = term(parser);
    let mut span = expr.span();

    while parser.take(Token::Dot) {
        span = span.join(parser.span());
        let name = parser.expect_ident();

        expr = Expr::Field(FieldExpr {
            input: Box::new(expr),
            name,
            span,
        });
    }

    expr
}

fn term(parser: &mut Parser) -> Expr {
    match parser.peek() {
        Token::Number(x) => number(parser, x),
        Token::String(x) => string(parser, x),
        Token::Ident(name) => named(parser, name),

        Token::Back => lambda(parser),

        Token::OpenParen => paren(parser),
        Token::OpenBrace => record(parser),

        _ => {
            let span = parser.expected("expression");
            Expr::Error(span)
        }
    }
}

fn paren(parser: &mut Parser) -> Expr {
    let start = parser.expect(Token::OpenParen);

    let expr = expr(parser);
    let expr = Box::new(expr);

    let end = parser.expect(Token::CloseParen);
    let span = start.join(end);

    Expr::Paren(ParenExpr { expr, span })
}

fn number(parser: &mut Parser, number: f64) -> Expr {
    let span = parser.consume();

    Expr::Num(NumExpr { number, span })
}

fn named(parser: &mut Parser, name: &'static str) -> Expr {
    let span = parser.consume();

    if parser.take(Token::ColonColon) {
        let import = Some(name);

        let span = span.join(parser.span());
        let Some(name) = parser.expect_ident() else {
            return Expr::Error(span);
        };

        return Expr::Named(NamedExpr { import, name, span });
    }

    let import = None;
    Expr::Named(NamedExpr { import, name, span })
}

fn string(parser: &mut Parser, string: &'static str) -> Expr {
    let span = parser.consume();

    Expr::Str(StrExpr { string, span })
}

fn lambda(parser: &mut Parser) -> Expr {
    let start = parser.expect(Token::Back);

    let mut params = vec![parse::pat(parser)];

    while parse::is_pat(parser.peek()) {
        let pat = parse::pat(parser);
        params.push(pat);
    }

    let end = parser.expect(Token::Dot);

    let expr = expr(parser);
    let expr = Box::new(expr);

    let span = start.join(end);

    Expr::Lambda(LambdaExpr { params, expr, span })
}

fn record(parser: &mut Parser) -> Expr {
    let start = parser.expect(Token::OpenBrace);
    let fields = fields(parser);
    let end = parser.expect(Token::CloseBrace);

    let span = start.join(end);

    Expr::Record(RecordExpr { fields, span })
}

fn fields(parser: &mut Parser) -> Vec<ExprField> {
    let mut fields = Vec::new();

    if parser.is(Token::Newline) {
        parser.take_all(Token::Newline);
        parser.expect(Token::Indent);
        parser.take_all(Token::Newline);

        while !parser.is(Token::Dedent) && !parser.is(Token::Eof) {
            let field = field_init(parser);
            fields.push(field);

            parser.take_all(Token::Newline);
        }

        parser.expect(Token::Dedent);
    } else {
        while !parser.is(Token::CloseBrace) && !parser.is(Token::Eof) {
            let field = field_init(parser);
            fields.push(field);

            if !parser.take(Token::Semi) {
                break;
            }
        }
    }

    fields
}

fn field_init(parser: &mut Parser) -> ExprField {
    let span = parser.span();
    let name = parser.expect_ident();

    parser.expect(Token::Colon);

    let expr = expr(parser);

    ExprField { name, expr, span }
}
