//! TN619 bootstrap compiler CLI (`tnc`).
//!
//! Usage:
//!   tnc run <file.tn>     Lex, parse, and execute a TN619 program.
//!   tnc tokens <file.tn>  Dump the token stream (proves bilingual lexing).

mod ast;
mod interp;
mod lexer;
mod parser;
mod token;
mod typeck;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (cmd, path) = match args.as_slice() {
        [_, cmd, path] => (cmd.as_str(), path.as_str()),
        [_, path] => ("run", path.as_str()),
        _ => {
            eprintln!("usage: tnc [run|tokens] <file.tn>");
            return ExitCode::FAILURE;
        }
    };

    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", path, e);
            return ExitCode::FAILURE;
        }
    };

    match run(cmd, &src) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(cmd: &str, src: &str) -> Result<(), String> {
    let tokens = lexer::Lexer::new(src).tokenize()?;

    if cmd == "tokens" {
        for t in &tokens {
            println!("{:>4}  {:?}", t.line, t.kind);
        }
        return Ok(());
    }

    let program = parser::Parser::new(tokens).parse_program()?;

    match cmd {
        "check" => {
            typeck::Checker::new().check(&program)?;
            println!("ok: type check passed");
            Ok(())
        }
        "run" => {
            // Static type check (incl. match exhaustiveness) before execution.
            typeck::Checker::new().check(&program)?;
            interp::Interp::new().run(&program)
        }
        other => Err(format!("unknown command '{}'", other)),
    }
}
