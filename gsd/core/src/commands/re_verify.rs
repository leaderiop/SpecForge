//! `re_verify <subject> [--stale-only]`: the verifier's verdicts within a phase or a plan, and whether each still
//! holds, so that the verifier judges exactly what no longer does (ADR 0377).
//!
//! For each rule `gsd-verifier` judges (`gsd/E501` on a truth, an artifact or a key link, `gsd/W525` on a
//! prohibition), each entity of a target kind whose plan is the subject or a plan of the subject phase (by the `phase`
//! edge, never an id prefix), and each of its judgements of the rule: `{entity, kind, rule, state, outcome?, at?,
//! changed: [{input, selector}], files: [{path, digest}], cited_tests}`. A changed input is named with the selector
//! the rule's target reads it with; the verdict's own citations (`cited`, `cited_tests`) are their own selector, and
//! what they cite is in `files` and `cited_tests`. `--stale-only` keeps the judgements that are not fresh (stale,
//! missing, unrecorded, unreadable): a fresh verdict, passing or failing, would be judged the same again. A command's
//! graph carries the settled judgements only, so an entity that owes no verdict yet (its plan is not complete) lists
//! nothing. The command reads; it declares no write.

use std::collections::BTreeSet;

use gsd_kit::judged::Target;
use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, required, run, target};

/// The judge whose verdicts the command lists.
const VERIFIER: &str = "gsd-verifier";

/// The phase or the plan `id` names: its full path, or the end of one no other phase or plan shares.
fn subject<'g>(graph: &'g CommandGraph, id: &str) -> Result<&'g GraphNode, String> {
    let kinds = |n: &&GraphNode| matches!(n.kind.as_str(), "phase" | "plan");
    if let Some(found) = graph.node(id).filter(kinds) {
        return Ok(found);
    }
    let tail = format!("/{id}");
    let found: Vec<&GraphNode> = graph
        .nodes()
        .iter()
        .filter(kinds)
        .filter(|n| n.id.ends_with(&tail))
        .collect();
    match found.as_slice() {
        [one] => Ok(one),
        [] => Err(format!("no phase or plan '{id}'")),
        many => Err(format!(
            "'{id}' is ambiguous: {}",
            many.iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The plans of `subject`: the plan itself, or the plans whose `phase` edge resolves to the phase.
fn plans<'g>(graph: &'g CommandGraph, subject: &'g GraphNode) -> BTreeSet<&'g str> {
    if subject.kind == "plan" {
        return BTreeSet::from([subject.id.as_str()]);
    }
    graph
        .nodes_of_kind("plan")
        .filter(|p| target(graph, &p.id, "phase") == Some(subject.id.as_str()))
        .map(|p| p.id.as_str())
        .collect()
}

/// The selector `target` reads the input `name` with; a verdict's citations (`cited`, `cited_tests`) and an input the
/// target no longer names are their own.
fn selector<'a>(target: &'a Target, name: &'a str) -> &'a str {
    target
        .inputs
        .iter()
        .find(|(input, _)| *input == name)
        .map_or(name, |(_, selector)| selector)
}

/// The answer over `graph` for the phase or plan `subject`: `{subject, items, holding, not_holding}`, where `holding`
/// and `not_holding` count every verdict of the subject (fresh, and not), whatever `stale_only` keeps in `items`.
pub fn re_verify(graph: &CommandGraph, subject: &str, stale_only: bool) -> Result<Value, String> {
    let subject = self::subject(graph, subject)?;
    let plans = plans(graph, subject);
    let (mut items, mut holding, mut not_holding) = (Vec::new(), 0, 0);
    for rule in crate::judged::JUDGED.iter().filter(|j| j.judge == VERIFIER) {
        let code = format!("gsd/{}", rule.code);
        for t in rule.targets {
            let entities = graph
                .nodes_of_kind(t.kind)
                .filter(|n| target(graph, &n.id, "plan").is_some_and(|p| plans.contains(p)));
            for node in entities {
                for j in node.judgements.iter().filter(|j| j.rule == code) {
                    let fresh = j.state == "fresh";
                    if fresh {
                        holding += 1;
                    } else {
                        not_holding += 1;
                    }
                    if stale_only && fresh {
                        continue;
                    }
                    let changed: Vec<Value> = j
                        .changed
                        .iter()
                        .map(|input| json!({ "input": input, "selector": selector(t, input) }))
                        .collect();
                    let files: Vec<Value> = j
                        .files
                        .iter()
                        .map(|f| json!({ "path": f.path, "digest": f.digest }))
                        .collect();
                    let mut item = json!({
                        "entity": node.id,
                        "kind": node.kind,
                        "rule": j.rule,
                        "state": j.state,
                        "changed": changed,
                        "files": files,
                        "cited_tests": j.cited_tests,
                    });
                    if let Some(outcome) = &j.outcome {
                        item["outcome"] = json!(outcome);
                    }
                    if let Some(at) = &j.at {
                        item["at"] = json!(at);
                    }
                    items.push(item);
                }
            }
        }
    }
    let key = |i: &Value| {
        let text = |k: &str| i[k].as_str().unwrap_or_default().to_string();
        (text("entity"), text("rule"))
    };
    items.sort_by_key(key);
    Ok(json!({
        "subject": subject.id,
        "items": items,
        "holding": holding,
        "not_holding": not_holding,
    }))
}

/// The text form: one line per item (`<rule> <entity> <state>`, and for a stale one what changed), then a count.
fn human(report: &Value) -> String {
    let mut out = String::new();
    for item in report["items"].as_array().into_iter().flatten() {
        let text = |key: &str| item[key].as_str().unwrap_or_default().to_string();
        out.push_str(&format!(
            "{} {} {}",
            text("rule"),
            text("entity"),
            text("state")
        ));
        let changed: Vec<String> = item["changed"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| {
                let part = |key: &str| c[key].as_str().unwrap_or_default().to_string();
                format!("{} ({})", part("input"), part("selector"))
            })
            .collect();
        if !changed.is_empty() {
            out.push_str(&format!(": {}", changed.join(", ")));
        }
        out.push('\n');
    }
    let count = |key: &str| report[key].as_u64().unwrap_or_default();
    let (holding, not_holding) = (count("holding"), count("not_holding"));
    let subject = report["subject"].as_str().unwrap_or_default();
    out.push_str(&match (holding, not_holding) {
        (0, 0) => format!("no verdict is owed in {subject}\n"),
        (_, 0) => format!("every verdict holds ({holding})\n"),
        _ => format!(
            "{not_holding} of {} verdict(s) no longer hold\n",
            holding + not_holding
        ),
    });
    out
}

fn re_verify_command(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let subject = required(call, "subject")?;
    let report = re_verify(call.graph(), subject, call.flag("stale_only"))?;
    let human = human(&report);
    Ok(answer(call, report, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("re_verify", |cmd| {
        cmd.description("List the verifier's verdicts within a phase or a plan and whether each still holds: its state, each input that changed since it was recorded with its selector, and what it cites. `specforge gsd re-verify <subject> --stale-only` keeps the verdicts that no longer hold, so the verifier judges exactly those")
            .arg("subject", |a| {
                a.required()
                    .description("the phase or the plan whose verdicts to list");
            })
            .arg("stale_only", |a| {
                a.flag().description(
                    "list only the verdicts that are not fresh: stale, missing, unrecorded or unreadable",
                );
            })
            .handler(|call| run(call, re_verify_command));
    });
}

#[cfg(test)]
mod tests;
