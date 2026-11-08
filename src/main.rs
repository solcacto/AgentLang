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
        "bench" => bench(&args),
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

fn bench(args: &[String]) -> ExitCode {
    use agentlang::bench::{load_tasks, run_suite, NoopRepairer, OracleRepairer, Repairer};
    use std::path::Path;

    let mut dir = String::from("benchmarks");
    let mut max_iters = 5usize;
    let mut use_oracle = true;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--max-iters" => {
                if let Some(value) = args.get(i + 1).and_then(|s| s.parse::<usize>().ok()) {
                    max_iters = value;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--repairer" => {
                if let Some(value) = args.get(i + 1) {
                    use_oracle = value != "none";
                    i += 2;
                } else {
                    i += 1;
                }
            }
            other => {
                dir = other.to_string();
                i += 1;
            }
        }
    }

    let tasks = match load_tasks(Path::new(&dir)) {
        Ok(tasks) => tasks,
        Err(err) => {
            eprintln!("error: could not load benchmarks from `{}`: {}", dir, err);
            return ExitCode::from(2);
        }
    };
    if tasks.is_empty() {
        eprintln!("error: no benchmark tasks found in `{}`", dir);
        return ExitCode::from(2);
    }

    let mut repairer: Box<dyn Repairer> = if use_oracle {
        Box::new(OracleRepairer)
    } else {
        Box::new(NoopRepairer)
    };
    let report = run_suite(&tasks, repairer.as_mut(), max_iters);

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{}", json),
        Err(err) => eprintln!("error: failed to serialize report: {}", err),
    }

    if report.unresolved == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_usage() {
    eprintln!("agent — the AgentLang toolchain");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  agent check <file.ag>   Parse and type-check a source file");
    eprintln!("  agent build <file.ag>   Compile a source file (stub in Phase 1)");
    eprintln!("  agent bench [dir]       Run the self-repair benchmark suite");
    eprintln!("      --repairer oracle|none  Repairer to use (default: oracle)");
    eprintln!("      --max-iters <n>     Max repair rounds per task (default: 5)");
    eprintln!("  agent help              Show this help");
}
