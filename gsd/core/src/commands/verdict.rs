//! `record_verdict`: writes a judge's verdict on a judged `gsd/` rule (K7), and nothing else. A prompt cannot write,
//! and an agent calls a command: this is the command. The judge is the rule's own (`crate::judged`), never an
//! argument, so a caller cannot name another one; the host checks it again.

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::{Cite, Verdict};

use crate::judged::{self, JUDGED};
use crate::prompts::GATES;

const INVALID: &str = "invalid_input";

/// The optional JSON arg `name` as `T`, `default` when it is not given.
fn json<T>(
    call: &CommandCall<'_>,
    name: &str,
    default: T,
    parse: impl FnOnce(&str) -> serde_json::Result<T>,
) -> Result<T, String> {
    match call.str(name) {
        None => Ok(default),
        Some(text) => parse(text).map_err(|e| format!("{name}: not the JSON it takes: {e}")),
    }
}

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

fn record(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let arg = |name: &str| {
        call.str(name)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| format!("{name} is required"))
    };
    let (entity, code, outcome, model) =
        (arg("entity")?, arg("rule")?, arg("outcome")?, arg("model")?);
    let Some(rule) = judged::get(code) else {
        return Err(format!("rule: '{code}' is no judged rule of @gsd/core"));
    };
    if !rule.outcomes.contains(&outcome) {
        return Err(format!(
            "outcome: '{outcome}' is no outcome of gsd/{code} ({})",
            rule.outcomes.join(", ")
        ));
    }
    let basis: BTreeMap<String, String> = serde_json::from_str(arg("basis")?)
        .map_err(|e| format!("basis is not the JSON object `specforge query <entity> --inputs gsd/{code}` printed: {e}"))?;
    let findings: Vec<Value> = json(call, "findings", Vec::new(), |t| serde_json::from_str(t))?;
    let cited: Vec<Cite> = json(call, "cited", Vec::new(), |t| serde_json::from_str(t))?;
    let values: BTreeMap<String, Value> =
        json(call, "values", BTreeMap::new(), |t| serde_json::from_str(t))?;
    if code == "E202" && outcome == "passed" {
        check_gates(&values)?;
    }
    let mut plan = call.plan();
    plan.set_verdict(Verdict {
        entity: entity.to_string(),
        rule: format!("gsd/{code}"),
        outcome: outcome.to_string(),
        judge: rule.judge.to_string(),
        model: model.to_string(),
        findings,
        cited,
        values,
        basis,
    });
    Ok(
        CommandOutput::ok(format!("verdict gsd/{code} {outcome} on {entity}\n"))
            .with_plan(plan.finish()),
    )
}

pub fn declare(c: &mut ContributionsBuilder) {
    let codes: Vec<&str> = JUDGED.iter().map(|j| j.code).collect();
    c.command("record_verdict", |cmd| {
        cmd.description("Record a judge's verdict on a judged gsd/ rule: read the inputs with `specforge query <entity> --inputs gsd/<code>`, judge, then record the outcome with that `basis`")
            .writes()
            .arg("entity", |a| {
                a.required().description("the entity judged (its path, or a short form that is not ambiguous)");
            })
            .arg("rule", |a| {
                a.required().one_of(&codes).description("the judged rule, as its local code");
            })
            .arg("outcome", |a| {
                a.required().description("the verdict's outcome, one of the rule's");
            })
            .arg("model", |a| {
                a.required().description("the model that judged");
            })
            .arg("basis", |a| {
                a.required().description("the JSON object `basis` of `specforge query <entity> --inputs gsd/<code> --format json`");
            })
            .arg("findings", |a| {
                a.description("JSON list of findings (default []): objects with severity and message");
            })
            .arg("cited", |a| {
                a.description("JSON list of what the verdict cites (default []): {\"entity\": ..}, {\"file\": ..} or {\"test\": ..}");
            })
            .arg("values", |a| {
                a.description("JSON object of the field values the rule binds (default {}), such as the scores of gsd/E202");
            })
            .handler(|call| match record(call) {
                Ok(out) => out,
                Err(why) => call.fail(&CommandError::new(INVALID, why), 1),
            });
    });
}
