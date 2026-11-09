//! Benchmark harness for the agentic self-repair loop.
//!
//! The idea: a task is a *broken* `.ag` program plus a known-good version. The
//! harness runs `check`, feeds the structured diagnostics to a [`Repairer`], and
//! repeats until the program is clean (or the iteration budget runs out).
//!
//! A real LLM plugs in either by implementing [`Repairer`] directly or, more
//! portably, by wrapping any agent CLI with [`CommandRepairer`]. The built-in
//! [`OracleRepairer`] returns the known-good source and exists to validate the
//! loop and record an ideal-case baseline.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::check_source;
use crate::diagnostic::Diagnostic;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A benchmark task: a broken source and the known-good source.
pub struct Task {
    pub name: String,
    pub broken: String,
    pub fixed: String,
}

/// Something that proposes repaired source given the current diagnostics.
///
/// An LLM adapter implements this by sending `source` + `diags` to a model and
/// returning the model's rewritten program (or `None` to give up).
pub trait Repairer {
    fn repair(&mut self, task: &Task, source: &str, diags: &[Diagnostic]) -> Option<String>;

    /// Short identifier recorded in the benchmark report metadata.
    fn name(&self) -> &'static str {
        "custom"
    }
}

/// Baseline repairer: returns the known-good source immediately.
pub struct OracleRepairer;

impl Repairer for OracleRepairer {
    fn repair(&mut self, task: &Task, _source: &str, _diags: &[Diagnostic]) -> Option<String> {
        Some(task.fixed.clone())
    }

    fn name(&self) -> &'static str {
        "oracle"
    }
}

/// A repairer that never repairs — measures first-pass behavior only.
pub struct NoopRepairer;

impl Repairer for NoopRepairer {
    fn repair(&mut self, _task: &Task, _source: &str, _diags: &[Diagnostic]) -> Option<String> {
        None
    }

    fn name(&self) -> &'static str {
        "none"
    }
}

/// Drive an external command as the repairer.
///
/// Each call creates a temporary directory containing:
///
/// - `source.ag` — the current (broken) program, which the command may rewrite
///   in place.
/// - `diagnostics.json` — the current diagnostics as JSON.
///
/// The command is run through the platform shell (`cmd /C` on Windows, `sh -c`
/// elsewhere). The following placeholders are substituted in the command string:
/// `{dir}`, `{source}`, `{diagnostics}`, `{task}`. The same paths are also
/// exposed as environment variables (`AGENTLANG_DIR`, `AGENTLANG_SOURCE_FILE`,
/// `AGENTLANG_DIAGNOSTICS_FILE`, `AGENTLANG_TASK`), which avoids quoting issues
/// on paths containing spaces.
///
/// Whatever the command leaves in `source.ag` is read back. Returning `None`
/// (unchanged source, failed spawn, or non-zero exit) ends the repair loop for
/// that task.
pub struct CommandRepairer {
    command: String,
}

impl CommandRepairer {
    pub fn new(command: impl Into<String>) -> Self {
        CommandRepairer {
            command: command.into(),
        }
    }

    pub fn command(&self) -> &str {
        &self.command
    }
}

impl Repairer for CommandRepairer {
    fn repair(&mut self, task: &Task, source: &str, diags: &[Diagnostic]) -> Option<String> {
        let dir = temp_dir_for(task);
        if let Err(err) = fs::create_dir_all(&dir) {
            eprintln!(
                "warning: could not create repair directory `{}`: {}",
                dir.display(),
                err
            );
            return None;
        }

        let source_path = dir.join("source.ag");
        let diagnostics_path = dir.join("diagnostics.json");

        let result = self.run(task, source, diags, &dir, &source_path, &diagnostics_path);
        if let Err(err) = fs::remove_dir_all(&dir) {
            eprintln!(
                "warning: could not remove repair directory `{}`: {}",
                dir.display(),
                err
            );
        }

        match result {
            Some(next) if next != source => Some(next),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        "command"
    }
}

impl CommandRepairer {
    fn run(
        &self,
        task: &Task,
        source: &str,
        diags: &[Diagnostic],
        dir: &Path,
        source_path: &Path,
        diagnostics_path: &Path,
    ) -> Option<String> {
        fs::write(source_path, source).ok()?;
        let diagnostics_json =
            serde_json::to_string_pretty(diags).unwrap_or_else(|_| "[]".to_string());
        fs::write(diagnostics_path, diagnostics_json).ok()?;

        let command = expand_command(
            &self.command,
            dir,
            source_path,
            diagnostics_path,
            &task.name,
        );

        let output = shell_command(&command)
            .env("AGENTLANG_TASK", &task.name)
            .env("AGENTLANG_DIR", dir)
            .env("AGENTLANG_SOURCE_FILE", source_path)
            .env("AGENTLANG_DIAGNOSTICS_FILE", diagnostics_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!(
                    "warning: repair command for `{}` exited with {}: {}",
                    task.name,
                    output.status,
                    stderr.trim()
                );
                return None;
            }
            Err(err) => {
                eprintln!(
                    "warning: could not run repair command for `{}`: {}",
                    task.name, err
                );
                return None;
            }
        }

        fs::read_to_string(source_path).ok()
    }
}

fn temp_dir_for(task: &Task) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let safe_name: String = task
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    std::env::temp_dir().join(format!(
        "agentlang-bench-{}-{}-{}",
        std::process::id(),
        safe_name,
        id
    ))
}

fn expand_command(
    template: &str,
    dir: &Path,
    source_path: &Path,
    diagnostics_path: &Path,
    task: &str,
) -> String {
    template
        .replace("{dir}", &dir.display().to_string())
        .replace("{source}", &source_path.display().to_string())
        .replace("{diagnostics}", &diagnostics_path.display().to_string())
        .replace("{task}", task)
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskOutcome {
    pub name: String,
    /// True when the broken source was already clean (should be false for
    /// genuinely-broken tasks; used to catch false positives).
    pub first_pass_ok: bool,
    /// Number of repair rounds applied.
    pub iterations: usize,
    /// True when the program was clean by the end of the loop.
    pub final_ok: bool,
    /// Total diagnostics observed across all rounds.
    pub diagnostics_seen: usize,
    /// Code of the first diagnostic seen (baseline signal).
    pub first_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchMetadata {
    /// Tool that produced the report.
    pub tool: String,
    /// Compiler version, so results can be tied to a specific build.
    pub version: String,
    /// Repairer identifier (`oracle`, `none`, `command`, …).
    pub repairer: String,
    /// Iteration budget used for every task.
    pub max_iters: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchReport {
    pub metadata: BenchMetadata,
    pub tasks: Vec<TaskOutcome>,
    pub total: usize,
    pub first_pass_ok: usize,
    pub fixed: usize,
    pub unresolved: usize,
    /// Mean iterations for tasks that were not already clean.
    pub avg_iterations: f64,
    pub total_diagnostics: usize,
}

/// Load every task directory under `dir`. Each must contain `broken.ag`
/// and optionally `fixed.ag` (defaults to `broken.ag` if absent).
pub fn load_tasks(dir: &Path) -> std::io::Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path();
        let broken_path = path.join("broken.ag");
        if !broken_path.exists() {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let broken = fs::read_to_string(&broken_path)?;
        let fixed_path = path.join("fixed.ag");
        let fixed = if fixed_path.exists() {
            fs::read_to_string(&fixed_path)?
        } else {
            broken.clone()
        };
        tasks.push(Task {
            name,
            broken,
            fixed,
        });
    }
    tasks.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tasks)
}

pub fn run_task(task: &Task, repairer: &mut dyn Repairer, max_iters: usize) -> TaskOutcome {
    let file_label = format!("benchmarks/{}/broken.ag", task.name);
    let mut source = task.broken.clone();
    let mut diags = check_source(&source, &file_label);
    let first_pass_ok = diags.is_empty();
    let first_code = diags.first().map(|d| d.code.clone());
    let mut diagnostics_seen = diags.len();
    let mut iterations = 0;

    while !diags.is_empty() && iterations < max_iters {
        match repairer.repair(task, &source, &diags) {
            Some(next) if next != source => {
                source = next;
                iterations += 1;
                diags = check_source(&source, &file_label);
                diagnostics_seen += diags.len();
            }
            _ => break,
        }
    }

    TaskOutcome {
        name: task.name.clone(),
        first_pass_ok,
        iterations,
        final_ok: diags.is_empty(),
        diagnostics_seen,
        first_code,
    }
}

pub fn run_suite(tasks: &[Task], repairer: &mut dyn Repairer, max_iters: usize) -> BenchReport {
    let outcomes: Vec<TaskOutcome> = tasks
        .iter()
        .map(|task| run_task(task, repairer, max_iters))
        .collect();

    let total = outcomes.len();
    let first_pass_ok = outcomes.iter().filter(|o| o.first_pass_ok).count();
    let fixed = outcomes.iter().filter(|o| o.final_ok).count();
    let unresolved = total - fixed;
    let total_diagnostics = outcomes.iter().map(|o| o.diagnostics_seen).sum();
    let repaired: Vec<&TaskOutcome> = outcomes.iter().filter(|o| !o.first_pass_ok).collect();
    let avg_iterations = if repaired.is_empty() {
        0.0
    } else {
        repaired.iter().map(|o| o.iterations as f64).sum::<f64>() / repaired.len() as f64
    };

    BenchReport {
        metadata: BenchMetadata {
            tool: "agentlang".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            repairer: repairer.name().to_string(),
            max_iters,
        },
        tasks: outcomes,
        total,
        first_pass_ok,
        fixed,
        unresolved,
        avg_iterations,
        total_diagnostics,
    }
}
