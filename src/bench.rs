//! Benchmark harness for the agentic self-repair loop.
//!
//! The idea: a task is a *broken* `.ag` program plus a known-good version. The
//! harness runs `check`, feeds the structured diagnostics to a [`Repairer`], and
//! repeats until the program is clean (or the iteration budget runs out).
//!
//! A real LLM plugs in by implementing [`Repairer`]. The built-in [`OracleRepairer`]
//! returns the known-good source and exists to validate the loop and record an
//! ideal-case baseline (all broken tasks should resolve in one iteration).

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::check_source;
use crate::diagnostic::Diagnostic;

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
}

/// Baseline repairer: returns the known-good source immediately.
pub struct OracleRepairer;

impl Repairer for OracleRepairer {
    fn repair(&mut self, task: &Task, _source: &str, _diags: &[Diagnostic]) -> Option<String> {
        Some(task.fixed.clone())
    }
}

/// A repairer that never repairs — measures first-pass behavior only.
pub struct NoopRepairer;

impl Repairer for NoopRepairer {
    fn repair(&mut self, _task: &Task, _source: &str, _diags: &[Diagnostic]) -> Option<String> {
        None
    }
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
pub struct BenchReport {
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
        tasks: outcomes,
        total,
        first_pass_ok,
        fixed,
        unresolved,
        avg_iterations,
        total_diagnostics,
    }
}
