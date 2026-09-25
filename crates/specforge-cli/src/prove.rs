//! `specforge analyze --prove` — numeric constraint verification.
//!
//! The first rung of the formal ladder (RES-25, Leino/de Moura anchors):
//! governance `constraint` entities declare `metric` blocks whose lines are
//! machine-parseable comparisons (`identifier (<|<=|>|>=|==) number [unit]`).
//! Each constraint's bounds are encoded as SMT-LIB2 assertions over Reals and
//! checked for satisfiability with z3. An unsatisfiable constraint is a
//! contradiction in the declared bounds (E046) — no test can ever satisfy it.
//!
//! Prose-only metrics (no parseable comparisons) are counted and skipped.
//! The general contract→VC encoding (full Boogie-style prove) stays future
//! until the condition layer carries formal expressions.

use std::process::Command;

use specforge_common::Diagnostic;
use specforge_emitter::analyze::AnalysisContext;
use specforge_graph::FieldValue;

/// Result of one prove run over the compiled project.
pub struct ProveReport {
    pub findings: Vec<Diagnostic>,
    pub summary: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SmtOp {
    Le,
    Ge,
    Lt,
    Gt,
    Eq,
}

impl SmtOp {
    fn parse(token: &str) -> Option<Self> {
        match token {
            "<=" => Some(Self::Le),
            ">=" => Some(Self::Ge),
            "<" => Some(Self::Lt),
            ">" => Some(Self::Gt),
            "==" | "=" => Some(Self::Eq),
            _ => None,
        }
    }

    fn as_smt(self) -> &'static str {
        match self {
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Eq => "=",
        }
    }
}

#[derive(Debug, Clone)]
struct Comparison {
    var: String,
    op: SmtOp,
    value: f64,
}

/// Split a token into its leading numeric prefix and the unit remainder.
/// `100ms` → (100.0, "ms"); `42.5` → (42.5, "").
fn numeric_prefix(token: &str) -> Option<(f64, String)> {
    let bytes = token.as_bytes();
    let mut end = 0usize;
    let mut seen_dot = false;
    while end < bytes.len() && (bytes[end].is_ascii_digit() || (bytes[end] == b'.' && !seen_dot)) {
        if bytes[end] == b'.' {
            seen_dot = true;
        }
        end += 1;
    }
    if end == 0 {
        return None;
    }
    let value = token[..end].parse::<f64>().ok()?;
    Some((value, token[end..].to_string()))
}

fn is_var_name(token: &str) -> bool {
    let mut chars = token.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
}

/// Parse the machine-checkable comparisons from a metric block. Lines that
/// do not match `var op number [unit]` are prose and are skipped.
fn parse_metric_comparisons(metric: &str) -> (Vec<Comparison>, usize) {
    let mut comparisons = Vec::new();
    let mut prose_lines = 0usize;
    for line in metric.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 3 {
            prose_lines += 1;
            continue;
        }
        let (Some(op), Some((value, _unit))) = (SmtOp::parse(tokens[1]), numeric_prefix(tokens[2]))
        else {
            prose_lines += 1;
            continue;
        };
        if !is_var_name(tokens[0]) {
            prose_lines += 1;
            continue;
        }
        comparisons.push(Comparison {
            var: tokens[0].to_string(),
            op,
            value,
        });
    }
    (comparisons, prose_lines)
}

/// Encode a constraint's comparisons as an SMT-LIB2 check-sat script.
fn encode_smt_lib(comparisons: &[Comparison]) -> String {
    let mut out = String::new();
    let mut declared: Vec<&str> = Vec::new();
    for c in comparisons {
        if !declared.contains(&c.var.as_str()) {
            out.push_str(&format!("(declare-const {} Real)\n", c.var));
            declared.push(&c.var);
        }
    }
    for c in comparisons {
        let literal = if c.value.fract() == 0.0 {
            format!("{:.1}", c.value)
        } else {
            format!("{}", c.value)
        };
        out.push_str(&format!(
            "(assert ({} {} {}))\n",
            c.op.as_smt(),
            c.var,
            literal
        ));
    }
    out.push_str("(check-sat)\n");
    out
}

fn z3_available() -> bool {
    Command::new("z3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run z3 on an SMT-LIB2 script; returns the first token of its output
/// (sat / unsat / unknown) or None when the solver is unavailable.
fn run_z3(script: &str, id: &str) -> Option<String> {
    let tmp = std::env::temp_dir().join(format!(
        "specforge-prove-{}.smt2",
        id.replace(['/', '@'], "_")
    ));
    std::fs::write(&tmp, script).ok()?;
    let output = Command::new("z3").arg(&tmp).output().ok()?;
    let _ = std::fs::remove_file(&tmp);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(stdout.split_whitespace().next().unwrap_or("").to_string())
}

/// Run the prove pass: verify every constraint entity's metric bounds are
/// satisfiable. Returns findings (E046 errors for unsatisfiable bounds) and
/// a summary with the verification breakdown.
pub fn run_prove(ctx: &AnalysisContext) -> ProveReport {
    let mut findings = Vec::new();
    let mut satisfiable = 0usize;
    let mut unsatisfiable = 0usize;
    let mut skipped_prose = 0usize;
    let mut solver_available = z3_available();
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
        let Some(FieldValue::String(metric)) = node.fields.get("metric") else {
            continue;
        };
        if metric.trim().is_empty() {
            continue;
        }

        let (comparisons, prose) = parse_metric_comparisons(metric);
        skipped_prose += prose;

        if comparisons.is_empty() || !solver_available {
            continue;
        }

        let script = encode_smt_lib(&comparisons);
        match run_z3(&script, &node.id.raw.as_str().replace(['/', '@'], "_")) {
            Some(result) if result == "unsat" => {
                unsatisfiable += 1;
                findings.push(
                    Diagnostic::error(
                        "E046",
                        format!(
                            "constraint '{}' is unsatisfiable: the metric bounds contradict each other",
                            node.id.raw
                        ),
                    )
                    .with_span(node.source_span.clone())
                    .with_suggestion(
                        "relax or correct one of the metric bounds in the metric block",
                    ),
                );
            }
            Some(result) if result == "sat" => satisfiable += 1,
            Some(_) | None => {
                // solver unavailable or returned unknown: leave unchecked
                solver_available = false;
                solver_version = "unavailable".to_string();
            }
        }
    }

    let summary = serde_json::json!({
        "solver": solver_version,
        "constraints_checked": satisfiable + unsatisfiable,
        "satisfiable": satisfiable,
        "unsatisfiable": unsatisfiable,
        "skipped_prose_metrics": skipped_prose,
        "solver_available": solver_available,
    });
    ProveReport { findings, summary }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comparisons_with_units_and_skips_prose() {
        let metric = "file_change_to_diagnostics < 100ms\n\
                      with up to 500 .spec files in the project\n\
                      peak_memory <= 50.5 MB\n\
                      requests == 1000\n\
                      timeout >= 5s\n";
        let (comparisons, prose) = parse_metric_comparisons(metric);
        assert_eq!(prose, 1);
        assert_eq!(comparisons.len(), 4);
        assert_eq!(comparisons[0].var, "file_change_to_diagnostics");
        assert_eq!(comparisons[0].op, SmtOp::Lt);
        assert_eq!(comparisons[0].value, 100.0);
        assert_eq!(comparisons[1].var, "peak_memory");
        assert_eq!(comparisons[1].op, SmtOp::Le);
        assert_eq!(comparisons[1].value, 50.5);
        assert_eq!(comparisons[2].op, SmtOp::Eq);
        assert_eq!(comparisons[3].op, SmtOp::Ge);
    }

    #[test]
    fn prose_only_metrics_parse_to_empty() {
        let (comparisons, prose) = parse_metric_comparisons(
            "The CLI and LSP binaries MUST build and run on:\n\
             Linux (x86_64, aarch64), macOS (x86_64, aarch64).\n",
        );
        assert!(comparisons.is_empty());
        assert_eq!(prose, 2);
    }

    #[test]
    fn encodes_declares_and_asserts() {
        let comparisons = vec![
            Comparison {
                var: "latency".into(),
                op: SmtOp::Lt,
                value: 100.0,
            },
            Comparison {
                var: "latency".into(),
                op: SmtOp::Gt,
                value: 500.0,
            },
        ];
        let smt = encode_smt_lib(&comparisons);
        assert_eq!(
            smt.matches("(declare-const latency Real)").count(),
            1,
            "each variable declared once: {smt}"
        );
        assert!(smt.contains("(assert (< latency 100.0))"));
        assert!(smt.contains("(assert (> latency 500.0))"));
        assert!(smt.contains("(check-sat)"));
    }

    #[test]
    fn z3_reports_unsat_for_contradictory_bounds() {
        if !z3_available() {
            eprintln!("z3 not installed — skipping solver test");
            return;
        }
        let script = encode_smt_lib(&[
            Comparison {
                var: "latency".into(),
                op: SmtOp::Lt,
                value: 100.0,
            },
            Comparison {
                var: "latency".into(),
                op: SmtOp::Gt,
                value: 500.0,
            },
        ]);
        assert_eq!(run_z3(&script, "unit").as_deref(), Some("unsat"));
    }

    #[test]
    fn z3_reports_sat_for_consistent_bounds() {
        if !z3_available() {
            eprintln!("z3 not installed — skipping solver test");
            return;
        }
        let script = encode_smt_lib(&[
            Comparison {
                var: "latency".into(),
                op: SmtOp::Lt,
                value: 100.0,
            },
            Comparison {
                var: "latency".into(),
                op: SmtOp::Gt,
                value: 10.0,
            },
        ]);
        assert_eq!(run_z3(&script, "unit2").as_deref(), Some("sat"));
    }
}
