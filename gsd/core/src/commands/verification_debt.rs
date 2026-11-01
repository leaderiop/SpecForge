//! `verification_debt [--milestone M] [--phase P]`: the UAT tests still open, with the uat_test kind's own rule, for
//! `/gsd-progress`'s debt lines and `/gsd-verify-work`'s UAT gates (upstream's `audit-uat` and `phase uat-passed`).
//!
//! A test is open unless it passed, was skipped with a `Deferred follow-up…` reason, or is an issue whose gaps are
//! all resolved: the rule a UAT session's `complete` status uses (`passes/derive/uat.rs`). A test with no result is
//! pending. The answer is `{milestone, current, archived, archived_total, blockers}`: `current` counts the open tests
//! of the milestone (`--milestone`, else `--phase`'s, else the one in progress, else the last), `archived` each
//! shipped milestone with open tests (`{milestone, open}`) and `archived_total` their sum, and `blockers` the open
//! tests' ids, of `--phase` when it is given, else of the milestone. A graph whose gaps lack their computed `status`
//! (a view whose checks were skipped) is refused, never guessed at (ADR 0299 D4).

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::reports::current;
use super::support::{answer, run};
use crate::passes::derive::uat::is_deferred;

/// Whether `test` is still open: see the module's rule. `gaps` are the graph's uat_gap nodes, each naming its `test`.
fn open(test: &GraphNode, gaps: &[&GraphNode]) -> bool {
    match test.text("result").unwrap_or("pending") {
        "pass" => false,
        "skipped" => !is_deferred(test.text("reason")),
        "issue" => {
            let mine: Vec<&&GraphNode> = gaps
                .iter()
                .filter(|g| g.text("test") == Some(test.id.as_str()))
                .collect();
            // upstream: every gap of the issue is resolved
            mine.is_empty() || mine.iter().any(|g| g.text("status") != Some("resolved"))
        }
        _ => true,
    }
}

/// The open tests whose id lies under `scope` (a milestone or a phase id), in graph order.
fn open_under<'g>(graph: &'g CommandGraph, scope: &str) -> Vec<&'g str> {
    let gaps: Vec<&GraphNode> = graph.nodes_of_kind("uat_gap").collect();
    let prefix = format!("{scope}/");
    graph
        .nodes_of_kind("uat_test")
        .filter(|t| t.id.starts_with(&prefix) && open(t, &gaps))
        .map(|t| t.id.as_str())
        .collect()
}

/// The debt answer over `graph`, for the milestone `milestone` (else `phase`'s, else the current one) and `phase`.
pub fn debt(
    graph: &CommandGraph,
    milestone: Option<&str>,
    phase: Option<&str>,
) -> Result<serde_json::Value, String> {
    let known = |id: &str, kind: &str| {
        graph
            .nodes_of_kind(kind)
            .any(|n| n.id == id)
            .then_some(())
            .ok_or_else(|| format!("no {kind} '{id}'"))
    };
    if let Some(p) = phase {
        known(p, "phase")?;
    }
    // A gap's status is computed: without it the debt is unknown, not open (ADR 0299 D4, as the reports refuse).
    if let Some(gap) = graph
        .nodes_of_kind("uat_gap")
        .find(|g| g.text("status").is_none())
    {
        return Err(format!(
            "the graph carries no computed status for {}; run `specforge check`",
            gap.id
        ));
    }
    let milestone = match (milestone, phase) {
        (Some(m), _) => {
            known(m, "milestone")?;
            m.to_string()
        }
        (None, Some(p)) => graph
            .nodes_of_kind("milestone")
            .find(|m| p.starts_with(&format!("{}/", m.id)))
            .map(|m| m.id.clone())
            .ok_or_else(|| format!("phase '{p}' is in no milestone"))?,
        (None, None) => current(graph)
            .map(|m| m.id.clone())
            .ok_or("the project has no milestone")?,
    };
    let archived: Vec<serde_json::Value> = graph
        .nodes_of_kind("milestone")
        .filter(|m| m.id != milestone && m.text("state") == Some("shipped"))
        .filter_map(|m| {
            let open = open_under(graph, &m.id).len();
            (open > 0).then(|| json!({ "milestone": m.id, "open": open }))
        })
        .collect();
    let archived_total: u64 = archived.iter().filter_map(|a| a["open"].as_u64()).sum();
    Ok(json!({
        "milestone": milestone,
        "current": open_under(graph, &milestone).len(),
        "archived": archived,
        "archived_total": archived_total,
        "blockers": open_under(graph, phase.unwrap_or(&milestone)),
    }))
}

fn verification_debt(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let report = debt(call.graph(), call.str("milestone"), call.str("phase"))?;
    let human = format!(
        "{} open UAT test(s) in {}, {} in shipped milestones\n{}",
        report["current"],
        report["milestone"].as_str().unwrap_or_default(),
        report["archived_total"],
        report["blockers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|b| b.as_str())
            .map(|b| format!("{b}\n"))
            .collect::<String>()
    );
    Ok(answer(call, report, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("verification_debt", |cmd| {
        cmd.description("Count the UAT tests still open (pending or missing, blocked, an issue with an open gap, a skip without a `Deferred follow-up` reason) in a milestone and in the shipped ones, and list a phase's")
            .arg("milestone", |a| {
                a.description("the milestone to count (default: --phase's, else the one in progress, else the last)");
            })
            .arg("phase", |a| {
                a.description("the phase whose open tests `blockers` lists (default: the milestone's)");
            })
            .handler(|call| run(call, verification_debt));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(id: &str, kind: &str, fields: &[(&str, &str)]) -> GraphNode {
        let mut node = GraphNode {
            id: id.to_string(),
            kind: kind.to_string(),
            ..Default::default()
        };
        for (name, value) in fields {
            node.fields.insert((*name).to_string(), json!(value));
        }
        node
    }

    fn graph() -> CommandGraph {
        CommandGraph::new(
            vec![
                node("v0.9", "milestone", &[("state", "shipped")]),
                node("v0.9/01", "phase", &[]),
                node("v0.9/01/01-UAT/U1", "uat_test", &[]),
                node("v0.9/01/01-UAT/U2", "uat_test", &[("result", "pass")]),
                node("v0.8", "milestone", &[("state", "shipped")]),
                node("v0.8/01/01-UAT/U1", "uat_test", &[("result", "pass")]),
                node("v1.0", "milestone", &[("state", "in_progress")]),
                node("v1.0/01", "phase", &[]),
                node("v1.0/02", "phase", &[]),
                node("v1.0/01/01-UAT/U1", "uat_test", &[("result", "pass")]),
                node(
                    "v1.0/01/01-UAT/U2",
                    "uat_test",
                    &[
                        ("result", "skipped"),
                        ("reason", "Deferred follow-up: a device"),
                    ],
                ),
                node(
                    "v1.0/01/01-UAT/U3",
                    "uat_test",
                    &[("result", "skipped"), ("reason", "no time")],
                ),
                node("v1.0/01/01-UAT/U4", "uat_test", &[("result", "issue")]),
                node(
                    "v1.0/01/G-01-1",
                    "uat_gap",
                    &[("test", "v1.0/01/01-UAT/U4"), ("status", "resolved")],
                ),
                node("v1.0/01/01-UAT/U5", "uat_test", &[("result", "issue")]),
                node(
                    "v1.0/01/G-01-2",
                    "uat_gap",
                    &[("test", "v1.0/01/01-UAT/U5"), ("status", "resolved")],
                ),
                node(
                    "v1.0/01/G-01-3",
                    "uat_gap",
                    &[("test", "v1.0/01/01-UAT/U5"), ("status", "open")],
                ),
                node("v1.0/01/01-UAT/U6", "uat_test", &[("result", "issue")]),
                node("v1.0/02/02-UAT/U1", "uat_test", &[("result", "blocked")]),
                node("v1.0/02/02-UAT/U2", "uat_test", &[("result", "pending")]),
                node("v1.0/02/02-UAT/U3", "uat_test", &[]),
                node("v1.0/02/02-UAT/U4", "uat_test", &[("result", "pass")]),
            ],
            Vec::new(),
        )
    }

    #[test]
    fn a_milestones_debt_follows_the_kinds_rule() {
        let report = debt(&graph(), None, None).unwrap();
        assert_eq!(report["milestone"], "v1.0");
        // U3 (an unexcused skip), U5 (an open gap), U6 (an issue with no gap), and 02's U1-U3
        assert_eq!(
            report["blockers"],
            json!([
                "v1.0/01/01-UAT/U3",
                "v1.0/01/01-UAT/U5",
                "v1.0/01/01-UAT/U6",
                "v1.0/02/02-UAT/U1",
                "v1.0/02/02-UAT/U2",
                "v1.0/02/02-UAT/U3",
            ])
        );
        assert_eq!(report["current"], 6);
        assert_eq!(
            report["archived"],
            json!([{"milestone": "v0.9", "open": 1}])
        );
        assert_eq!(report["archived_total"], 1);
    }

    #[test]
    fn a_phase_lists_its_own_blockers_and_names_its_milestone() {
        let report = debt(&graph(), None, Some("v1.0/02")).unwrap();
        assert_eq!(report["milestone"], "v1.0");
        assert_eq!(report["current"], 6);
        assert_eq!(
            report["blockers"],
            json!([
                "v1.0/02/02-UAT/U1",
                "v1.0/02/02-UAT/U2",
                "v1.0/02/02-UAT/U3"
            ])
        );
        // a closed milestone is counted as current when asked for, and not again as archived
        let report = debt(&graph(), Some("v0.9"), None).unwrap();
        assert_eq!(
            (report["current"].clone(), report["archived_total"].clone()),
            (json!(1), json!(0))
        );
    }

    /// Re-review of plan 05, item 11: a gap's `status` is computed; a graph without it (a view whose checks were
    /// skipped) is refused, as the reports refuse (ADR 0299 D4), never counted as open.
    #[test]
    fn a_graph_without_computed_gap_status_is_refused() {
        let graph = CommandGraph::new(
            vec![
                node("v1.0", "milestone", &[("state", "in_progress")]),
                node("v1.0/01", "phase", &[]),
                node("v1.0/01/01-UAT/U1", "uat_test", &[("result", "issue")]),
                node(
                    "v1.0/01/G-01-1",
                    "uat_gap",
                    &[("test", "v1.0/01/01-UAT/U1")],
                ),
            ],
            Vec::new(),
        );
        let why = debt(&graph, None, None).unwrap_err();
        assert!(
            why.contains("no computed status for v1.0/01/G-01-1"),
            "{why}"
        );
        assert!(why.contains("specforge check"), "{why}");
    }

    #[test]
    fn an_unknown_scope_is_refused() {
        assert_eq!(
            debt(&graph(), Some("v2.0"), None).unwrap_err(),
            "no milestone 'v2.0'"
        );
        assert_eq!(
            debt(&graph(), None, Some("v1.0/09")).unwrap_err(),
            "no phase 'v1.0/09'"
        );
        let empty = CommandGraph::new(Vec::new(), Vec::new());
        assert_eq!(
            debt(&empty, None, None).unwrap_err(),
            "the project has no milestone"
        );
    }
}
