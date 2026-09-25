//! `specforge analyze --prove` — formal expression verification.
//!
//! Rungs 2–3 of the formal ladder (RES-25; Leino/de Moura anchors):
//!
//! **Consistency** (rung 2): every `constraint`'s parseable metric lines
//! become individually named conjuncts asserted corpus-wide with z3. An
//! unsat result carries the unsat core naming the minimal set of bounds
//! that contradict each other (E046), across files, with per-bound
//! citations.
//!
//! **Entailment** (rung 3): any entity carrying an `expression` field is a
//! *claim*. Each claim is checked as a verification condition
//! `declared_bounds ⇒ claim` by asking z3 whether
//! `bounds ∧ ¬claim` is satisfiable: unsat proves the claim from the
//! declared bounds; sat yields a **counterexample model** — concrete
//! values satisfying every declared bound while violating the claim
//! (E047), which is exactly the failure evidence a counterexample-guided
//! loop consumes.
//!
//! Prose lines that do not parse as expressions are skipped and counted.
//! Unit suffixes (`ms`, `MB`) are carried but compared as raw numbers —
//! unit normalization is future work.

use std::process::Command;

use specforge_common::{Diagnostic, SourceSpan, Sym};
use specforge_emitter::analyze::AnalysisContext;
use specforge_parser::{Expr, SpannedExpr, parse_expression};

/// Result of one prove run over the compiled project.
pub struct ProveReport {
    pub findings: Vec<Diagnostic>,
    pub summary: serde_json::Value,
    /// Ids of entities whose formal claims were ENTAILED from the declared
    /// bounds; consumed by the coverage pass discharge funnel.
    pub proved_claim_ids: Vec<String>,
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
        Expr::Neg(e) | Expr::Not(e) => collect_vars(e, vars),
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
        Expr::Not(e) => format!("(not {})", encode_expr(e)),
    }
}

fn encode_conjunction(parts: &[SpannedExpr]) -> Option<String> {
    let mut iter = parts.iter();
    let first = encode_expr(iter.next()?);
    let rest: Vec<String> = iter.map(encode_expr).collect();
    Some(if rest.is_empty() {
        first
    } else {
        format!("(and {first} {})", rest.join(" "))
    })
}

// ── solver ──────────────────────────────────────────────────────────────────

fn z3_available() -> bool {
    Command::new("z3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run z3 and return its full stdout (sat/unsat line, optional core or
/// model sections), or None when the solver could not be executed.
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
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn first_result_line(stdout: &str) -> &str {
    stdout.lines().next().unwrap_or("").trim()
}

/// Extract the unsat core names from z3 stdout (a parenthesized name list
/// on the line after `unsat`).
fn parse_unsat_core(stdout: &str) -> Vec<String> {
    let mut lines = stdout.lines();
    if lines.next().map(str::trim) != Some("unsat") {
        return Vec::new();
    }
    lines
        .next()
        .map(|l| {
            l.trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// Extract `(name, value)` pairs from a z3 model section. Handles both the
/// one-line `(define-fun x () Real 250.0)` and the pretty-printed
/// multi-line form; rational results like `(/ 1.0 4.0)` pass through raw.
fn parse_model(stdout: &str) -> Vec<(String, String)> {
    // z3 emits the result line, then the model: a bare `(`, define-fun
    // blocks (one-line or pretty-printed), and a closing `)`. The model is
    // the only content after the result line, so scan everything after it.
    let mut out = Vec::new();
    let mut pending: Option<String> = None;
    for line in stdout.lines().skip(1) {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("(define-fun ") {
            let toks: Vec<&str> = rest.split_whitespace().collect();
            // toks: [name, "()", "Real", value...] — value present only in
            // the one-line form; the pretty-printed form wraps to the next line
            if toks.len() >= 3 && toks[2] == "Real" {
                if toks.len() > 3 {
                    let mut value = toks[3..].join(" ");
                    if value.ends_with(')') {
                        value.pop();
                    }
                    out.push((toks[0].to_string(), value));
                    pending = None;
                } else {
                    pending = Some(toks[0].to_string());
                }
            } else {
                pending = None;
            }
            continue;
        }
        if let Some(name) = pending.take() {
            let value = t.strip_suffix(')').unwrap_or(t).trim();
            if !value.is_empty() {
                out.push((name, value.to_string()));
            }
        }
    }
    out
}

fn render_counterexample(model: &[(String, String)]) -> String {
    model
        .iter()
        .take(4)
        .map(|(name, value)| format!("{name} = {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

// ── the pass ────────────────────────────────────────────────────────────────

/// One parseable formal bound, with provenance for diagnostics.
struct Conjunct {
    expr: SpannedExpr,
    text: String,
    /// Human location: metric-relative line (string form) or file line
    /// (first-class `expr { }` form).
    loc: String,
    /// Owning entity's source span (attached by the collection loops).
    span: SourceSpan,
}

struct Claim {
    id: String,
    span: SourceSpan,
    expr: SpannedExpr,
    text: String,
}

/// Parse a field that may be first-class (`expr { }`) or a legacy string
/// block, returning conjuncts plus the number of skipped prose lines.
fn conjuncts_from_field(value: &specforge_graph::FieldValue) -> (Vec<Conjunct>, usize) {
    let mut skipped = 0usize;
    let mut out = Vec::new();
    match value {
        specforge_graph::FieldValue::String(metric) => {
            for (i, line) in metric.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                match parse_expression(line) {
                    Ok(expr) => out.push(Conjunct {
                        expr,
                        text: line.trim().to_string(),
                        loc: format!("metric line {}", i + 1),
                        span: SourceSpan {
                            file: Sym::new(""),
                            start_line: 0,
                            start_col: 0,
                            end_line: 0,
                            end_col: 0,
                        },
                    }),
                    Err(_) => skipped += 1,
                }
            }
        }
        specforge_graph::FieldValue::Expression(exprs) => {
            for e in exprs {
                out.push(Conjunct {
                    expr: e.clone(),
                    text: e.to_string(),
                    loc: format!("line {}", e.span.start_line),
                    span: SourceSpan {
                        file: Sym::new(""),
                        start_line: 0,
                        start_col: 0,
                        end_line: 0,
                        end_col: 0,
                    },
                });
            }
        }
        _ => {}
    }
    (out, skipped)
}

/// Run the prove pass: consistency over declared bounds (E046 with unsat
/// cores) and entailment of formal claims with counterexample models (E047).
pub fn run_prove(ctx: &AnalysisContext) -> ProveReport {
    let mut findings = Vec::new();
    let mut skipped_prose_lines = 0usize;
    let mut constraints_with_metrics = 0usize;
    let mut conjunct_count = 0usize;
    let mut axioms: Vec<Conjunct> = Vec::new();
    let mut claims: Vec<Claim> = Vec::new();
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
        // Axioms: constraint metric bounds.
        if node.kind.raw.as_str() == "constraint"
            && let Some(field) = node.fields.get("metric")
        {
            let (mut conjuncts, skipped) = conjuncts_from_field(field);
            skipped_prose_lines += skipped;
            if !conjuncts.is_empty() {
                constraints_with_metrics += 1;
                conjunct_count += conjuncts.len();
                for c in &mut conjuncts {
                    c.span = node.source_span.clone();
                }
                axioms.extend(conjuncts);
            }
        }

        // Claims: any entity carrying a formal `expression`.
        if let Some(field) = node.fields.get("expression") {
            let (conjuncts, skipped) = conjuncts_from_field(field);
            skipped_prose_lines += skipped;
            for c in conjuncts {
                claims.push(Claim {
                    id: node.id.raw.to_string(),
                    span: node.source_span.clone(),
                    expr: c.expr,
                    text: c.text,
                });
            }
        }
    }

    let mut satisfiable = false;
    let mut unsat = false;
    let mut claims_proved = 0usize;
    let mut claims_unproved = 0usize;
    let mut proved_claim_ids: Vec<String> = Vec::new();

    if solver_available {
        // ── consistency: all bounds together must be satisfiable ────────
        if !axioms.is_empty() {
            let mut vars: Vec<String> = Vec::new();
            for conj in &axioms {
                collect_vars(&conj.expr, &mut vars);
            }
            let mut script = String::from("(set-option :produce-unsat-cores true)\n");
            for var in &vars {
                script.push_str(&format!("(declare-const {var} Real)\n"));
            }
            let mut names: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for (idx, conj) in axioms.iter().enumerate() {
                let name = format!("c{idx}");
                names.insert(name.clone(), idx);
                script.push_str(&format!(
                    "(assert (! {} :named {name}))\n",
                    encode_expr(&conj.expr)
                ));
            }
            script.push_str("(check-sat)\n(get-unsat-core)\n");

            if let Some(stdout) = run_z3(&script) {
                match first_result_line(&stdout) {
                    "unsat" => {
                        unsat = true;
                        let cited: Vec<String> = parse_unsat_core(&stdout)
                            .iter()
                            .filter_map(|name| names.get(name))
                            .map(|&idx| {
                                let conj = &axioms[idx];
                                format!("`{}` ({})", conj.text, conj.loc)
                            })
                            .collect();
                        let core = parse_unsat_core(&stdout);
                        let first_idx = core.first().and_then(|name| names.get(name)).copied();
                        let mut diagnostic = Diagnostic::error(
                            "E046",
                            format!("contradictory metric bounds: {}", cited.join("; ")),
                        )
                        .with_suggestion("relax or correct the listed metric bounds");
                        if let Some(idx) = first_idx {
                            diagnostic = diagnostic.with_span(axioms[idx].span.clone());
                        }
                        findings.push(diagnostic);
                    }
                    "sat" => satisfiable = true,
                    _ => {
                        findings.push(Diagnostic::info(
                            "I098",
                            "the solver could not decide the combined metric bounds".to_string(),
                        ));
                    }
                }
            }
        }

        // ── entailment: bounds ⇒ claim, per claim ────────────────────────
        for claim in &claims {
            let mut vars: Vec<String> = Vec::new();
            for conj in &axioms {
                collect_vars(&conj.expr, &mut vars);
            }
            collect_vars(&claim.expr, &mut vars);

            let mut script = String::from("(set-option :produce-models true)\n");
            for var in &vars {
                script.push_str(&format!("(declare-const {var} Real)\n"));
            }
            if let Some(bounds) =
                encode_conjunction(&axioms.iter().map(|c| c.expr.clone()).collect::<Vec<_>>())
            {
                script.push_str(&format!("(assert {bounds})\n"));
            }
            script.push_str(&format!("(assert (not {}))\n", encode_expr(&claim.expr)));
            script.push_str("(check-sat)\n(get-model)\n");

            if let Some(stdout) = run_z3(&script) {
                match first_result_line(&stdout) {
                    // bounds ∧ ¬claim unsat ⇒ bounds entail the claim
                    "unsat" => {
                        claims_proved += 1;
                        proved_claim_ids.push(claim.id.clone());
                    }
                    "sat" => {
                        claims_unproved += 1;
                        let model = parse_model(&stdout);
                        let evidence = if model.is_empty() {
                            "a satisfying assignment exists".to_string()
                        } else {
                            format!("counterexample: {}", render_counterexample(&model))
                        };
                        findings.push(
                            Diagnostic::warning(
                                "E047",
                                format!(
                                    "claim `{}` of {} is not entailed by the declared bounds ({evidence})",
                                    claim.text, claim.id
                                ),
                            )
                            .with_span(claim.span.clone())
                            .with_suggestion(
                                "strengthen the declared constraint bounds or weaken the claim",
                            ),
                        );
                    }
                    _ => {
                        findings.push(Diagnostic::info(
                            "I098",
                            format!(
                                "the solver could not decide whether the declared bounds entail `{}`",
                                claim.text
                            ),
                        ));
                    }
                }
            }
        }
    }

    let summary = serde_json::json!({
        "solver": solver_version,
        "solver_available": solver_available,
        "constraints_with_metrics": constraints_with_metrics,
        "axioms": axioms.len(),
        "conjuncts": conjunct_count,
        "skipped_prose_lines": skipped_prose_lines,
        "satisfiable": satisfiable,
        "unsatisfiable": unsat,
        "claims": claims.len(),
        "claims_proved": claims_proved,
        "claims_unproved": claims_unproved,
    });
    ProveReport {
        findings,
        summary,
        proved_claim_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::Sym;
    use specforge_graph::{FieldMap, Graph, Node};
    use specforge_parser::{EntityId, EntityKind, FieldValue};
    use specforge_registry::{FieldRegistry, KindRegistry};
    use std::path::Path;

    fn span(file: &str) -> SourceSpan {
        SourceSpan {
            file: Sym::new(file),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 2,
        }
    }

    fn expr_field(text: &str) -> FieldValue {
        FieldValue::Expression(vec![parse_expression(text).expect("parse claim")])
    }

    fn node(id: &str, kind: &str, fields: FieldMap) -> Node {
        Node {
            id: EntityId { raw: Sym::new(id) },
            kind: EntityKind {
                raw: Sym::new(kind),
            },
            title: None,
            fields,
            source_span: span("spec/main.spec"),
        }
    }

    fn constraint_node(id: &str, bound: &str) -> Node {
        let mut fields = FieldMap::new();
        fields.push(Sym::new("metric"), FieldValue::String(bound.to_string()));
        node(id, "constraint", fields)
    }

    fn claim_node(id: &str, expression: &str) -> Node {
        let mut fields = FieldMap::new();
        fields.push(Sym::new("expression"), expr_field(expression));
        node(id, "invariant", fields)
    }

    fn prove(graph: &Graph) -> ProveReport {
        let kind_registry = KindRegistry::default();
        let field_registry = FieldRegistry::default();
        let empty_proved = std::collections::HashSet::new();
        let ctx = AnalysisContext {
            graph,
            kind_registry: &kind_registry,
            field_registry: &field_registry,
            project_root: Some(Path::new(".")),
            test_results: None,
            proved_claims: Some(&empty_proved),
        };
        run_prove(&ctx)
    }

    #[test]
    fn contradictory_bounds_flagged_with_core() {
        let mut g = Graph::new();
        g.add_node(constraint_node("c1", "latency < 100ms"));
        g.add_node(constraint_node("c2", "latency > 500ms"));
        let report = prove(&g);
        assert!(report.summary["unsatisfiable"].as_bool().unwrap());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "E046" && f.message.contains("latency < 100ms"))
        );
    }

    #[test]
    fn satisfiable_bounds_prove_entailed_claim() {
        let mut g = Graph::new();
        g.add_node(constraint_node("budget", "latency < 100ms"));
        g.add_node(claim_node("inv", "latency < 200ms"));
        let report = prove(&g);
        assert!(report.summary["satisfiable"].as_bool().unwrap());
        assert_eq!(report.summary["claims"].as_u64(), Some(1));
        assert_eq!(report.summary["claims_proved"].as_u64(), Some(1));
        assert_eq!(report.summary["claims_unproved"].as_u64(), Some(0));
        assert_eq!(report.proved_claim_ids, vec!["inv".to_string()]);
        assert!(report.findings.iter().all(|f| f.code != "E047"));
    }

    #[test]
    fn unentailed_claim_yields_counterexample() {
        let mut g = Graph::new();
        g.add_node(constraint_node("budget", "latency < 100ms"));
        g.add_node(claim_node("inv", "latency > 500ms"));
        let report = prove(&g);
        assert_eq!(report.summary["claims_unproved"].as_u64(), Some(1));
        let e047 = report
            .findings
            .iter()
            .find(|f| f.code == "E047")
            .expect("E047 expected");
        assert!(
            e047.message.contains("counterexample") && e047.message.contains("latency ="),
            "counterexample must name a violating assignment: {}",
            e047.message
        );
        // the model must satisfy the bound and violate the claim
        assert!(
            e047.message.contains("latency >")
                || e047.message.contains("latency = 5")
                || e047.message.contains("latency = 1")
                || e047.message.contains("latency = 2")
                || e047.message.contains("latency = 3")
                || e047.message.contains("latency = 4")
                || e047.message.contains("latency = 6")
                || e047.message.contains("latency = 7")
                || e047.message.contains("latency = 8")
                || e047.message.contains("latency = 9"),
            "unexpected model: {}",
            e047.message
        );
    }

    #[test]
    fn claim_with_no_bounds_requires_tautology() {
        let mut g = Graph::new();
        g.add_node(claim_node("inv", "latency <= latency"));
        let report = prove(&g);
        assert_eq!(report.summary["claims_proved"].as_u64(), Some(1));

        let mut g2 = Graph::new();
        g2.add_node(claim_node("inv", "latency < 100ms"));
        let report2 = prove(&g2);
        assert_eq!(report2.summary["claims_unproved"].as_u64(), Some(1));
    }

    #[test]
    fn prose_metric_lines_are_skipped_and_counted() {
        let mut g = Graph::new();
        let mut fields = FieldMap::new();
        fields.push(
            Sym::new("metric"),
            FieldValue::String(
                "latency < 100ms\nwith up to 500 .spec files the parser stays fast".to_string(),
            ),
        );
        g.add_node(node("c1", "constraint", fields));
        let report = prove(&g);
        assert_eq!(report.summary["axioms"].as_u64(), Some(1));
        assert_eq!(report.summary["skipped_prose_lines"].as_u64(), Some(1));
        assert!(report.summary["satisfiable"].as_bool().unwrap());
    }

    #[test]
    fn expr_form_field_is_consumed() {
        let mut g = Graph::new();
        let mut fields = FieldMap::new();
        fields.push(Sym::new("metric"), expr_field("latency < 100ms"));
        g.add_node(node("budget", "constraint", fields));
        g.add_node(claim_node("inv", "latency < 150ms"));
        let report = prove(&g);
        assert_eq!(report.summary["claims_proved"].as_u64(), Some(1));
    }
}
