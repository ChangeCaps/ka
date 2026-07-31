use crate::{
    ast::{
        Arm, BlockExpr, CallExpr, Expr, MatchExpr, NamedExpr, NumExpr, ParenExpr, TagExpr,
        TupleExpr,
    },
    lex::Token,
    parse::{self, Parser},
};

pub fn is_expr(token: Token) -> bool {
    matches!(
        token,
        Token::Ident(..) | Token::Number(..) | Token::Colon | Token::OpenParen
    )
}

pub fn expr(parser: &mut Parser) -> Expr {
    match parser.peek() {
        Token::Match => r#match(parser),
        Token::Newline => block(parser),
        _ => tuple(parser),
    }
}

fn r#match(parser: &mut Parser) -> Expr {
    parser.expect(Token::Match);

    let expr = expr(parser);
    let expr = Box::new(expr);
    let arms = arms(parser);

    Expr::Match(MatchExpr { expr, arms })
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
    parser.expect(Token::Arrow);
    let expr = expr(parser);

    Arm { pat, expr }
}

fn block(parser: &mut Parser) -> Expr {
    parser.take_all(Token::Newline);
    parser.expect(Token::Indent);

    let mut defs = Vec::new();

    while parse::is_def(parser.peek()) {
        parser.take_all(Token::Newline);

        let def = parse::def(parser);
        defs.push(def);
    }

    parser.take_all(Token::Newline);

    let expr = expr(parser);
    let expr = Box::new(expr);

    parser.take_all(Token::Newline);
    parser.expect(Token::Dedent);

    Expr::Block(BlockExpr { defs, expr })
}

fn tuple(parser: &mut Parser) -> Expr {
    let first = tag(parser);

    if !parser.is(Token::Comma) {
        return first;
    }

    let mut fields = vec![first];

    while parser.take(Token::Comma) {
        let field = tag(parser);
        fields.push(field);
    }

    Expr::Tuple(TupleExpr { fields })
}

fn tag(parser: &mut Parser) -> Expr {
    if !parser.take(Token::Colon) {
        return call(parser);
    }

    let name = parser.expect_ident();

    let expr = is_expr(parser.peek()).then(|| call(parser)).map(Box::new);

    Expr::Tag(TagExpr { name, expr })
}

fn call(parser: &mut Parser) -> Expr {
    let mut expr = term(parser);

    while is_expr(parser.peek()) {
        let lambda = Box::new(expr);
        let input = term(parser);
        let input = Box::new(input);

        expr = Expr::Call(CallExpr { lambda, input });
    }

    expr
}

fn term(parser: &mut Parser) -> Expr {
    match parser.peek() {
        Token::OpenParen => paren(parser),
        Token::Number(x) => number(parser, x),
        Token::Ident(name) => named(parser, name),

        _ => {
            let span = parser.expected("expression");
            Expr::Error(span)
        }
    }
}

fn paren(parser: &mut Parser) -> Expr {
    parser.expect(Token::OpenParen);

    let expr = expr(parser);
    let expr = Box::new(expr);

    parser.expect(Token::CloseParen);

    Expr::Paren(ParenExpr { expr })
}

fn number(parser: &mut Parser, number: f64) -> Expr {
    let span = parser.consume();

    Expr::Num(NumExpr { number, span })
}

fn named(parser: &mut Parser, name: &'static str) -> Expr {
    let span = parser.consume();

    Expr::Named(NamedExpr { name, span })
}
