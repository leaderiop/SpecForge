//! The `specforge product <command>` commands: `cmd__product_*` exports the
//! manifest declares in `describe_surfaces.json`. Each reads its args, runs
//! a query over the graph the host passes and renders it in the format the
//! host asked for (`input.format`: `human`, the CLI default, or `json`,
//! always over MCP; ADR 0011). The same exports serve the MCP tools
//! `specforge.product.<id>`.
//!
//! Under `json` a command prints one object, the payload type its surface
//! behavior names; under `human` its layout here, a table with a header row
//! where the payload is tabular. A command that cannot answer prints one
//! `ProductSurfaceError` to stderr and nothing to stdout: `ENTITY_NOT_FOUND`
//! (exit 1) with the nearest id of the kind, or `INVALID_INPUT` (exit 2).

use crate::queries::{self, ListEntry, ListFilter, ListKind};
use serde::Serialize;
use specforge_extension_sdk::prelude::{CommandError, CommandGraph, CommandInput, CommandOutput};
use std::fmt::Write as _;

/// Run the command behind `export`; `None` when no command has it.
pub fn run(export: &str, input: &CommandInput) -> Option<CommandOutput> {
    Some(match export {
        "cmd__product_features" => list::<queries::FeatureListEntry>(input),
        "cmd__product_journeys" => list::<queries::JourneyListEntry>(input),
        "cmd__product_deliverables" => list::<queries::DeliverableListEntry>(input),
        "cmd__product_milestones" => list::<queries::MilestoneListEntry>(input),
        "cmd__product_modules" => list::<queries::ModuleListEntry>(input),
        "cmd__product_terms" => list::<queries::TermListEntry>(input),
        "cmd__product_personas" => list::<queries::PersonaListEntry>(input),
        "cmd__product_channels" => list::<queries::ChannelListEntry>(input),
        "cmd__product_releases" => list::<queries::ReleaseListEntry>(input),
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
                    r.completion_ratio * 100.0,
                    r.done_count,
                    r.total_features
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
                let pct = if r.total_features > 0 {
                    r.covered_count as f64 / r.total_features as f64 * 100.0
                } else {
                    0.0
                };
                let _ = writeln!(
                    out,
                    "Coverage: {pct:.0}% ({}/{} features done)",
                    r.covered_count, r.total_features
                );
                if !r.uncovered_features.is_empty() {
                    let _ = writeln!(out, "Uncovered:");
                    for f in &r.uncovered_features {
                        let _ = writeln!(out, "  {f}");
                    }
                }
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
        "cmd__product_feature_dependents" => {
            lookup(input, "feature", queries::feature_dependents, |r, out| {
                ids(
                    out,
                    &format!("Features depending on '{}'", r.feature_id),
                    &r.dependents,
                );
            })
        }
        "cmd__product_persona_features" => {
            lookup(input, "persona", queries::persona_features, |r, out| {
                ids(
                    out,
                    &format!("Features for persona '{}'", r.persona_id),
                    &r.features,
                );
            })
        }
        "cmd__product_channel_features" => {
            lookup(input, "channel", queries::channel_features, |r, out| {
                ids(
                    out,
                    &format!("Features for channel '{}'", r.channel_id),
                    &r.features,
                );
            })
        }
        "cmd__product_bulk_status" => {
            let result = queries::bulk_status(&input.graph);
            render(input, &result, |out| {
                if result.kinds.is_empty() {
                    let _ = writeln!(out, "No status-bearing entities found.");
                    return;
                }
                let rows: Vec<Vec<String>> = result
                    .kinds
                    .iter()
                    .flat_map(|k| {
                        k.by_status
                            .iter()
                            .map(|s| vec![k.kind.clone(), s.status.clone(), s.count.to_string()])
                    })
                    .collect();
                out.push_str(&table(&["kind", "status", "count"], &rows));
            })
        }
        "cmd__product_health" => {
            let report = queries::project_health(&input.graph);
            render(input, &report, |out| health(&report, out))
        }
        _ => return None,
    })
}

/// A list command over `T`'s kind: the page its filter args select, its
/// entries under the kind's plural. An arg it cannot use is
/// `INVALID_INPUT`: a page that is not a count, a value outside the enum a
/// filter takes, a sort field the kind does not have, a sort order that is
/// not `asc` or `desc`.
fn list<T: ListEntry>(input: &CommandInput) -> CommandOutput {
    let filter = match list_filter(input, T::KIND) {
        Ok(filter) => filter,
        Err(error) => return fail(input, &error, queries::INVALID_INPUT_EXIT),
    };
    let page = queries::list::<T>(&input.graph, &filter);
    render(input, &page.payload(T::KIND.plural), |out| {
        let rows: Vec<Vec<String>> = page.items.iter().map(ListEntry::row).collect();
        out.push_str(&table(T::HEADERS, &rows));
        if page.has_more {
            let _ = writeln!(
                out,
                "{} of {} {}; --offset {} for more",
                page.items.len(),
                page.total,
                T::KIND.plural,
                page.offset + page.items.len()
            );
        }
    })
}

/// The `ListFilter` `input`'s args set for `kind`, validated.
fn list_filter<'a>(
    input: &'a CommandInput,
    kind: &'a ListKind,
) -> Result<ListFilter<'a>, CommandError> {
    for page in ["limit", "offset"] {
        if input.args.contains_key(page) && input.arg_usize(page).is_none() {
            return Err(queries::invalid_input(format!(
                "{page} must be a non-negative integer, got {}",
                input.args[page]
            )));
        }
    }
    let mut filter = ListFilter::all(kind);
    filter.limit = input.arg_usize("limit");
    filter.offset = input.arg_usize("offset");
    for arg in kind.filters {
        let Some(value) = input.arg_str(arg.arg) else {
            continue;
        };
        if let Some(values) = arg.values {
            if !values.contains(&value) {
                return Err(one_of(arg.arg, values, value));
            }
        }
        filter.equals.push((arg.arg, value));
    }
    if let Some(tags) = input.arg_str("tags") {
        filter.tags = tags
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .collect();
    }
    if let Some(field) = input.arg_str("sort_by") {
        if !queries::sortable(kind.kind, field) {
            return Err(queries::invalid_input(format!(
                "sort_by: a {} has no field '{field}'",
                kind.kind
            )));
        }
        filter.sort_by = field;
    }
    if let Some(order) = input.arg_str("sort_order") {
        if !queries::SORT_ORDER.contains(&order) {
            return Err(one_of("sort_order", queries::SORT_ORDER, order));
        }
        filter.descending = order == "desc";
    }
    Ok(filter)
}

/// `INVALID_INPUT` for `value`, which is not one of `arg`'s `values`.
fn one_of(arg: &str, values: &[&str], value: &str) -> CommandError {
    queries::invalid_input(format!(
        "{arg} must be one of {}, got '{value}'",
        values.join(", ")
    ))
}

/// A query about the entity the positional arg `kind` names: its payload
/// rendered, or `ENTITY_NOT_FOUND` (exit 1) with the nearest id of the kind.
fn lookup<T: Serialize>(
    input: &CommandInput,
    kind: &str,
    query: fn(&CommandGraph, &str) -> Option<T>,
    human: impl FnOnce(&T, &mut String),
) -> CommandOutput {
    let id = input.arg_str(kind).unwrap_or_default();
    match query(&input.graph, id) {
        Some(result) => render(input, &result, |out| human(&result, out)),
        None => fail(
            input,
            &queries::not_found(&input.graph, kind, id),
            queries::NOT_FOUND_EXIT,
        ),
    }
}

/// Entity ids, one per line under `heading`.
fn ids(out: &mut String, heading: &str, ids: &[String]) {
    let _ = writeln!(out, "{heading}:");
    for i in ids {
        let _ = writeln!(out, "  {i}");
    }
    if ids.is_empty() {
        let _ = writeln!(out, "  (none)");
    }
}

fn health(report: &queries::HealthPayload, out: &mut String) {
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

/// `rows` under `headers`, each column as wide as its widest cell, two
/// spaces apart; no trailing spaces.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let mut out = String::new();
    let header: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
    for row in std::iter::once(&header).chain(rows) {
        let mut line = String::new();
        for (i, (cell, width)) in row.iter().zip(&widths).enumerate() {
            if i > 0 {
                line.push_str("  ");
            }
            let _ = write!(line, "{cell:<width$}");
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// `error` on stderr in the format the host asked for, exit `exit_code`.
fn fail(input: &CommandInput, error: &CommandError, exit_code: i32) -> CommandOutput {
    CommandOutput::error(input.format, error, exit_code)
}

/// `result` as pretty JSON when the host asked for json, else as `human`
/// writes it.
fn render<T: Serialize>(
    input: &CommandInput,
    result: &T,
    human: impl FnOnce(&mut String),
) -> CommandOutput {
    let mut out = String::new();
    if input.is_json() {
        out = serde_json::to_string_pretty(result).unwrap_or_default();
        out.push('\n');
    } else {
        human(&mut out);
    }
    CommandOutput::ok(out)
}
