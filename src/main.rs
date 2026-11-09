//! `agent` command-line interface.
//!
//! Phase 1 exposes `agent check <file>` (parse + type check, JSON diagnostics)
//! and a `build` stub.

use std::env;
use std::fs;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return ExitCode::from(2);
    }

    match args[1].as_str() {
        "check" => check(&args),
        "build" => build(&args),
        "run" => run(&args),
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
    let mut file: Option<&str> = None;
    let mut json = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--format" => match args.get(i + 1).map(|value| value.as_str()) {
                Some("json") => {
                    json = true;
                    i += 2;
                }
                Some("human") | Some("text") => {
                    json = false;
                    i += 2;
                }
                Some(other) => {
                    eprintln!(
                        "error: unknown format `{}` (expected `json` or `human`)",
                        other
                    );
                    return ExitCode::from(2);
                }
                None => {
                    eprintln!("error: `--format` requires a value (`json` or `human`)");
                    return ExitCode::from(2);
                }
            },
            "--json" => {
                json = true;
                i += 1;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown option `{}`", other);
                return ExitCode::from(2);
            }
            other => {
                if file.is_some() {
                    eprintln!("error: unexpected argument `{}`", other);
                    return ExitCode::from(2);
                }
                file = Some(other);
                i += 1;
            }
        }
    }

    let file = match file {
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

    if json {
        // Machine mode: always emit a JSON array (empty on success) to stdout.
        match serde_json::to_string_pretty(&diagnostics) {
            Ok(rendered) => println!("{}", rendered),
            Err(err) => {
                eprintln!("error: failed to serialize diagnostics: {}", err);
                return ExitCode::from(2);
            }
        }
    } else if diagnostics.is_empty() {
        println!("OK: {} passed all checks", file);
    } else {
        match serde_json::to_string_pretty(&diagnostics) {
            Ok(rendered) => println!("{}", rendered),
            Err(err) => eprintln!("error: failed to serialize diagnostics: {}", err),
        }
    }

    if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn build(args: &[String]) -> ExitCode {
    let file = match args.get(2) {
        Some(f) => f.as_str(),
        None => {
            eprintln!("error: `agent build` requires a file path");
            return ExitCode::from(2);
        }
    };

    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: could not read `{}`: {}", file, err);
            return ExitCode::from(2);
        }
    };

    match agentlang::check_and_lower(&source, file) {
        Ok((module, resolved_types)) => {
            let mut codegen =
                agentlang::codegen::Codegen::new().with_resolved_types(resolved_types);
            codegen.lower_module(&module);
            let rust_source = codegen.into_output();

            let out_path = if let Some(stem) = file.strip_suffix(".ag") {
                format!("{}.rs", stem)
            } else {
                format!("{}.rs", file)
            };

            match fs::write(&out_path, &rust_source) {
                Ok(_) => {
                    println!("build: wrote {}", out_path);
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("error: could not write `{}`: {}", out_path, err);
                    ExitCode::from(2)
                }
            }
        }
        Err(diags) => {
            match serde_json::to_string_pretty(&diags) {
                Ok(rendered) => println!("{}", rendered),
                Err(err) => eprintln!("error: failed to serialize diagnostics: {}", err),
            }
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> ExitCode {
    let file = match args.get(2) {
        Some(f) => f.as_str(),
        None => {
            eprintln!("error: `agent run` requires a file path");
            return ExitCode::from(2);
        }
    };

    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: could not read `{}`: {}", file, err);
            return ExitCode::from(2);
        }
    };

    match agentlang::check_and_lower(&source, file) {
        Ok((module, resolved_types)) => {
            let mut codegen =
                agentlang::codegen::Codegen::new().with_resolved_types(resolved_types);
            codegen.lower_module(&module);
            let rust_source = codegen.into_output();

            let out_path = if let Some(stem) = file.strip_suffix(".ag") {
                format!("{}.rs", stem)
            } else {
                format!("{}.rs", file)
            };

            if let Err(err) = fs::write(&out_path, &rust_source) {
                eprintln!("error: could not write `{}`: {}", out_path, err);
                return ExitCode::from(2);
            }

            let exe_path = if let Some(stem) = file.strip_suffix(".ag") {
                format!("./{}.exe", stem)
            } else {
                format!("./{}.exe", file)
            };

            let compile_status = Command::new("rustc")
                .arg(&out_path)
                .arg("-o")
                .arg(&exe_path)
                .status();

            match compile_status {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    eprintln!(
                        "error: rustc compilation failed with exit code {:?}",
                        status.code()
                    );
                    return ExitCode::from(1);
                }
                Err(err) => {
                    eprintln!("error: could not run rustc: {}", err);
                    return ExitCode::from(2);
                }
            }

            let exec_status = Command::new(&exe_path).status();

            match exec_status {
                Ok(status) => match status.code() {
                    Some(code) => {
                        #[cfg(windows)]
                        {
                            return ExitCode::from(code as u8);
                        }
                        #[cfg(not(windows))]
                        {
                            return ExitCode::from(code);
                        }
                    }
                    None => ExitCode::FAILURE,
                },
                Err(err) => {
                    eprintln!("error: could not run `{}`: {}", exe_path, err);
                    ExitCode::from(2)
                }
            }
        }
        Err(diags) => {
            match serde_json::to_string_pretty(&diags) {
                Ok(rendered) => println!("{}", rendered),
                Err(err) => eprintln!("error: failed to serialize diagnostics: {}", err),
            }
            ExitCode::FAILURE
        }
    }
}

fn bench(args: &[String]) -> ExitCode {
    use agentlang::bench::{
        load_tasks, run_suite, CommandRepairer, NoopRepairer, OracleRepairer, Repairer,
    };
    use std::path::Path;

    let mut dir = String::from("benchmarks");
    let mut max_iters = 5usize;
    let mut repairer_kind = String::from("oracle");
    let mut repair_cmd: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--max-iters" => {
                if let Some(value) = args.get(i + 1).and_then(|s| s.parse::<usize>().ok()) {
                    max_iters = value;
                    i += 2;
                } else {
                    eprintln!("error: `--max-iters` requires an integer");
                    return ExitCode::from(2);
                }
            }
            "--repairer" => {
                if let Some(value) = args.get(i + 1) {
                    repairer_kind = value.to_string();
                    i += 2;
                } else {
                    eprintln!("error: `--repairer` requires a value (oracle|none|command)");
                    return ExitCode::from(2);
                }
            }
            "--repair-cmd" => {
                if let Some(value) = args.get(i + 1) {
                    repair_cmd = Some(value.to_string());
                    i += 2;
                } else {
                    eprintln!("error: `--repair-cmd` requires a command string");
                    return ExitCode::from(2);
                }
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown option `{}`", other);
                return ExitCode::from(2);
            }
            other => {
                dir = other.to_string();
                i += 1;
            }
        }
    }

    let mut repairer: Box<dyn Repairer> = match repairer_kind.as_str() {
        "oracle" => Box::new(OracleRepairer),
        "none" => Box::new(NoopRepairer),
        "command" => match &repair_cmd {
            Some(command) if !command.trim().is_empty() => {
                Box::new(CommandRepairer::new(command.clone()))
            }
            _ => {
                eprintln!("error: `--repairer command` requires `--repair-cmd <command>`");
                return ExitCode::from(2);
            }
        },
        other => {
            eprintln!(
                "error: unknown repairer `{}` (expected `oracle`, `none`, or `command`)",
                other
            );
            return ExitCode::from(2);
        }
    };

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

    let report = run_suite(&tasks, repairer.as_mut(), max_iters);

    match serde_json::to_string_pretty(&report) {
        Ok(rendered) => println!("{}", rendered),
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
    eprintln!("      --format json       Always emit a JSON array to stdout ([] on success)");
    eprintln!("      --json              Alias for `--format json`");
    eprintln!("  agent build <file.ag>   Compile a source file to Rust (.rs)");
    eprintln!("  agent run <file.ag>     Compile and execute a source file");
    eprintln!("  agent bench [dir]       Run the self-repair benchmark suite");
    eprintln!("      --repairer oracle|none|command  Repairer to use (default: oracle)");
    eprintln!("      --repair-cmd <cmd>  Shell command for `--repairer command`");
    eprintln!("      --max-iters <n>     Max repair rounds per task (default: 5)");
    eprintln!("  agent help              Show this help");
}
