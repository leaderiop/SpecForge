//! `thread_plan <phase>`: the phase's plans and tasks in the order they run, as the agent plan `specforge trace
//! --plan` checks against the graph (ADR 0378).
//!
//! The answer is `{plan_id, entries: [{entity_id, action}]}`, exactly the kernel's `AgentPlan`: the plans whose `phase`
//! edge targets the phase (a quick plan has none, so it is in no phase's thread), ordered by the computed
//! `strict_wave` then by id as a person reads it, each followed by its tasks (the tasks whose `plan` edge targets it)
//! in the same order: the one order of the placement, the critical path and `progress-report`. A plan's action
//! is `execute`, a task's `implement`. The order satisfies `depends_on` (a plan's strict wave is past each plan it
//! depends on) and `task.plan` by construction, so a gap `trace --plan` finds in the unedited answer comes from an
//! edge the strict waves do not order, such as a coupling peer that runs later, and one in an edited list from the
//! reordering. The order of the tasks inside a plan is not checked: a task carries no `depends_on`. A phase whose
//! `depends_on` has a cycle has no strict wave, and is refused. The command reads; it declares no write.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, id_order, node_in, required, run, target};

/// The agent plan of the phase `phase` names (its full path, or the end of one no other phase shares).
pub fn thread(graph: &CommandGraph, phase: &str) -> Result<Value, String> {
    let phase = node_in(graph, "phase", phase)?;
    let mut plans = Vec::new();
    for plan in graph
        .nodes_of_kind("plan")
        .filter(|p| target(graph, &p.id, "phase") == Some(phase.id.as_str()))
    {
        let Some(wave) = plan.fields.get("strict_wave").and_then(Value::as_i64) else {
            return Err(format!(
                "the plans of '{}' have no strict wave: their depends_on has a cycle (gsd/E102)",
                phase.id
            ));
        };
        plans.push((wave, plan.id.as_str()));
    }
    plans.sort_by_key(|&(wave, id)| (wave, id_order(id)));
    let mut tasks: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for task in graph.nodes_of_kind("task") {
        if let Some(plan) = target(graph, &task.id, "plan") {
            tasks.entry(plan).or_default().push(task.id.as_str());
        }
    }
    let mut entries = Vec::new();
    for (_, plan) in plans {
        entries.push(json!({ "entity_id": plan, "action": "execute" }));
        let mut of_plan = tasks.remove(plan).unwrap_or_default();
        of_plan.sort_by_key(|id| id_order(id));
        entries.extend(
            of_plan
                .into_iter()
                .map(|task| json!({ "entity_id": task, "action": "implement" })),
        );
    }
    Ok(json!({ "plan_id": phase.id, "entries": entries }))
}

/// The text form: one line per plan (its strict wave, its id and how many tasks follow it), then how to check it.
fn human(report: &Value, graph: &CommandGraph) -> String {
    let phase = report["plan_id"].as_str().unwrap_or_default();
    let entries = report["entries"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut out = String::new();
    let mut plans = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e["action"] == "execute")
        .peekable();
    if plans.peek().is_none() {
        return format!("no plan in {phase}\n");
    }
    for (at, plan) in plans {
        let id = plan["entity_id"].as_str().unwrap_or_default();
        let tasks = entries[at + 1..]
            .iter()
            .take_while(|e| e["action"] == "implement")
            .count();
        let wave = graph
            .node(id)
            .and_then(|p| p.fields.get("strict_wave"))
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let noun = if tasks == 1 { "task" } else { "tasks" };
        out.push_str(&format!("wave {wave}  {id}  ({tasks} {noun})\n"));
    }
    out.push_str(&format!(
        "check the order: write `specforge gsd thread-plan {phase} --format json` to a file and run \
         `specforge trace --plan <file>`\n"
    ));
    out
}

fn thread_plan_command(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let report = thread(call.graph(), required(call, "phase")?)?;
    let human = human(&report, call.graph());
    Ok(answer(call, report, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("thread_plan", |cmd| {
        cmd.description("Answer a phase's plans and tasks in the order they run, as the agent plan the kernel checks: each plan by its strict wave then its id, followed by its tasks. Write `specforge gsd thread-plan <phase> --format json` to a file and run `specforge trace --plan <file>`: an entry before a plan or task it depends on is an ordering gap, an entry naming no entity is E003")
            .arg("phase", |a| {
                a.required()
                    .description("the phase whose plans and tasks to thread");
            })
            .handler(|call| run(call, thread_plan_command));
    });
}

#[cfg(test)]
mod tests;
