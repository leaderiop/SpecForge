//! `health`, `stats` and `progress_report` (D-r41-12, ADR 0299): the reports gsd-tools prints, answered from the
//! graph a command receives, which carries each phase's computed `status` and `plans_complete`. `progress` is the one
//! computation `progress_report` and the statusline share. A graph without those computed values (a view whose checks
//! were skipped) is refused, never guessed at (ADR 0299 D4).

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, run};

/// The label `progress json` gives a phase status (`phase-status.cts`).
pub(crate) fn label(status: &str) -> Option<&'static str> {
    Some(match status {
        "not_started" => "Pending",
        "planned" => "Planned",
        "in_progress" => "In Progress",
        "executed" => "Executed",
        "needs_review" => "Needs Review",
        "complete" => "Complete",
        _ => return None,
    })
}

/// One phase of the current milestone, as the graph's computed values give it.
struct Phase {
    number: String,
    name: String,
    plans: u64,
    summaries: u64,
    status: &'static str,
}

impl Phase {
    /// The phase directory's name in GSD: `02-persistence`.
    fn directory(&self) -> String {
        format!("{}-{}", self.number, self.name.replace(' ', "-"))
    }
}

/// The milestone a report is about: the one in progress, else the last.
pub(super) fn current(graph: &CommandGraph) -> Option<&GraphNode> {
    let milestones: Vec<&GraphNode> = graph.nodes_of_kind("milestone").collect();
    milestones
        .iter()
        .find(|m| m.text("state") == Some("in_progress"))
        .or(milestones.last())
        .copied()
}

fn phases(graph: &CommandGraph, milestone: Option<&GraphNode>) -> Result<Vec<Phase>, String> {
    let scope = milestone.map(|m| format!("{}/", m.id));
    graph
        .nodes_of_kind("phase")
        .filter(|p| scope.as_ref().is_none_or(|s| p.id.starts_with(s)))
        // The sentinel 999 (backlog) phases are left out, as `milestone.progress` and gsd-tools do (#3167).
        .filter(|p| {
            !p.id
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .starts_with("999.")
        })
        .map(|p| {
            let absent = |field: &str| {
                format!(
                    "the graph carries no computed {field} for {}; run `specforge check`",
                    p.id
                )
            };
            let status = p.text("status").ok_or_else(|| absent("status"))?;
            let status =
                label(status).ok_or_else(|| format!("{}: unknown status '{status}'", p.id))?;
            let counts = p
                .text("plans_complete")
                .and_then(|c| c.split_once('/'))
                .and_then(|(s, n)| Some((s.parse().ok()?, n.parse().ok()?)))
                .ok_or_else(|| absent("plans_complete"))?;
            Ok(Phase {
                number: p.id.rsplit('/').next().unwrap_or_default().to_string(),
                name: p.text("slug").unwrap_or_default().replace('-', " "),
                summaries: counts.0,
                plans: counts.1,
                status,
            })
        })
        .collect()
}

fn percent(part: u64, whole: u64) -> u64 {
    if whole == 0 {
        0
    } else {
        (part as f64 * 100.0 / whole as f64).round() as u64
    }
}

/// What `progress json` answers: the phases with their plans, summaries and status label, and the plan-based percent.
/// `milestone_version` and `milestone_name` are the milestone's (upstream reads them from STATE.md, which a project of
/// this extension has none of).
pub(crate) fn progress(graph: &CommandGraph) -> Result<Value, String> {
    let milestone = current(graph);
    let found = phases(graph, milestone)?;
    let plans: u64 = found.iter().map(|p| p.plans).sum();
    let summaries: u64 = found.iter().map(|p| p.summaries).sum();
    Ok(json!({
        "milestone_version": milestone.map(|m| m.id.as_str()),
        "milestone_name": milestone.and_then(|m| m.text("name")),
        "phases": found.iter().map(|p| json!({
            "number": p.number, "name": p.name, "plans": p.plans, "summaries": p.summaries, "status": p.status,
        })).collect::<Vec<_>>(),
        "total_plans": plans,
        "total_summaries": summaries,
        "percent": percent(summaries, plans),
        "phase_scope": "complete",
    }))
}

fn progress_report(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let report = progress(call.graph())?;
    let mut human = String::new();
    for p in report["phases"].as_array().into_iter().flatten() {
        human.push_str(&format!(
            "{:<6} {:<12} {}/{} plans  {}\n",
            p["number"].as_str().unwrap_or_default(),
            p["status"].as_str().unwrap_or_default(),
            p["summaries"],
            p["plans"],
            p["name"].as_str().unwrap_or_default()
        ));
    }
    human.push_str(&format!("{}% of plans have a summary\n", report["percent"]));
    Ok(answer(call, report, human, None))
}

/// `validate health`'s shape. Only `I001` has a counterpart in a graph (a phase with plans lacking a summary); the
/// missing-file and file-hygiene codes (`E002`-`E004` ...) have none, and `specforge check` is the health check of the
/// rest.
fn health(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let found = phases(call.graph(), current(call.graph()))?;
    let info: Vec<Value> = found
        .iter()
        .filter(|p| p.plans > p.summaries)
        .map(|p| {
            json!({
                "code": "I001",
                "message": format!(
                    "Phase {} has {} plan(s) without a matching summary",
                    p.directory(),
                    p.plans - p.summaries
                ),
                "fix": "May be in progress",
                "repairable": false,
            })
        })
        .collect();
    let mut human = String::from("healthy\n");
    for i in &info {
        human.push_str(&format!(
            "  info {}: {}\n",
            i["code"].as_str().unwrap_or_default(),
            i["message"].as_str().unwrap_or_default()
        ));
    }
    human.push_str("Run `specforge check` for the project's diagnostics.\n");
    Ok(answer(
        call,
        json!({ "status": "healthy", "errors": [], "warnings": [], "info": info, "repairable_count": 0 }),
        human,
        None,
    ))
}

/// The date of a timestamp (`2026-10-29T10:00:00Z`).
fn date(at: &str) -> &str {
    at.get(..10).unwrap_or(at)
}

/// `cmdStats`' shape: `progress`'s phases and totals, the phase and requirement counts, the recorded commits and the
/// newest summary's date (`null` with none; upstream reads STATE.md's text). `git_commits` and
/// `git_first_commit_date` come from the `commit` entities the `gsd-git` collector recorded, and are `0` and `null`
/// with none (upstream asks git itself).
fn stats(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let graph = call.graph();
    let mut report = progress(graph)?;
    let phases = report["phases"].as_array().cloned().unwrap_or_default();
    // `stats` (unlike `progress json`) says `Not Started` for a phase nothing has begun.
    for p in report["phases"].as_array_mut().into_iter().flatten() {
        if p["status"] == "Pending" {
            p["status"] = json!("Not Started");
        }
    }
    let complete = phases.iter().filter(|p| p["status"] == "Complete").count() as u64;
    let requirements: Vec<&GraphNode> = graph
        .nodes_of_kind("requirement")
        .filter(|r| r.text("tier") == Some("v1"))
        .collect();
    let commits: Vec<&str> = graph
        .nodes_of_kind("commit")
        .filter_map(|c| c.text("at"))
        .collect();
    let last = graph
        .nodes_of_kind("summary")
        .filter_map(|s| s.text("completed"))
        .max()
        .map(|at| date(at).to_string());
    let total = phases.len() as u64;
    let plans = report["total_plans"].as_u64().unwrap_or(0);
    let summaries = report["total_summaries"].as_u64().unwrap_or(0);
    let extra = json!({
        "phases_completed": complete,
        "phases_total": total,
        "percent": percent(complete, total),
        "plan_percent": percent(summaries, plans),
        "requirements_total": requirements.len(),
        "requirements_complete": requirements.iter().filter(|r| r.text("status") == Some("complete")).count(),
        "last_activity": last,
        "git_commits": commits.len(),
        "git_first_commit_date": commits.iter().min().map(|first| date(first)),
    });
    let map = report.as_object_mut().expect("an object");
    map.extend(extra.as_object().cloned().unwrap_or_default());
    let human = format!(
        "{complete}/{total} phases complete, {summaries}/{plans} plans with a summary, last activity {}\n",
        last.as_deref().unwrap_or("unknown")
    );
    Ok(answer(call, report, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("health", |cmd| {
        cmd.description("Report the project's health as gsd-tools does: phases whose plans lack a summary (I001); `specforge check` has the rest")
            .handler(|call| run(call, health));
    });
    c.command("stats", |cmd| {
        cmd.description("Report the project's statistics as gsd-tools does: phases, plans, requirements, recorded commits and the last activity")
            .handler(|call| run(call, stats));
    });
    c.command("progress_report", |cmd| {
        cmd.description("Report each phase's plans, summaries and status, and the plan-based percent, as gsd-tools' progress json does")
            .handler(|call| run(call, progress_report));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_report_without_computed_status_is_refused() {
        let graph = CommandGraph::new(
            vec![
                node("v1.0", "milestone", &[("state", "in_progress")]),
                node("v1.0/01", "phase", &[("slug", "core")]),
            ],
            Vec::new(),
        );
        let why = progress(&graph).unwrap_err();
        assert!(why.contains("no computed status for v1.0/01"), "{why}");
        assert!(why.contains("specforge check"), "{why}");
    }

    #[test]
    fn a_phase_reads_its_computed_values() {
        let graph = CommandGraph::new(
            vec![
                node(
                    "v1.0",
                    "milestone",
                    &[("state", "in_progress"), ("name", "One")],
                ),
                node(
                    "v1.0/02.1",
                    "phase",
                    &[
                        ("slug", "a-fix"),
                        ("status", "planned"),
                        ("plans_complete", "1/3"),
                    ],
                ),
            ],
            Vec::new(),
        );
        let report = progress(&graph).unwrap();
        assert_eq!(report["phases"][0]["number"], "02.1");
        assert_eq!(report["phases"][0]["name"], "a fix");
        assert_eq!(report["phases"][0]["status"], "Planned");
        assert_eq!(
            (report["total_plans"].clone(), report["percent"].clone()),
            (json!(3), json!(33))
        );
        assert_eq!(report["milestone_name"], "One");
    }
}
