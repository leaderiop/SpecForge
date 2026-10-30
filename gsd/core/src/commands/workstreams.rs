//! `workstreams` and `cleanup` (D-r41-9, D-r41-11): the workstreams behind `/gsd-workstreams` and the report behind
//! `/gsd-cleanup`. A command cannot read the project's state, so a workstream is
//! `complete` when each of its milestones has a `completion` (what the computed status says), its progress counts the
//! plans that have a summary, and which workstream is active is written (`switch`) but not answered (`status`).
//!
//! Also ports gsd-core bin/lib modules: active-workstream-store.cjs.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, node, required, run, text, title};

const ACTIONS: &[&str] = &[
    "list", "status", "progress", "create", "complete", "switch", "resume",
];

/// A milestone with the numbers `status` and `progress` give.
struct Milestone<'a> {
    node: &'a GraphNode,
    phases: usize,
    plans: usize,
    summarised: usize,
}

impl Milestone<'_> {
    fn shipped(&self) -> bool {
        self.node.has_field("completion")
    }

    /// Plans with a summary over plans, as a percent; a milestone with no plan is 0.
    fn percent(&self) -> usize {
        (self.summarised * 100).checked_div(self.plans).unwrap_or(0)
    }

    fn json(&self) -> Value {
        json!({
            "milestone": self.node.id, "shipped": self.shipped(), "phases": self.phases,
            "plans": self.plans, "plans_with_summary": self.summarised, "percent": self.percent(),
        })
    }
}

fn milestone_of<'a>(call: &CommandCall<'a>, node: &'a GraphNode) -> Milestone<'a> {
    let graph = call.graph();
    let under = format!("{}/", node.id);
    let count = |kind: &str, depth: usize| {
        graph
            .nodes()
            .iter()
            .filter(|n| {
                n.kind == kind
                    && n.id.starts_with(&under)
                    && n.id.matches('/').count() == node.id.matches('/').count() + depth
            })
            .count()
    };
    Milestone {
        node,
        phases: count("phase", 1),
        plans: count("plan", 2),
        summarised: count("summary", 3),
    }
}

/// The milestones of workstream `name`, or of every workstream (and none) when it is `None`.
fn milestones<'a>(call: &CommandCall<'a>, workstream: Option<&str>) -> Vec<Milestone<'a>> {
    call.graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "milestone")
        .filter(|n| workstream.is_none_or(|w| n.text("workstream") == Some(w)))
        .map(|n| milestone_of(call, n))
        .collect()
}

fn workstream<'a>(call: &CommandCall<'a>) -> Result<&'a GraphNode, String> {
    node(call, "workstream", required(call, "name")?)
}

fn report(call: &CommandCall<'_>, action: &str) -> Result<CommandOutput, String> {
    let chosen = text(call, "name")?.map(|_| workstream(call)).transpose()?;
    let all: Vec<&GraphNode> = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "workstream")
        .filter(|n| chosen.is_none_or(|c| c.id == n.id))
        .collect();
    let mut rows = Vec::new();
    let mut human = String::new();
    for w in &all {
        let ms = milestones(call, Some(&w.id));
        let complete = !ms.is_empty() && ms.iter().all(Milestone::shipped);
        human.push_str(&format!(
            "{}: {} ({} milestones){}\n",
            w.id,
            if complete { "complete" } else { "active" },
            ms.len(),
            w.text("purpose")
                .map_or(String::new(), |p| format!(" - {p}"))
        ));
        if action != "list" {
            for m in &ms {
                human.push_str(&format!(
                    "  {}: {} phases, {}/{} plans with a summary ({}%)\n",
                    m.node.id,
                    m.phases,
                    m.summarised,
                    m.plans,
                    m.percent()
                ));
            }
        }
        rows.push(json!({
            "workstream": w.id, "complete": complete, "purpose": w.text("purpose"),
            "milestones": ms.iter().map(Milestone::json).collect::<Vec<_>>(),
        }));
    }
    if all.is_empty() {
        human.push_str("no workstreams\n");
    }
    Ok(answer(call, json!({ "workstreams": rows }), human, None))
}

fn create(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let name = required(call, "name")?;
    let purpose = required(call, "purpose")?;
    if call.graph().node(name).is_some() {
        return Err(format!("'{name}' already exists"));
    }
    let file = call
        .graph()
        .nodes()
        .iter()
        .find(|n| n.kind == "project")
        .and_then(|n| n.file.clone())
        .unwrap_or_else(|| "spec/project.spec".to_string());
    let mut plan = call.plan();
    plan.create(
        "workstream",
        name,
        &title(name),
        &file,
        fields([("purpose", json!(purpose))]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "wrote": [name] }),
        format!("workstream {name} created\n"),
        Some(&plan),
    ))
}

fn complete(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let w = workstream(call)?;
    let open: Vec<String> = milestones(call, Some(&w.id))
        .iter()
        .filter(|m| !m.shipped())
        .map(|m| m.node.id.clone())
        .collect();
    let human = if open.is_empty() {
        format!(
            "workstream {} is complete: every milestone is shipped\n",
            w.id
        )
    } else {
        format!(
            "workstream {} is not complete: {} not shipped\n",
            w.id,
            open.join(", ")
        )
    };
    Ok(answer(
        call,
        json!({ "workstream": w.id, "complete": open.is_empty(), "open": open }),
        human,
        None,
    ))
}

/// `switch` and `resume`: the active workstream, written to the project's state.
fn switch(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let w = workstream(call)?;
    let mut out = answer(
        call,
        json!({ "active_workstream": w.id }),
        format!("active workstream: {}\n", w.id),
        None,
    );
    out.state = Some(StatePatch {
        project: Some(json!({ "active_workstream": w.id })),
        session: None,
    });
    Ok(out)
}

fn workstreams(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    match required(call, "action")? {
        action @ ("list" | "status" | "progress") => report(call, action),
        "create" => create(call),
        "complete" => complete(call),
        _ => switch(call),
    }
}

/// `cleanup`: what upstream archives, and why nothing moves here.
fn cleanup(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let shipped: Vec<Milestone<'_>> = milestones(call, None)
        .into_iter()
        .filter(Milestone::shipped)
        .collect();
    let mut human = String::new();
    if shipped.is_empty() {
        human.push_str("nothing is shipped: no milestone has a completion\n");
    }
    for m in &shipped {
        human.push_str(&format!(
            "{}: shipped {}; {} phases, {} plans\n",
            m.node.id,
            m.node
                .fields
                .get("completion")
                .and_then(|c| c["at"].as_str())
                .unwrap_or_default(),
            m.phases,
            m.plans
        ));
    }
    human.push_str("GSD would archive each shipped milestone's phase directories; a shipped milestone's entities stay where they are here, so nothing moves.\nPruning stale branches is the agent's git step.\n");
    Ok(answer(
        call,
        json!({ "shipped": shipped.iter().map(Milestone::json).collect::<Vec<_>>(), "archived": false }),
        human,
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("workstreams", |cmd| {
        cmd.description("Manage workstreams: list them, show their status and progress, create one, say whether one is complete, or switch to one")
            .writes()
            .arg("action", |a| {
                a.required().one_of(ACTIONS).description("what to do");
            })
            .arg("name", |a| {
                a.description("the workstream (all but list; status and progress take every workstream without it)");
            })
            .arg("purpose", |a| {
                a.description("what the workstream is for (create)");
            })
            .handler(|call| run(call, workstreams));
    });
    c.command("cleanup", |cmd| {
        cmd.description("Report the shipped milestones GSD would archive; nothing is archived here and nothing is written")
            .handler(|call| run(call, cleanup));
    });
}
