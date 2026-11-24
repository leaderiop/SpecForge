//! A shipped milestone's `tag`, read from `refs/tags/v<version>` (ADR 0390 D6).

use super::super::tags::tags;
use super::*;

const SHA: &str = "1111111111111111111111111111111111111111";

/// A milestone, shipped when `shipped`.
fn milestone(id: &str, shipped: bool) -> GraphNode {
    let mut node = node(id, "milestone", id);
    if shipped {
        node.fields
            .insert("completion".into(), json!({ "at": "2026-02-01" }));
    }
    node
}

/// The `tag` each milestone is set to, collecting over `repo` (one commit no plan names) and `milestones`.
fn tagged(repo: Repo, milestones: Vec<GraphNode>) -> Vec<(String, Value)> {
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.extend(milestones);
    let input = CollectInput {
        graph: Some(graph),
        ..Default::default()
    };
    let repo = Repo {
        log: vec![logged(&"a".repeat(40), &[], 10, "chore: tidy", &[])],
        ..repo
    };
    let output = served(repo, &input).0.expect("an answer");
    output
        .plan
        .iter()
        .flat_map(|plan| &plan.ops)
        .filter_map(|op| match op {
            PlanOp::Set {
                entity,
                field,
                value,
                ..
            } if field == "tag" => Some((entity.clone(), value.clone())),
            _ => None,
        })
        .collect()
}

fn tagged_v17() -> Repo {
    Repo {
        tags: HashMap::from([("v1.7".to_string(), SHA.to_string())]),
        ..Repo::default()
    }
}

#[test]
fn tags_of_a_shipped_version_record_the_commit_it_resolves_to() {
    let set = tagged(
        tagged_v17(),
        vec![
            milestone("v1.7", true),
            milestone("v1.8", false),
            milestone("mvp", true),
        ],
    );
    assert_eq!(
        set,
        [("v1.7".to_string(), json!({ "name": "v1.7", "commit": SHA }))]
    );
}

#[test]
fn tags_that_do_not_resolve_are_recorded_without_a_commit() {
    let set = tagged(
        Repo::default(),
        vec![milestone("v1.7", true), milestone("a/1.7.2", true)],
    );
    assert_eq!(
        set,
        [
            ("v1.7".to_string(), json!({ "name": "v1.7" })),
            ("a/1.7.2".to_string(), json!({ "name": "v1.7.2" })),
        ]
    );
}

#[test]
fn tags_already_held_are_not_set_again() {
    let mut held = milestone("v1.7", true);
    held.fields
        .insert("tag".into(), json!({ "name": "v1.7", "commit": SHA }));
    assert!(tagged(tagged_v17(), vec![held]).is_empty());
}

#[test]
fn tags_are_not_read_on_a_shallow_clone_or_while_git_create_tag_is_false() {
    let shallow = Repo {
        shallow: true,
        ..tagged_v17()
    };
    assert!(tagged(shallow, vec![milestone("v1.7", true)]).is_empty());
    // No port is asked: this runs with no host.
    assert_eq!(
        tags(&[milestone("v1.7", true)], false, false),
        Ok(Vec::new())
    );
}

#[test]
fn tags_ask_nothing_for_a_name_that_is_no_version() {
    // The fake repository answers `refs/tags/<t>` for any `t`: a name that does not parse asks nothing at all.
    let names = ["v1.7;rm -rf", "v1.7^{tree}", "--all", "v1.7 HEAD", "mvp"];
    let milestones = names.iter().map(|n| milestone(n, true)).collect();
    assert!(tagged(tagged_v17(), milestones).is_empty());
    let nodes: Vec<GraphNode> = names.iter().map(|n| milestone(n, true)).collect();
    assert_eq!(tags(&nodes, false, true), Ok(Vec::new()));
}
