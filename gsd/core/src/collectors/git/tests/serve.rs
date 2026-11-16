//! What `collect` answers with a repository served, over the graph as the host exports it: each task's `plan` an
//! edge, resolved as the host resolves a scope field.

use super::*;

/// What `collect` answers with `repo` served (its top `/r` when none is set), over the input's graph with its tasks'
/// `plan` edges resolved.
pub(crate) fn served(
    mut repo: Repo,
    input: &CollectInput,
) -> (Result<CollectOutput, String>, Rc<Repo>) {
    if repo.top.is_empty() {
        repo.top = "/r".into();
    }
    let repo = Rc::new(repo);
    let input = CollectInput {
        graph: input.graph.clone().map(with_plan_edges),
        ..input.clone()
    };
    let output = in_process::serve(Rc::clone(&repo) as Rc<dyn InProcessHost>, || {
        collect(&input)
    });
    (output, repo)
}

/// `graph` with each task's `plan` edge, its scope field resolved as the graph resolves one (ADR 0284): from the
/// project, a path or a rooted `/path`, else a unique suffix on a `/` boundary.
fn with_plan_edges(mut graph: GraphWire) -> GraphWire {
    let plans: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "plan")
        .map(|n| n.id.as_str())
        .collect();
    let resolve = |written: &str| -> Option<String> {
        let path = written.strip_prefix('/').unwrap_or(written);
        if plans.contains(&path) {
            return Some(path.to_string());
        }
        let tail = format!("/{written}");
        match plans
            .iter()
            .filter(|p| !written.starts_with('/') && p.ends_with(&tail))
            .collect::<Vec<_>>()[..]
        {
            [one] => Some(one.to_string()),
            _ => None,
        }
    };
    let edges: Vec<GraphEdge> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "task")
        .filter_map(|task| {
            Some(GraphEdge {
                source: task.id.clone(),
                target: resolve(task.text("plan")?)?,
                label: "plan".into(),
            })
        })
        .collect();
    graph.edges.extend(edges);
    graph
}
