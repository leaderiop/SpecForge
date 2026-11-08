//! `milestone_branches <milestone>`: the local branches a milestone's work sits on, for the fences of
//! complete-milestone, from the strategy, each computed `branch` and the git port. Read-only. A phase the strategy
//! names no branch for is refused (`gsd/W409`), so no merge goes on without it; a backlog phase has none by design.

use serde_json::json;
use specforge_extension_sdk::host::git;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, required, run};

/// The `branch` of `entity`, or the refusal that it has none.
fn branch_of(entity: &GraphNode) -> Result<String, String> {
    entity
        .fields
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            format!(
                "'{}' has no branch name: see gsd/W409 (specforge check)",
                entity.id
            )
        })
}

/// The names the strategy gives the milestone's work, one each, in the graph's order.
fn names(
    call: &CommandCall<'_>,
    milestone: &GraphNode,
    strategy: &str,
) -> Result<Vec<String>, String> {
    if strategy == "milestone" {
        return Ok(vec![branch_of(milestone)?]);
    }
    if strategy != "phase" {
        return Ok(Vec::new());
    }
    let graph = call.graph();
    let in_milestone = |phase: &GraphNode| {
        graph.edges().iter().any(|edge| {
            edge.source == phase.id && edge.label == "milestone" && edge.target == milestone.id
        })
    };
    graph
        .nodes()
        .iter()
        .filter(|n| {
            n.kind == "phase"
                && !n
                    .id
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name.starts_with("999."))
                && in_milestone(n)
        })
        .map(branch_of)
        .collect()
}

fn milestone_branches(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let milestone = node(call, "milestone", required(call, "milestone")?)?;
    let settings = Settings::current();
    let strategy = settings.string("git.branching_strategy").unwrap_or("none");
    let mut branches = Vec::new();
    for name in names(call, milestone, strategy)? {
        let found = git::commit(&format!("refs/heads/{name}"))
            .map_err(|error| format!("the branch '{name}' cannot be read: {error:?}"))?;
        if found.is_some() {
            branches.push(name);
        }
    }
    let human = branches.join("\n");
    Ok(answer(
        call,
        json!({
            "milestone": milestone.id,
            "strategy": strategy,
            "branches": branches,
        }),
        human,
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("milestone_branches", |cmd| {
        cmd.description("Answer the local branches a milestone's work sits on, by the branching strategy: read-only")
            .arg("milestone", |a| {
                a.required().description("the milestone, by its id");
            })
            .handler(|call| run(call, milestone_branches));
    });
}
