//! Ported from gsd-core v1.19.0 src/edge-probe.cts (its CLI, runProbeCli) and gsd-core/workflows/spec-phase.md (the requirements guard), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `edge_probe <subject>`: the edge coverage of a phase spec or of a phase (ADR 0333). The requirements are the graph's
//! (a spec's locked requirements, a phase's `requirements`), the resolutions a spec's recorded `edge` rows, and the
//! proposal and roll-up are the probe engine's ([`crate::probe`]); upstream's CLI read both from JSON files. An edge
//! the engine did not propose (the agent raised it) is an item of the report, never an orphan.
//!
//! `edge_probe_requirements --requirements <json>`: whether the requirements an agent wrote for the edge probe are
//! fit to run it on, the guard `/gsd-spec-phase` and the specless probe fallback run before the probe: the text is a
//! non-empty JSON array, and every item has a string `text` that is not blank (JavaScript's `trim()`) and holds no
//! `<replace:` placeholder. Upstream's node also failed on a `null` item (reading its `text` throws); each failure is
//! named here, where node only exited 1.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, natural, node, required, run};
use super::user_story::js_space;
use crate::probe::edge::{
    EDGE_VALIDATORS, EdgeRequirement, Shape, TAXONOMY, UNCLASSIFIED_PROBE, propose_all,
};
use crate::probe::{Item, Resolution, analyze_coverage};

/// The verdict on `requirements`: `{valid, errors, count}`, `count` the items of an array.
pub fn verdict(requirements: &str) -> Value {
    let mut errors = Vec::new();
    let mut count = 0;
    match serde_json::from_str::<Value>(requirements) {
        Err(e) => errors.push(format!("the requirements are not JSON: {e}")),
        Ok(Value::Array(items)) if items.is_empty() => {
            errors.push("the requirements are an empty array".to_string())
        }
        Ok(Value::Array(items)) => {
            count = items.len();
            for (at, item) in items.iter().enumerate() {
                match item.get("text") {
                    Some(Value::String(text)) if text.trim_matches(js_space).is_empty() => {
                        errors.push(format!("requirement {at}: its text is blank"));
                    }
                    Some(Value::String(text)) if text.contains("<replace:") => {
                        errors.push(format!(
                            "requirement {at}: its text still holds the <replace: …> placeholder"
                        ));
                    }
                    Some(Value::String(_)) => {}
                    _ => errors.push(format!("requirement {at}: it has no string text")),
                }
            }
        }
        Ok(_) => errors.push("the requirements are not an array".to_string()),
    }
    json!({ "valid": errors.is_empty(), "errors": errors, "count": count })
}

fn edge_probe_requirements(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    // shortcut: the requirements come as an argument (a command reads no runtime path), so a list over the platform's
    // argument size (128 KiB on Linux) cannot be checked; read the file through the host when one exceeds it
    let verdict = verdict(call.str("requirements").unwrap_or(""));
    let human = match verdict["errors"].as_array() {
        Some(errors) if !errors.is_empty() => errors
            .iter()
            .filter_map(Value::as_str)
            .map(|e| format!("{e}\n"))
            .collect(),
        _ => format!("{} requirement(s), each with a text\n", verdict["count"]),
    };
    Ok(answer(call, verdict, human, None))
}

/// The resolved target of `id`'s reference field `label`.
fn target<'a>(graph: &'a CommandGraph, id: &str, label: &str) -> Option<&'a str> {
    graph
        .edges_from(id)
        .into_iter()
        .find(|e| e.label == label)
        .map(|e| e.target.as_str())
}

/// The requirements of a phase spec (its locked requirements, by id) or of a phase (its `requirements`, in field
/// order), as the edge probe reads them: a locked requirement's text is its title, current, target and acceptance,
/// a line each, as the SPEC shows it.
fn requirements(graph: &CommandGraph, subject: &GraphNode, spec: bool) -> Vec<EdgeRequirement> {
    let nodes: Vec<&GraphNode> = if spec {
        let mut reqs: Vec<&GraphNode> = graph
            .nodes_of_kind("locked_req")
            .filter(|r| target(graph, &r.id, "spec") == Some(subject.id.as_str()))
            .collect();
        reqs.sort_by_key(|r| natural(&r.id));
        reqs
    } else {
        graph
            .edges_from(&subject.id)
            .into_iter()
            .filter(|e| e.label == "requirements")
            .filter_map(|e| graph.node(&e.target))
            .collect()
    };
    nodes
        .into_iter()
        .map(|n| {
            let text = if spec {
                let parts: Vec<&str> = ["title", "current", "target", "acceptance"]
                    .into_iter()
                    .filter_map(|f| n.text(f))
                    .collect();
                Some(parts.join("\n"))
            } else {
                n.text("text").map(String::from)
            };
            EdgeRequirement {
                id: n.id.clone(),
                text,
                text_en: n.text("text_en").map(String::from),
                // a value the engine cannot map is passed on as written, for its refusal
                shapes: n.has_field("shapes").then(|| {
                    n.list("shapes")
                        .into_iter()
                        .map(|v| {
                            Shape::from_value(v)
                                .map_or_else(|| v.to_string(), |s| s.upstream().to_string())
                        })
                        .collect()
                }),
            }
        })
        .collect()
}

/// A recorded edge of the spec: its id and its resolution, keyed by its requirement and category.
fn recorded(graph: &CommandGraph, spec: &str) -> Vec<(String, Resolution)> {
    let mut edges: Vec<&GraphNode> = graph
        .nodes_of_kind("edge")
        .filter(|e| target(graph, &e.id, "spec") == Some(spec))
        .collect();
    edges.sort_by_key(|e| natural(&e.id));
    edges
        .into_iter()
        .map(|e| {
            let text = |f: &str| e.text(f).map(String::from);
            let resolution = Resolution {
                requirement_id: target(graph, &e.id, "requirement")
                    .unwrap_or_default()
                    .to_string(),
                category: text("category").unwrap_or_default(),
                status: text("status").unwrap_or_default(),
                verification: text("verification"),
                resolution: text("resolution"),
                reason: text("reason"),
            };
            (e.id.clone(), resolution)
        })
        .collect()
}

fn edge_probe(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let subject = required(call, "subject")?;
    let graph = call.graph();
    let (node, spec) = match node(call, "phase_spec", subject) {
        Ok(spec) => (spec, true),
        Err(_) => match node(call, "phase", subject) {
            Ok(phase) => (phase, false),
            Err(_) => return Err(format!("no phase spec or phase '{subject}'")),
        },
    };
    let reqs = requirements(graph, node, spec);
    if reqs.is_empty() {
        return Err(format!("{} has no requirements: nothing to probe", node.id));
    }
    let edges = if spec {
        recorded(graph, &node.id)
    } else {
        Vec::new()
    };
    let mut items = propose_all(&reqs)?;
    let proposed = items.len();
    // an edge the engine did not propose was raised by the agent: an item of the report, never an orphan
    for (_, r) in &edges {
        let held = items[..proposed]
            .iter()
            .any(|i| i.requirement_id == r.requirement_id && i.category == r.category);
        if !held {
            let probe = TAXONOMY
                .iter()
                .find(|c| c.id == r.category)
                .map_or(UNCLASSIFIED_PROBE, |c| c.probe);
            items.push(Item {
                requirement_id: r.requirement_id.clone(),
                category: r.category.clone(),
                status: "unresolved".to_string(),
                verification: None,
                resolution: None,
                reason: None,
                probe: probe.to_string(),
            });
        }
    }
    let resolutions: Vec<Resolution> = edges.iter().map(|(_, r)| r.clone()).collect();
    let report = analyze_coverage(items, &resolutions, &EDGE_VALIDATORS)?;
    let rows: Vec<Value> = report
        .items
        .iter()
        .enumerate()
        .map(|(at, item)| {
            let edge = edges
                .iter()
                .find(|(_, r)| {
                    r.requirement_id == item.requirement_id && r.category == item.category
                })
                .map(|(id, _)| id.as_str());
            let mut row = item.to_json();
            row["edge"] = json!(edge);
            row["proposed"] = json!(at < proposed);
            row
        })
        .collect();
    let c = &report.coverage;
    let mut human: String = report
        .items
        .iter()
        .map(|i| {
            format!(
                "{} {} {} — {}\n",
                i.requirement_id, i.category, i.status, i.probe
            )
        })
        .collect();
    human.push_str(&format!(
        "Coverage: {}/{} resolved · {} unresolved\n",
        c.resolved, c.applicable, c.unresolved
    ));
    // upstream's report, each item with its edge and whether the engine proposed it, and the subject
    let mut payload = report.to_json();
    payload["items"] = Value::Array(rows);
    payload["subject"] = json!(node.id);
    payload["kind"] = json!(if spec { "phase_spec" } else { "phase" });
    Ok(answer(call, payload, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("edge_probe", |cmd| {
        cmd.description("Propose and roll up a spec's edge coverage: the edge probe's rows for each locked requirement of a phase spec (or each requirement of a phase), merged with the spec's recorded edges; answers the items and the coverage (applicable, resolved, unresolved, unclassified, byVerification)")
            .arg("subject", |a| {
                a.required().description("the phase spec or the phase (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, edge_probe));
    });
    c.command("edge_probe_requirements", |cmd| {
        cmd.description("Check the requirements written for the edge probe: a non-empty JSON array whose every item has a string text that is not blank and holds no <replace: …> placeholder; answers valid, the errors and the count")
            .arg("requirements", |a| {
                a.description("the requirements' JSON text, `[{\"id\", \"text\", …}]`");
            })
            .handler(|call| run(call, edge_probe_requirements));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(requirements: &str) -> bool {
        verdict(requirements)["valid"] == true
    }

    #[test]
    fn a_list_of_texts_is_valid() {
        assert_eq!(
            verdict(
                r#"[{"id":"R1","text":"a user signs in","shapes":["auth"]},{"id":"R2","text":"x"}]"#
            ),
            json!({"valid": true, "errors": [], "count": 2})
        );
    }

    #[test]
    fn each_failure_of_upstreams_check_is_refused() {
        for bad in [
            "",
            "not json",
            "[]",
            r#"{"text":"a"}"#,
            r#""a""#,
            "[null]",
            r#"["a"]"#,
            r#"[{"id":"R1"}]"#,
            r#"[{"text":1}]"#,
            r#"[{"text":"  \n\t"}]"#,
            "[{\"text\":\"\u{feff}\u{3000}\"}]",
            r#"[{"text":"<replace: requirement text from the SPEC>"}]"#,
            r#"[{"text":"ok"},{"text":"see <replace: x>"}]"#,
        ] {
            assert!(!valid(bad), "{bad}");
        }
    }

    #[test]
    fn each_bad_item_is_named() {
        assert_eq!(
            verdict(r#"[{"text":"ok"},{"text":" "},{"id":"R3"}]"#)["errors"],
            json!([
                "requirement 1: its text is blank",
                "requirement 2: it has no string text"
            ])
        );
    }
}
