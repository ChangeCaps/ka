use std::fs;

use ka::{
    diagnostic::{DebugEmitter, File},
    intern::Interner,
    lex::{Token, Tokens},
    parse::Parser,
};

fn main() {
    let file = File { index: 0 };
    let input = fs::read_to_string("test.ka").unwrap();

    let mut emitter = DebugEmitter;
    let mut interner = Interner::new();
    let tokens = Tokens::lex(&mut emitter, &mut interner, file, &input);

    println!("tokens:");
    for (token, _) in tokens.iter() {
        if token != Token::Whitespace {
            println!("  {:?}", token);
        }
    }

    let mut parser = Parser::new(&mut emitter, &tokens);
    let ast = ka::parse::file(&mut parser);
    println!("\nast:");
    println!("{:#?}", ast);
}
