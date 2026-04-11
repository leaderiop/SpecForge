use specforge_graph::Graph;
use specforge_watch::{
    plan_incremental_dispatch, GraphDelta, KindDescriptor, NodeChange, ValidatorDescriptor,
    ValidatorInput,
};
use specforge_test_macros::test as spec;

fn delta_with_kinds(kinds: &[&str]) -> GraphDelta {
    GraphDelta {
        added_nodes: kinds
            .iter()
            .enumerate()
            .map(|(i, k)| NodeChange {
                id: format!("node_{}", i),
                kind: k.to_string(),
                file: Some("a.spec".to_string()),
                line: Some(1),
            })
            .collect(),
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec!["a.spec".to_string()],
    }
}

#[spec(behavior = "dispatch_incremental_validators", verify = "incremental extension receives delta only")]
#[test]
fn incremental_extension_receives_delta_input() {
    let validators = vec![ValidatorDescriptor {
        extension_name: "@specforge/software".to_string(),
        kinds: vec![KindDescriptor {
            kind_name: "behavior".to_string(),
            incremental: true,
        }],
    }];

    let delta = delta_with_kinds(&["behavior"]);
    let graph = Graph::new();
    let plan = plan_incremental_dispatch(&validators, &delta, &graph);

    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].input, ValidatorInput::Delta);
}

#[spec(behavior = "dispatch_incremental_validators", verify = "non-incremental extension receives full graph")]
#[test]
fn non_incremental_extension_receives_full_graph_input() {
    let validators = vec![ValidatorDescriptor {
        extension_name: "@specforge/governance".to_string(),
        kinds: vec![KindDescriptor {
            kind_name: "decision".to_string(),
            incremental: false,
        }],
    }];

    let delta = delta_with_kinds(&["decision"]);
    let graph = Graph::new();
    let plan = plan_incremental_dispatch(&validators, &delta, &graph);

    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].input, ValidatorInput::FullGraph);
}

#[spec(behavior = "dispatch_incremental_validators", verify = "dispatch follows topological order")]
#[test]
fn dispatch_preserves_topological_order() {
    let validators = vec![
        ValidatorDescriptor {
            extension_name: "first".to_string(),
            kinds: vec![KindDescriptor {
                kind_name: "a".to_string(),
                incremental: true,
            }],
        },
        ValidatorDescriptor {
            extension_name: "second".to_string(),
            kinds: vec![KindDescriptor {
                kind_name: "b".to_string(),
                incremental: false,
            }],
        },
        ValidatorDescriptor {
            extension_name: "third".to_string(),
            kinds: vec![KindDescriptor {
                kind_name: "c".to_string(),
                incremental: true,
            }],
        },
    ];

    let delta = delta_with_kinds(&["a", "b", "c"]);
    let graph = Graph::new();
    let plan = plan_incremental_dispatch(&validators, &delta, &graph);

    let names: Vec<&str> = plan
        .entries
        .iter()
        .map(|e| e.extension_name.as_str())
        .collect();
    assert_eq!(names, vec!["first", "second", "third"]);
}

#[spec(behavior = "dispatch_incremental_validators", verify = "mixed incremental and non-incremental kinds dispatch separately")]
#[test]
fn mixed_incremental_and_non_incremental_dispatch_separately() {
    let validators = vec![
        ValidatorDescriptor {
            extension_name: "inc_ext".to_string(),
            kinds: vec![KindDescriptor {
                kind_name: "behavior".to_string(),
                incremental: true,
            }],
        },
        ValidatorDescriptor {
            extension_name: "full_ext".to_string(),
            kinds: vec![KindDescriptor {
                kind_name: "decision".to_string(),
                incremental: false,
            }],
        },
    ];

    let delta = delta_with_kinds(&["behavior", "decision"]);
    let graph = Graph::new();
    let plan = plan_incremental_dispatch(&validators, &delta, &graph);

    assert_eq!(plan.entries[0].input, ValidatorInput::Delta);
    assert_eq!(plan.entries[1].input, ValidatorInput::FullGraph);
}

#[spec(behavior = "dispatch_incremental_validators", verify = "kind with incremental=false triggers full graph validation for that kind")]
#[test]
fn kind_with_incremental_false_triggers_full_graph_for_that_kind() {
    // Extension has both incremental and non-incremental kinds.
    // When a non-incremental kind appears in the delta, full graph is used.
    let validators = vec![ValidatorDescriptor {
        extension_name: "@specforge/software".to_string(),
        kinds: vec![
            KindDescriptor {
                kind_name: "behavior".to_string(),
                incremental: true,
            },
            KindDescriptor {
                kind_name: "type".to_string(),
                incremental: false,
            },
        ],
    }];

    // Delta contains the non-incremental kind "type"
    let delta = delta_with_kinds(&["type"]);
    let graph = Graph::new();
    let plan = plan_incremental_dispatch(&validators, &delta, &graph);

    assert_eq!(plan.entries.len(), 1);
    assert_eq!(
        plan.entries[0].input,
        ValidatorInput::FullGraph,
        "non-incremental kind in delta should trigger full graph"
    );
}

#[spec(behavior = "dispatch_incremental_validators")]
#[test]
fn mixed_kinds_delta_only_incremental_kinds_uses_delta() {
    // Same extension with mixed kinds, but delta only has incremental kinds
    let validators = vec![ValidatorDescriptor {
        extension_name: "@specforge/software".to_string(),
        kinds: vec![
            KindDescriptor {
                kind_name: "behavior".to_string(),
                incremental: true,
            },
            KindDescriptor {
                kind_name: "type".to_string(),
                incremental: false,
            },
        ],
    }];

    // Delta only contains "behavior" (incremental=true)
    let delta = delta_with_kinds(&["behavior"]);
    let graph = Graph::new();
    let plan = plan_incremental_dispatch(&validators, &delta, &graph);

    assert_eq!(plan.entries.len(), 1);
    assert_eq!(
        plan.entries[0].input,
        ValidatorInput::Delta,
        "only incremental kinds in delta should use delta"
    );
}
