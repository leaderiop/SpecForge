//! `specforge analyze --prove` — formal expression verification.
//!
//! Rung 1 of the formal ladder (RES-25; Leino/de Moura anchors): governance
//! `constraint` entities declare `metric` blocks whose lines are parsed as
//! expressions in a small arithmetic/comparison language:
//!
//! ```text
//! latency < 100ms
//! latency > 500ms and latency > 0
//! ```
//!
//! Each constraint's parseable lines conjoin into one formula. All formulas
//! are checked together with z3 using **unsat cores**: when the corpus's
//! bounds are mutually contradictory, the core names the minimal set of
//! constraints that contradict each other (E046). Prose lines that do not
//! parse as expressions are skipped and counted. Unit suffixes (`ms`, `MB`)
//! are carried on numeric literals but compared as raw numbers — unit
//! normalization is future work.

use std::process::Command;

use specforge_common::{Diagnostic, SourceSpan};
use specforge_emitter::analyze::AnalysisContext;
use specforge_graph::FieldValue;

/// Result of one prove run over the compiled project.
pub struct ProveReport {
    pub findings: Vec<Diagnostic>,
    pub summary: serde_json::Value,
}

// ── expression language ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64, String), // value, unit suffix
    Var(String),
    And,
    Or,
    Not,
    Le,
    Ge,
    Lt,
    Gt,
    Eq,
    Ne,
    Plus,
    Minus,
    LParen,
    RParen,
}

fn tokenize(line: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            ' ' | '\t' => i += 1,
            '(' => {
                toks.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                toks.push(Tok::RParen);
                i += 1;
            }
            '+' => {
                toks.push(Tok::Plus);
                i += 1;
            }
            '-' => {
                toks.push(Tok::Minus);
                i += 1;
            }
            '<' if line[i..].starts_with("<=") => {
                toks.push(Tok::Le);
                i += 2;
            }
            '>' if line[i..].starts_with(">=") => {
                toks.push(Tok::Ge);
                i += 2;
            }
            '=' if line[i..].starts_with("==") || line[i..].starts_with("=") => {
                let eq_len = if line[i..].starts_with("==") { 2 } else { 1 };
                toks.push(Tok::Eq);
                i += eq_len;
            }
            '!' if line[i..].starts_with("!=") => {
                toks.push(Tok::Ne);
                i += 2;
            }
            '<' => {
                toks.push(Tok::Lt);
                i += 1;
            }
            '>' => {
                toks.push(Tok::Gt);
                i += 1;
            }
            _ if c.is_ascii_digit() => {
                let start = i;
                let mut seen_dot = false;
                while i < bytes.len()
                    && (bytes[i].is_ascii_digit() || (bytes[i] == b'.' && !seen_dot))
                {
                    if bytes[i] == b'.' {
                        seen_dot = true;
                    }
                    i += 1;
                }
                // optional unit suffix (letters only)
                let unit_start = i;
                while i < bytes.len() && (bytes[i] as char).is_ascii_alphabetic() {
                    i += 1;
                }
                let value = line[start..i]
                    .trim_end_matches(|ch: char| ch.is_ascii_alphabetic())
                    .parse::<f64>()
                    .map_err(|e| format!("invalid number '{}': {e}", &line[start..i]))?;
                let unit = line[unit_start..i].to_string();
                toks.push(Tok::Num(value, unit));
            }
            _ if c.is_ascii_lowercase() || c == '_' => {
                let start = i;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch.is_ascii_lowercase() || ch == '_' || ch.is_ascii_digit() {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let word = &line[start..i];
                match word {
                    "and" => toks.push(Tok::And),
                    "or" => toks.push(Tok::Or),
                    "not" => toks.push(Tok::Not),
                    _ => toks.push(Tok::Var(word.to_string())),
                }
            }
            _ => return Err(format!("unexpected character '{c}'")),
        }
    }
    Ok(toks)
}

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Var(String),
    Cmp(CmpOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CmpOp {
    Le,
    Ge,
    Lt,
    Gt,
    Eq,
    Ne,
}

impl CmpOp {
    fn as_smt(self) -> &'static str {
        match self {
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Eq => "=",
            Self::Ne => "distinct",
        }
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn new(toks: Vec<Tok>) -> Self {
        Self { toks, pos: 0 }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// or (lowest) → and → comparison → additive → unary → primary
    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_and()?;
        while self.peek() == Some(&Tok::Or) {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_cmp()?;
        while self.peek() == Some(&Tok::And) {
            self.bump();
            let rhs = self.parse_cmp()?;
            lhs = Expr::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_add()?;
        let op = match self.peek() {
            Some(Tok::Le) => CmpOp::Le,
            Some(Tok::Ge) => CmpOp::Ge,
            Some(Tok::Lt) => CmpOp::Lt,
            Some(Tok::Gt) => CmpOp::Gt,
            Some(Tok::Eq) => CmpOp::Eq,
            Some(Tok::Ne) => CmpOp::Ne,
            _ => return Ok(lhs), // bare term: allowed as a top-level assertion
        };
        self.bump();
        let rhs = self.parse_add()?;
        Ok(Expr::Cmp(op, Box::new(lhs), Box::new(rhs)))
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => {
                    self.bump();
                    let rhs = self.parse_unary()?;
                    lhs = Expr::Add(Box::new(lhs), Box::new(rhs));
                }
                Some(Tok::Minus) => {
                    self.bump();
                    let rhs = self.parse_unary()?;
                    lhs = Expr::Sub(Box::new(lhs), Box::new(rhs));
                }
                _ => return Ok(lhs),
            }
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Tok::Minus) => {
                self.bump();
                let inner = self.parse_unary()?;
                Ok(Expr::Neg(Box::new(inner)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Some(Tok::Num(v, _unit)) => Ok(Expr::Num(v)),
            Some(Tok::Var(name)) => Ok(Expr::Var(name)),
            Some(Tok::LParen) => {
                let inner = self.parse_or()?;
                match self.bump() {
                    Some(Tok::RParen) => Ok(inner),
                    _ => Err("expected ')'".to_string()),
                }
            }
            other => Err(format!("unexpected token {other:?}")),
        }
    }
}

fn parse_line_expr(line: &str) -> Result<Expr, String> {
    let mut parser = Parser::new(tokenize(line)?);
    let expr = parser.parse_or()?;
    if parser.pos != parser.toks.len() {
        return Err("trailing tokens after expression".to_string());
    }
    Ok(expr)
}

// ── SMT-LIB2 encoding ───────────────────────────────────────────────────────

fn collect_vars(expr: &Expr, vars: &mut Vec<String>) {
    match expr {
        Expr::Num(_) => {}
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

fn encode_expr(expr: &Expr) -> String {
    match expr {
        Expr::Num(v) => {
            if v.fract() == 0.0 {
                format!("{v:.1}")
            } else {
                format!("{v}")
            }
        }
        Expr::Var(name) => name.clone(),
        Expr::Cmp(op, l, r) => {
            format!("({} {} {})", op.as_smt(), encode_expr(l), encode_expr(r))
        }
        Expr::And(l, r) => format!("(and {} {})", encode_expr(l), encode_expr(r)),
        Expr::Or(l, r) => format!("(or {} {})", encode_expr(l), encode_expr(r)),
        Expr::Add(l, r) => format!("(+ {} {})", encode_expr(l), encode_expr(r)),
        Expr::Sub(l, r) => format!("(- {} {})", encode_expr(l), encode_expr(r)),
        Expr::Neg(e) => format!("(- {})", encode_expr(e)),
    }
}

fn encode_formula(parts: &[Expr]) -> Option<String> {
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

/// Run the prove pass: parse every constraint's metric lines into formulas,
/// conjoin all formulas corpus-wide, and check the conjunction with z3. An
/// unsat result carries an unsat core naming the minimal set of constraints
/// whose bounds contradict each other (E046).
pub fn run_prove(ctx: &AnalysisContext) -> ProveReport {
    let mut findings = Vec::new();
    let mut skipped_prose_lines = 0usize;
    let mut constraints_with_bounds = 0usize;
    let mut formula_constraints: Vec<(String, SourceSpan, Vec<Expr>)> = Vec::new();
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

        let mut parts: Vec<Expr> = Vec::new();
        for line in metric.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match parse_line_expr(line) {
                Ok(expr) => parts.push(expr),
                Err(_) => skipped_prose_lines += 1,
            }
        }
        if !parts.is_empty() {
            formula_constraints.push((node.id.raw.to_string(), node.source_span.clone(), parts));
        }
    }

    let mut satisfiable = false;
    let mut unsat = false;

    if solver_available && !formula_constraints.is_empty() {
        let mut vars: Vec<String> = Vec::new();
        for (_, _, parts) in &formula_constraints {
            for part in parts {
                collect_vars(part, &mut vars);
            }
        }

        let mut script = String::from("(set-option :produce-unsat-cores true)\n");
        for var in &vars {
            script.push_str(&format!("(declare-const {var} Real)\n"));
        }
        for (idx, (id, _, parts)) in formula_constraints.iter().enumerate() {
            let formula = encode_formula(parts).unwrap_or_else(|| "true".to_string());
            let safe_id = id.replace(['/', '@'], "_");
            script.push_str(&format!("(assert (! {formula} :named c{idx}_{safe_id}))\n"));
        }
        script.push_str("(check-sat)\n");
        script.push_str("(get-unsat-core)\n");

        match run_z3(&script) {
            Some((result, core)) if result == "unsat" => {
                unsat = true;
                let core_constraints: Vec<String> = core
                    .iter()
                    .filter_map(|name| {
                        // core names look like c{idx}_{safe_id}; map back by index
                        formula_constraints
                            .iter()
                            .enumerate()
                            .find(|(pos, (id, _, _))| {
                                let safe = id.replace(['/', '@'], "_");
                                name == &format!("c{pos}_{safe}")
                            })
                            .map(|(pos, (id, _, _))| {
                                let _ = pos;
                                id.clone()
                            })
                    })
                    .collect();
                findings.push(
                    Diagnostic::error(
                        "E046",
                        format!(
                            "constraint bounds are mutually contradictory across: {}",
                            core_constraints.join(", ")
                        ),
                    )
                    .with_suggestion(
                        "relax or correct the metric bounds of the listed constraints",
                    ),
                );
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
        "constraints_with_metrics": constraints_with_bounds,
        "formulas": formula_constraints.len(),
        "skipped_prose_lines": skipped_prose_lines,
        "satisfiable": satisfiable,
        "unsatisfiable": unsat,
    });
    ProveReport { findings, summary }
}
