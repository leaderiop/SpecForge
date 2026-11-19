//! `record_verdict`: writes a judge's verdict on a judged `gsd/` rule, built by `gsd_kit::judged::record` over
//! core's table (`crate::judged`). The judge is the rule's own, never an argument; core adds its per-rule checks.

use std::collections::BTreeMap;

use gsd_kit::judged::record::{Check, Proposed, Recorder, declare_record};
use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use crate::judged::JUDGED;
use crate::prompts::GATES;

/// A `passed` on `gsd/E202` needs the five scores, each 0..=100, and every one within its gate (the judge's own
/// numbers must show what it concluded). Without them the rule would be satisfied by a spec that fails its gates.
fn check_gates(values: &BTreeMap<String, Value>) -> Result<(), String> {
    let scores = values.get("scores").and_then(Value::as_object);
    let mut failing = Vec::new();
    for (name, bound, max) in GATES {
        let score = scores
            .and_then(|s| s.get(name))
            .and_then(Value::as_i64)
            .filter(|n| (0..=100).contains(n))
            .ok_or_else(|| format!("values: a `passed` on gsd/E202 needs values.scores.{name} as an integer in 0..=100"))?;
        if if max { score > bound } else { score < bound } {
            failing.push(format!(
                "{name} {score} (gate {} {bound})",
                if max { "<=" } else { ">=" }
            ));
        }
    }
    if failing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "values: gsd/E202 cannot be `passed` with scores outside the gates: {}; record `failed`",
            failing.join(", ")
        ))
    }
}

/// [`check_gates`] on a `passed` verdict of `gsd/E202`.
fn gates(verdict: &Proposed<'_>) -> Result<(), String> {
    if verdict.rule.code == "E202" && verdict.outcome == "passed" {
        check_gates(verdict.values)
    } else {
        Ok(())
    }
}

static RECORDER: Recorder = Recorder {
    command: "record_verdict",
    namespace: "gsd",
    extension: "@gsd/core",
    table: JUDGED,
    checks: &[gates as Check],
};

pub fn declare(c: &mut ContributionsBuilder) {
    declare_record(c, &RECORDER);
}
