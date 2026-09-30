use specforge_graph::Graph;
use specforge_test_macros::test as spec;
use specforge_watch::{
    GraphDelta, KindDescriptor, NodeChange, ValidatorDescriptor, ValidatorInput,
    plan_incremental_dispatch,
};

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

#[spec(
    behavior = "dispatch_incremental_validators",
    verify = "incremental extension receives delta only"
)]
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

#[spec(
    behavior = "dispatch_incremental_validators",
    verify = "non-incremental extension receives full graph"
)]
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

#[spec(
    behavior = "dispatch_incremental_validators",
    verify = "dispatch follows topological order"
)]
fn dispatch_preserves_topological_order() {
    // Validators arrive in topological extension order (dependencies first,
    // as specforge-wasm's topological_sort_extensions yields them). The order
    // is deliberately neither alphabetical nor grouped by input type, and
    // the deltas below list kinds in other orders: the plan must keep the
    // topological order regardless of delta content.
    let ext = |name: &str, kind: &str, incremental: bool| ValidatorDescriptor {
        extension_name: name.to_string(),
        kinds: vec![KindDescriptor {
            kind_name: kind.to_string(),
            incremental,
        }],
    };
    let validators = vec![
        ext("@specforge/software", "behavior", true),    // base
        ext("@specforge/governance", "decision", false), // depends on software
        ext("@specforge/product", "journey", true),      // depends on software
        ext("@specforge/formal", "refinement", false),   // depends on governance
    ];
    let topological = vec![
        "@specforge/software",
        "@specforge/governance",
        "@specforge/product",
        "@specforge/formal",
    ];
    let graph = Graph::new();

    for kinds in [
        &["refinement", "journey", "decision", "behavior"][..],
        &["journey"][..],
        &["refinement", "behavior"][..],
    ] {
        let plan = plan_incremental_dispatch(&validators, &delta_with_kinds(kinds), &graph);
        let names: Vec<&str> = plan
            .entries
            .iter()
            .map(|e| e.extension_name.as_str())
            .collect();
        assert_eq!(names, topological, "delta kinds {kinds:?}");
    }
}

#[spec(
    behavior = "dispatch_incremental_validators",
    verify = "mixed incremental and non-incremental kinds dispatch separately"
)]
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

#[spec(
    behavior = "dispatch_incremental_validators",
    verify = "kind with incremental=false triggers full graph validation for that kind"
)]
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
