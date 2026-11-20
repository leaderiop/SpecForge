//! The measures ADR 0379 defines, computed from what `specforge` prints and never typed: verdict freshness at the ship
//! gate. A child of `harness.rs`, so every crate that includes the harness (core's tests and `@gsd/security`'s) shares
//! them.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::Project;

/// A verdict as the measures key it: `(rule, entity)`, the rule qualified (`gsd/E501`, `gsd.security/W302`).
pub type Verdict = (String, String);

/// Verdict freshness at the ship gate of a phase: the judgements the gate reads, the fresh ones among them, and what
/// its `gsd/E516` refuses.
#[derive(Debug)]
pub struct Freshness {
    /// Every judgement the graph lists, except a `missing` one outside the phase (E516's scope for a phase subject).
    pub required: BTreeSet<Verdict>,
    /// The required judgements whose state is `fresh`.
    pub fresh: BTreeSet<Verdict>,
    /// Each judgement a `gsd/E516` diagnostic of the refused ship names, with what the diagnostic says of it after
    /// the entity (`is missing`, `is stale: … changed`).
    pub refused: BTreeMap<Verdict, String>,
    /// Whether the ship gate's `gsd/E516` entry passes; a ship that renders passes every gate.
    pub gate_passes: bool,
}

impl Freshness {
    /// The judgements the gate reads, counted. A measure over nothing is not a measure: an empty set panics.
    fn denominator(&self) -> usize {
        assert!(
            !self.required.is_empty(),
            "the ship gate reads no judgement: {self:#?}"
        );
        self.required.len()
    }

    /// Fresh over required.
    pub fn ratio(&self) -> f64 {
        self.fresh.len() as f64 / self.denominator() as f64
    }

    /// The share in whole percent, rounded down: 100 only when every judgement is fresh.
    pub fn percent(&self) -> usize {
        self.fresh.len() * 100 / self.denominator()
    }

    /// The judgements the gate reads that are not fresh.
    pub fn not_fresh(&self) -> BTreeSet<Verdict> {
        self.required.difference(&self.fresh).cloned().collect()
    }

    /// The refused judgements, without what the diagnostics say of them.
    pub fn refused_set(&self) -> BTreeSet<Verdict> {
        self.refused.keys().cloned().collect()
    }

    /// The printed value: `verdict freshness at ship on the <twin> twin: <fresh> of <required> fresh (<percent>%)`.
    pub fn line(&self, twin: &str) -> String {
        format!(
            "verdict freshness at ship on the {twin} twin: {} of {} fresh ({}%)",
            self.fresh.len(),
            self.required.len(),
            self.percent()
        )
    }
}

/// `(rule, entity)` and what follows the entity, from a `gsd/E516` message: `the <rule> verdict on '<entity>' <rest>`.
fn e516_verdict(message: &str) -> (Verdict, String) {
    let parsed = message.strip_prefix("the ").and_then(|rest| {
        let (rule, rest) = rest.split_once(" verdict on '")?;
        let (entity, rest) = rest.split_once("' ")?;
        Some(((rule.to_string(), entity.to_string()), rest.to_string()))
    });
    parsed.unwrap_or_else(|| panic!("not a gsd/E516 verdict message: {message}"))
}

impl Project {
    /// Verdict freshness at the ship gate of `phase` (`v1.0/01`), read from `specforge export --format graph` (the
    /// judgements on its nodes) and `specforge prompt gsd.ship --format json -- <phase>` (exit 0: it renders; exit 1:
    /// the `gate_failed` answer, its `gsd/E516` diagnostics and gate entry). A refusal by another gate (gsd/E515 on a
    /// twin that records no commit) leaves the measure to E516's own entry and diagnostics.
    pub fn ship_freshness(&self, phase: &str) -> Freshness {
        let run = self.run(&["export", "--format", "graph"]);
        let graph: Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("export prints JSON ({e}): {}", run.all()));
        let scoped = format!("{phase}/");
        let mut required = BTreeSet::new();
        let mut fresh = BTreeSet::new();
        for node in graph["nodes"].as_array().expect("nodes") {
            let id = node["id"].as_str().expect("an id");
            for judgement in node["judgements"].as_array().into_iter().flatten() {
                let state = judgement["state"].as_str().expect("a state");
                if state == "missing" && id != phase && !id.starts_with(&scoped) {
                    continue;
                }
                let verdict = (
                    judgement["rule"].as_str().expect("a rule").to_string(),
                    id.to_string(),
                );
                if state == "fresh" {
                    fresh.insert(verdict.clone());
                }
                required.insert(verdict);
            }
        }
        let run = self.run(&[
            "prompt",
            "gsd.ship",
            "--host",
            "claude-code",
            "--format",
            "json",
            "--",
            phase,
        ]);
        let (refused, gate_passes) = match run.code {
            0 => (BTreeMap::new(), true),
            1 => refused_by_e516(&run.all()),
            _ => panic!("ship renders or refuses: {}", run.all()),
        };
        Freshness {
            required,
            fresh,
            refused,
            gate_passes,
        }
    }
}

/// The `gsd/E516` part of a refused prompt's `gate_failed` answer: each judgement a diagnostic names (one diagnostic
/// each: a second on the same judgement panics), and whether the gate's entry passes.
fn refused_by_e516(all: &str) -> (BTreeMap<Verdict, String>, bool) {
    let start = all
        .find("{\"code\"")
        .unwrap_or_else(|| panic!("no JSON in: {all}"));
    let answer: Value =
        serde_json::from_str(&all[start..]).unwrap_or_else(|e| panic!("{e}: {all}"));
    assert_eq!(answer["code"], "gate_failed", "{all}");
    let mut refused = BTreeMap::new();
    for diagnostic in answer["data"]["failing"].as_array().expect("failing") {
        if diagnostic["code"] != "gsd/E516" {
            continue;
        }
        let (verdict, says) = e516_verdict(diagnostic["message"].as_str().expect("a message"));
        assert!(
            refused.insert(verdict.clone(), says).is_none(),
            "two gsd/E516 diagnostics on {verdict:?}: {all}"
        );
    }
    let gate = answer["data"]["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|g| g["code"] == "gsd/E516")
        .unwrap_or_else(|| panic!("ship has no gsd/E516 gate (is the verdict gate on?): {all}"));
    (refused, gate["passes"].as_bool().expect("passes"))
}
