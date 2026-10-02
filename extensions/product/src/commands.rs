//! The `specforge product <command>` commands: `cmd__product_*` exports the
//! manifest declares in `describe_surfaces.json`. Each reads its args, runs
//! a query over the graph the host passes and renders it: `--format human`
//! (the default) or `json`. The same exports serve the MCP tools
//! `specforge.product.<id>`.

use crate::queries::{self, ListFilter};
use serde::Serialize;
use specforge_extension_sdk::prelude::{CommandInput, CommandOutput};
use std::fmt::Write as _;

/// Run the command behind `export`; `None` when no command has it.
pub fn run(export: &str, input: &CommandInput) -> Option<CommandOutput> {
    let list = |kind| list(input, kind);
    Some(match export {
        "cmd__product_features" => list("feature"),
        "cmd__product_journeys" => list("journey"),
        "cmd__product_deliverables" => list("deliverable"),
        "cmd__product_milestones" => list("milestone"),
        "cmd__product_modules" => list("module"),
        "cmd__product_terms" => list("term"),
        "cmd__product_personas" => list("persona"),
        "cmd__product_channels" => list("channel"),
        "cmd__product_releases" => list("release"),
        "cmd__product_milestone_completion" => lookup(
            input,
            "milestone",
            queries::milestone_completion,
            |r, out| {
                let _ = writeln!(
                    out,
                    "Milestone: {} ({})",
                    r.milestone_id,
                    r.status.as_deref().unwrap_or("-")
                );
                let _ = writeln!(
                    out,
                    "Completion: {:.0}% ({}/{} features done)",
                    r.completion_pct, r.done_features, r.total_features
                );
                for f in &r.features {
                    let _ = writeln!(out, "  {} [{}]", f.id, f.status.as_deref().unwrap_or("-"));
                }
            },
        ),
        "cmd__product_journey_coverage" => {
            lookup(input, "journey", queries::journey_coverage, |r, out| {
                let _ = writeln!(
                    out,
                    "Journey: {} (persona: {})",
                    r.journey_id,
                    r.persona.as_deref().unwrap_or("-")
                );
                let _ = writeln!(
                    out,
                    "Coverage: {:.0}% ({}/{} features covered by modules)",
                    r.coverage_pct, r.covered_by_modules, r.total_features
                );
            })
        }
        "cmd__product_feature_impact" => {
            lookup(input, "feature", queries::feature_impact, |r, out| {
                let _ = writeln!(out, "Feature: {}", r.feature_id);
                let _ = writeln!(out, "  Journeys: {:?}", r.referenced_by_journeys);
                let _ = writeln!(out, "  Milestones: {:?}", r.referenced_by_milestones);
                let _ = writeln!(out, "  Modules: {:?}", r.referenced_by_modules);
                let _ = writeln!(out, "  Depends on: {:?}", r.depends_on);
                let _ = writeln!(out, "  Depended on by: {:?}", r.depended_on_by);
            })
        }
        "cmd__product_feature_dependents" => id_list(
            input,
            "feature",
            "Features depending on",
            queries::feature_dependents,
        ),
        "cmd__product_persona_features" => id_list(
            input,
            "persona",
            "Features for persona",
            queries::persona_features,
        ),
        "cmd__product_channel_features" => id_list(
            input,
            "channel",
            "Features for channel",
            queries::channel_features,
        ),
        "cmd__product_bulk_status" => {
            let results = queries::bulk_status(&input.graph);
            render(input, &results, |out| {
                for bs in &results {
                    let _ = writeln!(out, "{} ({} total):", bs.kind, bs.total);
                    for sc in &bs.by_status {
                        let _ = writeln!(out, "  {}: {}", sc.status, sc.count);
                    }
                }
                if results.is_empty() {
                    let _ = writeln!(out, "No status-bearing entities found.");
                }
            })
        }
        "cmd__product_health" => {
            let report = queries::project_health(&input.graph);
            render(input, &report, |out| health(&report, out))
        }
        _ => return None,
    })
}

/// A list command over `kind`, with the filters its args set.
fn list(input: &CommandInput, kind: &str) -> CommandOutput {
    let filter = ListFilter {
        kind,
        status: input.arg_str("status"),
        priority: input.arg_str("priority"),
        limit: input.arg_usize("limit"),
        offset: input.arg_usize("offset"),
    };
    let result = queries::list_entities(&input.graph, &filter);
    render(input, &result, |out| {
        let _ = writeln!(
            out,
            "{} {} entities (showing {}):",
            result.total,
            kind,
            result.entities.len()
        );
        for e in &result.entities {
            let _ = writeln!(
                out,
                "  {} {} [{}] pri={} in={} out={}",
                e.id,
                e.title.as_deref().unwrap_or(""),
                e.status.as_deref().unwrap_or("-"),
                e.priority.as_deref().unwrap_or("-"),
                e.incoming_edges,
                e.outgoing_edges
            );
        }
    })
}

/// A query about the entity the `kind` arg names: its result rendered, or
/// `<kind> '<id>' not found` on stderr with exit code 1.
fn lookup<T: Serialize>(
    input: &CommandInput,
    kind: &str,
    query: fn(&specforge_extension_sdk::prelude::CommandGraph, &str) -> Option<T>,
    human: impl FnOnce(&T, &mut String),
) -> CommandOutput {
    let id = input.arg_str(kind).unwrap_or_default();
    match query(&input.graph, id) {
        Some(result) => render(input, &result, |out| human(&result, out)),
        None => CommandOutput::fail(format!("{kind} '{id}' not found\n")),
    }
}

/// A query returning entity ids, printed one per line under `heading`.
fn id_list(
    input: &CommandInput,
    kind: &str,
    heading: &str,
    query: fn(&specforge_extension_sdk::prelude::CommandGraph, &str) -> Option<Vec<String>>,
) -> CommandOutput {
    let id = input.arg_str(kind).unwrap_or_default().to_string();
    lookup(input, kind, query, |ids, out| {
        let _ = writeln!(out, "{heading} '{id}':");
        for i in ids {
            let _ = writeln!(out, "  {i}");
        }
        if ids.is_empty() {
            let _ = writeln!(out, "  (none)");
        }
    })
}

fn health(report: &queries::HealthReport, out: &mut String) {
    let _ = writeln!(out, "Project Health Score: {:.0}/100", report.score.overall);
    let _ = writeln!(out, "  Coverage:     {:.0}%", report.score.coverage);
    let _ = writeln!(out, "  Connectivity: {:.0}%", report.score.connectivity);
    let _ = writeln!(out, "  Completeness: {:.0}%", report.score.completeness);
    let _ = writeln!(out);
    let _ = writeln!(out, "Entity counts:");
    for ec in report.entity_counts.iter().filter(|ec| ec.count > 0) {
        let _ = writeln!(out, "  {}: {}", ec.kind, ec.count);
    }
    if !report.orphan_counts.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "Orphan entities:");
        for oc in report.orphan_counts.iter().filter(|oc| oc.orphans > 0) {
            let _ = writeln!(out, "  {}: {}/{}", oc.kind, oc.orphans, oc.total);
        }
    }
}

/// `result` as pretty JSON under `--format json`, else as `human` writes it.
fn render<T: Serialize>(
    input: &CommandInput,
    result: &T,
    human: impl FnOnce(&mut String),
) -> CommandOutput {
    let mut out = String::new();
    if input.arg_str("format") == Some("json") {
        out = serde_json::to_string_pretty(result).unwrap_or_default();
        out.push('\n');
    } else {
        human(&mut out);
    }
    CommandOutput::ok(out)
}
