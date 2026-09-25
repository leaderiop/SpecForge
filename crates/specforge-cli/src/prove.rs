//! `specforge analyze --prove` — formal expression verification.
//!
//! Rung 2 of the formal ladder (RES-25; Leino/de Moura anchors): governance
//! `constraint` entities declare `metric` blocks whose lines are parsed as
//! expressions in the shared formal language ([`specforge_parser::expr`]):
//!
//! ```text
//! latency < 100ms
//! latency > 500ms and latency > 0
//! ```
//!
//! Every parseable line becomes a conjunct asserted **individually** with a
//! `:named` annotation. All conjuncts corpus-wide are checked together with
//! z3 using **unsat cores**: an unsat result names the minimal set of
//! individual bounds that contradict each other (E046), across files —
//! with the constraint id and metric-relative line for each. Prose lines
//! that do not parse as expressions are skipped and counted. Unit suffixes
//! (`ms`, `MB`) are carried but compared as raw numbers — unit
//! normalization is future work.

use std::collections::HashMap;
use std::process::Command;

use specforge_common::{Diagnostic, SourceSpan};
use specforge_emitter::analyze::AnalysisContext;
use specforge_parser::{Expr, SpannedExpr, parse_expression};

/// Result of one prove run over the compiled project.
pub struct ProveReport {
    pub findings: Vec<Diagnostic>,
    pub summary: serde_json::Value,
}

// ── SMT-LIB2 encoding over the shared AST ───────────────────────────────────

fn collect_vars(expr: &SpannedExpr, vars: &mut Vec<String>) {
    match &expr.expr {
        Expr::Num(_, _) => {}
        Expr::Var(name) => {
            if !vars.contains(name) {
                vars.push(name.clone());
            }
        }
        Expr::Cmp(_, l, r)
        | Expr::And(l, r)
        | Expr::Or(l, r)
        | Expr::Add(l, r)
        | Expr::Sub(l, r) => {
            collect_vars(l, vars);
            collect_vars(r, vars);
        }
        Expr::Neg(e) => collect_vars(e, vars),
    }
}

fn encode_expr(expr: &SpannedExpr) -> String {
    match &expr.expr {
        Expr::Num(v, _unit) => {
            if v.fract() == 0.0 {
                format!("{v:.1}")
            } else {
                format!("{v}")
            }
        }
        Expr::Var(name) => name.clone(),
        Expr::Cmp(op, l, r) => format!("({} {} {})", op.as_smt(), encode_expr(l), encode_expr(r)),
        Expr::And(l, r) => format!("(and {} {})", encode_expr(l), encode_expr(r)),
        Expr::Or(l, r) => format!("(or {} {})", encode_expr(l), encode_expr(r)),
        Expr::Add(l, r) => format!("(+ {} {})", encode_expr(l), encode_expr(r)),
        Expr::Sub(l, r) => format!("(- {} {})", encode_expr(l), encode_expr(r)),
        Expr::Neg(e) => format!("(- {})", encode_expr(e)),
    }
}

// ── solver ──────────────────────────────────────────────────────────────────

fn z3_available() -> bool {
    Command::new("z3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run z3 with unsat cores enabled; returns (result, core-names-or-empty).
fn run_z3(script: &str) -> Option<(String, Vec<String>)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "specforge-prove-{}-{}.smt2",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&tmp, script).ok()?;
    let output = Command::new("z3").arg(&tmp).output().ok()?;
    let _ = std::fs::remove_file(&tmp);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let result = lines.next()?.trim().to_string();
    let core = lines
        .next()
        .filter(|_| result == "unsat")
        .map(|l| {
            l.trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some((result, core))
}

// ── the pass ────────────────────────────────────────────────────────────────

/// One parseable metric line, with provenance for diagnostics.
struct Conjunct {
    expr: SpannedExpr,
    text: String,
    /// 1-based line within the metric content.
    rel_line: usize,
}

struct ConstraintFormulas {
    id: String,
    span: SourceSpan,
    conjuncts: Vec<Conjunct>,
}

/// Run the prove pass: parse every constraint's metric lines into conjuncts,
/// assert them corpus-wide with per-conjunct names, and check the
/// conjunction with z3. An unsat result carries the unsat core naming the
/// minimal set of bounds that contradict each other (E046).
pub fn run_prove(ctx: &AnalysisContext) -> ProveReport {
    let mut findings = Vec::new();
    let mut skipped_prose_lines = 0usize;
    let mut constraints_with_metrics = 0usize;
    let mut conjunct_count = 0usize;
    let mut constraints: Vec<ConstraintFormulas> = Vec::new();
    let solver_available = z3_available();
    let mut solver_version = String::from("not found");

    if solver_available && let Ok(output) = Command::new("z3").arg("--version").output() {
        solver_version = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("z3")
            .to_string();
    }

    for node in ctx.graph.nodes() {
        if node.kind.raw.as_str() != "constraint" {
            continue;
        }
        let Some(specforge_graph::FieldValue::String(metric)) = node.fields.get("metric") else {
            continue;
        };
        if metric.trim().is_empty() {
            continue;
        }
        constraints_with_metrics += 1;

        let mut conjuncts: Vec<Conjunct> = Vec::new();
        for (i, line) in metric.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match parse_expression(line) {
                Ok(expr) => conjuncts.push(Conjunct {
                    expr,
                    text: line.trim().to_string(),
                    rel_line: i + 1,
                }),
                Err(_) => skipped_prose_lines += 1,
            }
        }
        if !conjuncts.is_empty() {
            conjunct_count += conjuncts.len();
            constraints.push(ConstraintFormulas {
                id: node.id.raw.to_string(),
                span: node.source_span.clone(),
                conjuncts,
            });
        }
    }

    let mut satisfiable = false;
    let mut unsat = false;

    if solver_available && !constraints.is_empty() {
        let mut vars: Vec<String> = Vec::new();
        for c in &constraints {
            for conj in &c.conjuncts {
                collect_vars(&conj.expr, &mut vars);
            }
        }

        // name → (constraint index, conjunct index)
        let mut names: HashMap<String, (usize, usize)> = HashMap::new();
        let mut script = String::from("(set-option :produce-unsat-cores true)\n");
        for var in &vars {
            script.push_str(&format!("(declare-const {var} Real)\n"));
        }
        for (ci, c) in constraints.iter().enumerate() {
            let safe_id = c.id.replace(['/', '@'], "_");
            for (ji, conj) in c.conjuncts.iter().enumerate() {
                let name = format!("c{ci}_{ji}_{safe_id}");
                names.insert(name.clone(), (ci, ji));
                script.push_str(&format!(
                    "(assert (! {} :named {name}))\n",
                    encode_expr(&conj.expr)
                ));
            }
        }
        script.push_str("(check-sat)\n");
        script.push_str("(get-unsat-core)\n");

        match run_z3(&script) {
            Some((result, core)) if result == "unsat" => {
                unsat = true;
                let cited: Vec<String> = core
                    .iter()
                    .filter_map(|name| {
                        names.get(name).map(|(ci, ji)| {
                            let c = &constraints[*ci];
                            let conj = &c.conjuncts[*ji];
                            format!(
                                "`{}` in constraint {} (metric line {})",
                                conj.text, c.id, conj.rel_line
                            )
                        })
                    })
                    .collect();
                let mut diagnostic = Diagnostic::error(
                    "E046",
                    format!("contradictory metric bounds: {}", cited.join("; ")),
                )
                .with_suggestion("relax or correct the listed metric bounds");
                if let Some(first) = core
                    .first()
                    .and_then(|name| names.get(name))
                    .map(|(ci, _)| &constraints[*ci].span)
                {
                    diagnostic = diagnostic.with_span(first.clone());
                }
                findings.push(diagnostic);
            }
            Some((result, _)) if result == "sat" => satisfiable = true,
            Some((_, _)) => {
                findings.push(Diagnostic::info(
                    "I098",
                    "the solver could not decide the combined metric bounds".to_string(),
                ));
            }
            None => {}
        }
    }

    let summary = serde_json::json!({
        "solver": solver_version,
        "solver_available": solver_available,
        "constraints_with_metrics": constraints_with_metrics,
        "formulas": constraints.len(),
        "conjuncts": conjunct_count,
        "skipped_prose_lines": skipped_prose_lines,
        "satisfiable": satisfiable,
        "unsatisfiable": unsat,
    });
    ProveReport { findings, summary }
}
