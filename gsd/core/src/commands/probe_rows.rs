//! `resolve_edge` and `resolve_prohibition`: a probe row's resolution, written whole in one plan (ADR 0333). gsd/E523
//! is an error, so the host refuses a typed edit that leaves a row half resolved (E080): two `specforge set` calls
//! cannot move an edge from unresolved to resolved, its status and its tier landing apart. Each command takes
//! upstream's resolution object (`status`, `verification`, `resolution`, `reason`), judges it with the probe engine
//! before anything is written, and writes the row so it holds exactly that resolution.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, next_id, node, required, run, text};
use crate::probe::edge::EDGE_VALIDATORS;
use crate::probe::{Resolution, validate_prohibition_resolution, validate_resolution};

/// The five scalars of a prohibition's check descriptor, which ride only a resolved `test` row.
const CHECK: [&str; 5] = [
    "check_kind",
    "check_target",
    "check_rule",
    "check_violation_fixture",
    "check_clean_fixture",
];

/// The resolved target of `id`'s reference field `label`.
fn target<'a>(graph: &'a CommandGraph, id: &str, label: &str) -> Option<&'a str> {
    graph
        .edges_from(id)
        .into_iter()
        .find(|e| e.label == label)
        .map(|e| e.target.as_str())
}

/// Sets each given field of `id` and unsets each absent one it holds, so the row holds exactly `values`.
fn write(plan: &mut PlanBuilder, row: &GraphNode, values: &[(&str, Option<&str>)]) {
    for (name, value) in values {
        match value {
            Some(value) => {
                plan.set(&row.id, name, json!(value));
            }
            None if row.has_field(name) => {
                plan.unset(&row.id, name);
            }
            None => {}
        }
    }
}

/// The row as the answer shows it.
fn shown(id: &str, values: &[(&str, Option<&str>)]) -> Value {
    let mut row = json!({ "id": id });
    for (name, value) in values {
        row[*name] = json!(value);
    }
    row
}

fn resolve_edge(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let requirement = node(call, "locked_req", required(call, "requirement")?)?;
    let category = required(call, "category")?;
    if !EDGE_VALIDATORS.categories.contains(&category) {
        return Err(format!(
            "unknown category \"{category}\" — not one of: {}",
            EDGE_VALIDATORS.categories.join(", ")
        ));
    }
    let resolution = Resolution {
        requirement_id: requirement.id.clone(),
        category: category.to_string(),
        status: required(call, "status")?.to_string(),
        verification: text(call, "verification")?.map(String::from),
        resolution: text(call, "resolution")?.map(String::from),
        reason: text(call, "reason")?.map(String::from),
    };
    validate_resolution(&resolution, &EDGE_VALIDATORS)?;
    let graph = call.graph();
    let spec = target(graph, &requirement.id, "spec")
        .ok_or_else(|| format!("locked requirement {} names no phase spec", requirement.id))?;
    let values = [
        ("status", Some(resolution.status.as_str())),
        ("verification", resolution.verification.as_deref()),
        ("resolution", resolution.resolution.as_deref()),
        ("reason", resolution.reason.as_deref()),
    ];
    let held = graph.nodes_of_kind("edge").find(|e| {
        target(graph, &e.id, "spec") == Some(spec)
            && target(graph, &e.id, "requirement") == Some(requirement.id.as_str())
            && e.text("category") == Some(category)
    });
    let mut plan = call.plan();
    let (id, created) = match held {
        Some(edge) => {
            write(&mut plan, edge, &values);
            (edge.id.clone(), false)
        }
        None => {
            let name = next_id(graph, "edge", &format!("{spec}/"), "EC");
            let file = graph
                .node(spec)
                .and_then(|s| s.file.clone())
                .ok_or_else(|| format!("phase spec {spec} has no file"))?;
            let short = requirement.id.rsplit('/').next().unwrap_or(&requirement.id);
            plan.create(
                "edge",
                &name,
                &format!("{short} {category}"),
                &file,
                fields([
                    ("spec", json!(spec)),
                    ("requirement", json!(requirement.id)),
                    ("category", json!(category)),
                    ("status", json!(resolution.status)),
                    ("verification", json!(resolution.verification)),
                    ("resolution", json!(resolution.resolution)),
                    ("reason", json!(resolution.reason)),
                ]),
                Vec::new(),
            );
            (format!("{spec}/{name}"), true)
        }
    };
    let mut edge = shown(&id, &values);
    edge["requirement"] = json!(requirement.id);
    edge["category"] = json!(category);
    Ok(answer(
        call,
        json!({ "wrote": [id], "created": created, "edge": edge }),
        format!("{id} {} {category} {}\n", requirement.id, resolution.status),
        Some(&plan),
    ))
}

fn resolve_prohibition(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let row = node(call, "spec_prohibition", required(call, "prohibition")?)?;
    let resolution = Resolution {
        requirement_id: row.id.clone(),
        category: String::new(),
        status: required(call, "status")?.to_string(),
        verification: text(call, "verification")?.map(String::from),
        resolution: None,
        reason: text(call, "reason")?.map(String::from),
    };
    // upstream keys a row `<requirement>::<category>`; the row's own id names it here
    validate_prohibition_resolution(&resolution)
        .map_err(|why| why.replace(&format!("{}::", row.id), &row.id))?;
    let values = [
        ("status", Some(resolution.status.as_str())),
        ("verification", resolution.verification.as_deref()),
        ("reason", resolution.reason.as_deref()),
    ];
    let mut plan = call.plan();
    write(&mut plan, row, &values);
    let tested =
        resolution.status == "resolved" && resolution.verification.as_deref() == Some("test");
    if !tested {
        for name in CHECK.into_iter().filter(|f| row.has_field(f)) {
            plan.unset(&row.id, name);
        }
    }
    Ok(answer(
        call,
        json!({ "wrote": [row.id], "created": false, "prohibition": shown(&row.id, &values) }),
        format!("{} {}\n", row.id, resolution.status),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("resolve_edge", |cmd| {
        cmd.description("Write an edge-coverage row's resolution in one edit: the edge of a locked requirement and a category (created in the requirement's spec when none holds the pair) gets the status, tier, resolution and reason given, and only those; refused before anything is written when the probe engine refuses the resolution")
            .writes()
            .arg("requirement", |a| {
                a.required().description("the locked requirement (its path, or a short form that is not ambiguous)");
            })
            .arg("category", |a| {
                a.required().description("the edge category: one of the taxonomy's eight, or unclassified");
            })
            .arg("status", |a| {
                a.required().description("resolved, dismissed or unresolved");
            })
            .arg("verification", |a| {
                a.description("a resolved row's tier: explicit or backstop");
            })
            .arg("resolution", |a| {
                a.description("the acceptance criterion or backstop note a resolved row requires");
            })
            .arg("reason", |a| {
                a.description("why a dismissed row is dismissed");
            })
            .handler(|call| run(call, resolve_edge));
    });
    c.command("resolve_prohibition", |cmd| {
        cmd.description("Write a spec prohibition's resolution in one edit: its status, tier and reason, and only those; the check descriptor (check_*) is cleared unless the row is resolved with the test tier; refused before anything is written when the probe engine refuses the resolution")
            .writes()
            .arg("prohibition", |a| {
                a.required().description("the spec prohibition (its path, or a short form that is not ambiguous)");
            })
            .arg("status", |a| {
                a.required().description("resolved, dismissed or unresolved");
            })
            .arg("verification", |a| {
                a.description("a resolved row's tier: test or judgment");
            })
            .arg("reason", |a| {
                a.description("why a dismissed row is dismissed");
            })
            .handler(|call| run(call, resolve_prohibition));
    });
}
