//! `agent` command-line interface.
//!
//! Phase 1 exposes `agent check <file>` (parse + type check, JSON diagnostics)
//! and a `build` stub.

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return ExitCode::from(2);
    }

    match args[1].as_str() {
        "check" => check(&args),
        "build" => build(&args),
        "-h" | "--help" | "help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("error: unknown command `{}`", other);
            print_usage();
            ExitCode::from(2)
        }
    }
}

fn check(args: &[String]) -> ExitCode {
    let file = match args.get(2) {
        Some(file) => file,
        None => {
            eprintln!("error: `agent check` requires a file path");
            return ExitCode::from(2);
        }
    };

    let source = match fs::read_to_string(file) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("error: could not read `{}`: {}", file, err);
            return ExitCode::from(2);
        }
    };

    let diagnostics = agentlang::check_source(&source, file);
    if diagnostics.is_empty() {
        println!("OK: {} passed all checks", file);
        ExitCode::SUCCESS
    } else {
        match serde_json::to_string_pretty(&diagnostics) {
            Ok(json) => println!("{}", json),
            Err(err) => eprintln!("error: failed to serialize diagnostics: {}", err),
        }
        ExitCode::FAILURE
    }
}

fn build(args: &[String]) -> ExitCode {
    let file = args.get(2).map(|s| s.as_str()).unwrap_or("<none>");
    println!(
        "build: not yet implemented (Phase 1 stub). Target: {}",
        file
    );
    ExitCode::SUCCESS
}

fn print_usage() {
    eprintln!("agent — the AgentLang toolchain");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  agent check <file.ag>   Parse and type-check a source file");
    eprintln!("  agent build <file.ag>   Compile a source file (stub in Phase 1)");
    eprintln!("  agent help              Show this help");
}
