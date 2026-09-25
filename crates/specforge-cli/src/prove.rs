//! `specforge analyze --prove` — numeric constraint verification.
//!
//! The first rung of the formal ladder (RES-25, Leino/de Moura anchors):
//! governance `constraint` entities declare `metric` blocks whose lines are
//! machine-parseable comparisons (`identifier (<|<=|>|>=|==) number [unit]`).
//! All comparisons are grouped per variable+unit across the whole corpus and
//! each group is encoded as SMT-LIB2 assertions over Reals and checked for
//! satisfiability with z3. An unsatisfiable group is a contradiction in the
//! declared bounds (E046) — no value can ever satisfy it, no matter which
//! constraint declared which side.
//!
//! Prose-only metrics (no parseable comparisons) are counted and skipped.
//! The general contract→VC encoding (full Boogie-style prove) stays future
//! until the condition layer carries formal expressions.

use std::collections::BTreeMap;
use std::process::Command;

use specforge_common::Diagnostic;
use specforge_emitter::analyze::AnalysisContext;
use specforge_graph::FieldValue;

/// Result of one prove run over the compiled project.
pub struct ProveReport {
    pub findings: Vec<Diagnostic>,
    pub summary: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    unit: String,
    op: SmtOp,
    value: f64,
    /// The constraint entity that declared this bound.
    source: String,
    /// Span of the declaring constraint entity (for diagnostics).
    span: specforge_common::SourceSpan,
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

/// Parse the machine-checkable comparisons from one metric block. Lines that
/// do not match `var op number [unit]` are prose and are skipped. `source`
/// and `span` tag each comparison with its declaring constraint.
fn parse_metric_comparisons(
    metric: &str,
    source: &str,
    span: &specforge_common::SourceSpan,
) -> (Vec<Comparison>, usize) {
    let mut comparisons = Vec::new();
    let mut prose_lines = 0usize;
    for line in metric.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 3 {
            prose_lines += 1;
            continue;
        }
        let (Some(op), Some((value, unit))) = (SmtOp::parse(tokens[1]), numeric_prefix(tokens[2]))
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
            unit,
            op,
            value,
            source: source.to_string(),
            span: span.clone(),
        });
    }
    (comparisons, prose_lines)
}

/// Encode one bound group as an SMT-LIB2 check-sat script.
fn encode_smt_lib(var: &str, unit: &str, comparisons: &[Comparison]) -> String {
    let _ = unit;
    let mut out = String::new();
    out.push_str(&format!("(declare-const {var} Real)\n"));
    for c in comparisons {
        let literal = if c.value.fract() == 0.0 {
            format!("{:.1}", c.value)
        } else {
            format!("{}", c.value)
        };
        out.push_str(&format!("(assert ({} {var} {literal}))\n", c.op.as_smt()));
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
fn run_z3(script: &str) -> Option<String> {
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
    Some(stdout.split_whitespace().next().unwrap_or("").to_string())
}

/// Run the prove pass: collect every machine-parseable metric bound in the
/// corpus, group them per variable+unit, and verify each group is
/// satisfiable with z3. An unsatisfiable group is E046 — the bounds
/// contradict each other across constraints.
pub fn run_prove(ctx: &AnalysisContext) -> ProveReport {
    let mut findings = Vec::new();
    let mut skipped_prose = 0usize;
    let mut comparisons: Vec<Comparison> = Vec::new();
    let mut constraints_with_bounds = 0usize;
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
        let Some(FieldValue::String(metric)) = node.fields.get("metric") else {
            continue;
        };
        if metric.trim().is_empty() {
            continue;
        }
        constraints_with_bounds += 1;

        let (mut parsed, prose) =
            parse_metric_comparisons(metric, node.id.raw.as_str(), &node.source_span);
        skipped_prose += prose;
        if !solver_available {
            // Without a solver the comparisons cannot be checked; skip them.
            parsed.clear();
        }
        comparisons.append(&mut parsed);
    }

    // Group per (variable, unit). Bounds declared in different units are
    // different groups — comparing 100ms with 2s requires unit normalization
    // that is deliberately out of scope for the v1 rung.
    let mut groups: BTreeMap<(String, String), Vec<Comparison>> = BTreeMap::new();
    for c in comparisons {
        groups
            .entry((c.var.clone(), c.unit.clone()))
            .or_default()
            .push(c);
    }

    let mut satisfiable_groups = 0usize;
    let mut unsat_groups = 0usize;
    for ((var, unit), comps) in &groups {
        let script = encode_smt_lib(var, unit, comps);
        match run_z3(&script) {
            Some(result) if result == "unsat" => {
                unsat_groups += 1;
                let sources: Vec<&str> =
                    comps.iter().map(|c| c.source.as_str()).collect::<Vec<_>>();
                let source_clause = if sources.len() == 1 {
                    format!(" in constraint '{}'", sources[0])
                } else {
                    format!(" across constraints: {}", sources.join(", "))
                };
                findings.push(
                    Diagnostic::error(
                        "E046",
                        format!(
                            "variable '{var}' ({unit}) has contradictory bounds{source_clause}"
                        ),
                    )
                    .with_span(comps[0].span.clone())
                    .with_suggestion(
                        "relax or correct one of the metric bounds so the group is satisfiable",
                    ),
                );
            }
            Some(result) if result == "sat" => satisfiable_groups += 1,
            _ => {
                findings.push(
                    Diagnostic::info(
                        "I098",
                        format!(
                            "variable '{var}' ({unit}): the solver could not decide the bound group"
                        ),
                    )
                    .with_span(comps[0].span.clone()),
                );
            }
        }
    }

    let summary = serde_json::json!({
        "solver": solver_version,
        "solver_available": solver_available,
        "constraints_with_bounds": constraints_with_bounds,
        "variables_checked": groups.len(),
        "satisfiable_groups": satisfiable_groups,
        "unsatisfiable_groups": unsat_groups,
        "skipped_prose_metrics": skipped_prose,
    });
    ProveReport { findings, summary }
}

#[cfg(test)]
mod grouping_tests {
    use super::*;
    use specforge_graph::{EntityId, EntityKind, FieldMap, Graph, Node};
    use specforge_registry::{FieldRegistry, KindRegistry};

    fn constraint_node(id: &str, metric: &str) -> Node {
        let mut fields = FieldMap::new();
        fields.push(
            specforge_common::Sym::new("metric"),
            FieldValue::String(metric.to_string()),
        );
        Node {
            id: EntityId {
                raw: specforge_common::Sym::new(id),
            },
            kind: EntityKind {
                raw: specforge_common::Sym::new("constraint"),
            },
            title: None,
            fields,
            source_span: specforge_common::SourceSpan {
                file: specforge_common::Sym::new(id),
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 1,
            },
        }
    }

    fn context<'a>(
        graph: &'a Graph,
        kind_registry: &'a KindRegistry,
        field_registry: &'a FieldRegistry,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            graph,
            kind_registry,
            field_registry,
            project_root: None,
            test_results: None,
        }
    }

    #[test]
    fn cross_constraint_contradictions_are_detected() {
        let kind_reg = KindRegistry::new();
        let field_reg = FieldRegistry::new();
        let mut graph = Graph::new();
        graph.add_node(constraint_node("a", "latency < 100ms"));
        graph.add_node(constraint_node("b", "latency > 500ms"));

        let report = run_prove(&context(&graph, &kind_reg, &field_reg));
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "E046" && f.message.contains("across constraints")),
            "cross-constraint contradiction must be reported: {:?}",
            report.findings
        );
        assert_eq!(report.summary["unsatisfiable_groups"], 1);
        assert_eq!(report.summary["variables_checked"], 1);
    }

    #[test]
    fn different_units_are_separate_groups() {
        let kind_reg = KindRegistry::new();
        let field_reg = FieldRegistry::new();
        let mut graph = Graph::new();
        graph.add_node(constraint_node("a", "latency < 100ms"));
        graph.add_node(constraint_node("b", "latency > 2s"));

        let report = run_prove(&context(&graph, &kind_reg, &field_reg));
        assert!(
            !report.findings.iter().any(|f| f.code == "E046"),
            "different units must not be compared: {:?}",
            report.findings
        );
        assert_eq!(report.summary["variables_checked"], 2);
    }
}
