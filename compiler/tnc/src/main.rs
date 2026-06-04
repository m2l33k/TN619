//! TN619 bootstrap compiler CLI (`tnc`).
//!
//! Usage:
//!   tnc run <file.tn>     Lex, parse, type-check, and execute a program.
//!   tnc check <file.tn>   Type-check only.
//!   tnc tokens <file.tn>  Dump the token stream (proves bilingual lexing).
//!   tnc serve [port]      Start the local web playground (default port 8080).

mod ast;
mod interp;
mod lexer;
mod parser;
mod server;
mod token;
mod typeck;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    // `serve [port]` takes no source file.
    if let Some(first) = args.get(1) {
        if first == "serve" {
            let port: u16 = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(8080);
            return match server::serve(port) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {}", e);
                    ExitCode::FAILURE
                }
            };
        }
    }

    let (cmd, path) = match args.as_slice() {
        [_, cmd, path] => (cmd.as_str(), path.as_str()),
        [_, path] => ("run", path.as_str()),
        _ => {
            eprintln!("usage: tnc [run|check|tokens] <file.tn>  |  tnc serve [port]");
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
            let output = interp::Interp::new().run(&program)?;
            print!("{}", output);
            Ok(())
        }
        other => Err(format!("unknown command '{}'", other)),
    }
}

/// Compile + run a source string, returning either captured output or the error
/// message. Shared by the CLI and the web playground.
pub fn compile_and_run(src: &str) -> Result<String, String> {
    let tokens = lexer::Lexer::new(src).tokenize()?;
    let program = parser::Parser::new(tokens).parse_program()?;
    typeck::Checker::new().check(&program)?;
    interp::Interp::new().run(&program)
}
