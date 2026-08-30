use crate::{
    ast::{
        Arm, BinOp, BinaryExpr, BindStmt, BlockExpr, BlockStmt, CallExpr, ConsExpr, DoExpr, DoKind,
        DoStmt, Expr, ExprField, FieldExpr, LambdaExpr, LetStmt, ListExpr, MatchExpr, NamedExpr,
        NumExpr, ParenExpr, RecordExpr, StrExpr, TupleExpr, UnOp, UnaryExpr, VariantExpr, WithExpr,
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
            | Token::Nat
            | Token::Int
            | Token::Real
            | Token::Bang
            | Token::Do
            | Token::Colon
            | Token::Back
            | Token::OpenParen
            | Token::OpenBrace
            | Token::OpenBracket
    )
}

pub fn expr(parser: &mut Parser) -> Expr {
    match parser.peek() {
        Token::Do => r#do(parser),
        Token::Match => r#match(parser),
        Token::Newline => block(parser),
        Token::Back => lambda(parser),
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

    let mut stmts = Vec::new();

    while is_block_stmt(parser.peek()) {
        let stmt = block_stmt(parser);
        stmts.push(stmt);

        parser.take_all(Token::Newline);
    }

    let expr = expr(parser);
    let expr = Box::new(expr);
    let span = expr.span();

    parser.take_all(Token::Newline);
    parser.expect(Token::Dedent);

    Expr::Block(BlockExpr { stmts, expr, span })
}

fn is_block_stmt(token: Token) -> bool {
    matches!(token, Token::Is | Token::Let) || parse::is_def(token)
}

fn block_stmt(parser: &mut Parser) -> BlockStmt {
    match parser.peek() {
        Token::Is | Token::Let => BlockStmt::Let(let_stmt(parser)),
        _ => BlockStmt::Def(parse::def(parser)),
    }
}

fn let_stmt(parser: &mut Parser) -> LetStmt {
    let ty = parse::is(parser);
    let span = parser.expect(Token::Let);

    let pat = parse::pat(parser);
    let params = parse::pats(parser);

    let is_bind = parser.take(Token::LeftArrow);

    if !is_bind {
        parser.expect(Token::Eq);
    }

    let expr = parse::expr(parser);

    LetStmt {
        ty,
        pat,
        params,
        expr,
        span,
    }
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
    match parser.peek() {
        Token::Is | Token::Let => bind_stmt(parser),
        token if parse::is_def(token) => DoStmt::Def(parse::def(parser)),
        _ => DoStmt::Expr(expr(parser)),
    }
}

fn bind_stmt(parser: &mut Parser) -> DoStmt {
    let ty = parse::is(parser);
    let span = parser.expect(Token::Let);

    let pat = parse::pat(parser);
    let params = parse::pats(parser);

    let is_bind = match parser.peek() {
        Token::Eq => false,
        Token::LeftArrow => true,

        _ => {
            parser.expected("");

            false
        }
    };

    parser.consume();

    let expr = parse::expr(parser);

    match is_bind {
        true => DoStmt::Bind(BindStmt {
            ty,
            pat,
            params,
            expr,
            span,
        }),

        false => DoStmt::Let(LetStmt {
            ty,
            pat,
            params,
            expr,
            span,
        }),
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
    binary(parser, &[(Token::And, BinOp::And)], eq)
}

fn eq(parser: &mut Parser) -> Expr {
    binary(
        parser,
        &[(Token::EqEq, BinOp::Eq), (Token::BangEq, BinOp::Ne)],
        cmp,
    )
}

fn cmp(parser: &mut Parser) -> Expr {
    binary(
        parser,
        &[
            (Token::Gt, BinOp::Gt),
            (Token::Lt, BinOp::Lt),
            (Token::GtEq, BinOp::GtEq),
            (Token::LtEq, BinOp::LtEq),
        ],
        cons,
    )
}

fn cons(parser: &mut Parser) -> Expr {
    let item = add_sub(parser);

    if !parser.take(Token::ColonColon) {
        return item;
    }

    let list = cons(parser);
    let span = item.span().join(list.span());

    let item = Box::new(item);
    let list = Box::new(list);

    Expr::Cons(ConsExpr { item, list, span })
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
        let input = argument(parser);
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

fn argument(parser: &mut Parser) -> Expr {
    match parser.peek() {
        Token::Colon => {
            let span = parser.consume();
            let span = span.join(parser.span());
            let name = parser.expect_ident();
            let expr = None;

            Expr::Variant(VariantExpr { name, expr, span })
        }

        _ => with(parser),
    }
}

fn with(parser: &mut Parser) -> Expr {
    let expr = unary(parser);

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

fn unary(parser: &mut Parser) -> Expr {
    let op = match parser.peek() {
        Token::Nat => UnOp::Nat,
        Token::Int => UnOp::Int,
        Token::Real => UnOp::Real,
        Token::Bang => UnOp::Not,

        _ => return field(parser),
    };

    let span = parser.consume();

    let input = unary(parser);
    let input = Box::new(input);

    let span = span.join(input.span());

    Expr::Unary(UnaryExpr { op, input, span })
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
        Token::OpenBracket => list(parser),

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

    Expr::Named(NamedExpr { name, span })
}

fn string(parser: &mut Parser, string: &'static str) -> Expr {
    let span = parser.consume();

    Expr::Str(StrExpr { string, span })
}

fn list(parser: &mut Parser) -> Expr {
    let start = parser.expect(Token::OpenBracket);

    let mut items = Vec::new();

    if parser.is(Token::Newline) {
        parser.take_all(Token::Newline);
        parser.expect(Token::Indent);
        parser.take_all(Token::Newline);

        while !parser.is(Token::Dedent) && !parser.is(Token::Eof) {
            let item = expr(parser);
            items.push(item);

            parser.take_all(Token::Newline);
        }

        parser.expect(Token::Dedent);
    } else {
        while !parser.is(Token::CloseBracket)
            && !parser.is(Token::Newline)
            && !parser.is(Token::Eof)
        {
            let item = expr(parser);
            items.push(item);

            if !parser.take(Token::Semi) {
                break;
            }
        }
    }

    let end = parser.expect(Token::CloseBracket);

    let span = start.join(end);

    Expr::List(ListExpr { items, span })
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
