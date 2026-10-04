use crate::queries::*;
use specforge_extension_sdk::prelude::{
    CommandFormat, CommandGraph, CommandInput, GraphEdge, GraphNode,
};

/// A graph built entity by entity.
#[derive(Default)]
struct G {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

impl G {
    fn node(mut self, id: &str, kind: &str, fields: &[(&str, &str)]) -> Self {
        self.nodes.push(GraphNode {
            id: id.to_string(),
            kind: kind.to_string(),
            title: Some(id.to_string()),
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
                .collect(),
        });
        self
    }

    fn n(self, id: &str, kind: &str) -> Self {
        self.node(id, kind, &[])
    }

    fn edge(mut self, source: &str, target: &str, label: &str) -> Self {
        self.edges.push(GraphEdge {
            source: source.to_string(),
            target: target.to_string(),
            label: label.to_string(),
        });
        self
    }

    fn build(self) -> CommandGraph {
        CommandGraph::new(self.nodes, self.edges)
    }
}

/// The ids `filter` lists over `g`, in order, as features.
fn listed(g: &CommandGraph, filter: &ListFilter) -> Vec<String> {
    list::<FeatureListEntry>(g, filter)
        .items
        .into_iter()
        .map(|e| e.id)
        .collect()
}

// ── lists ──────────────────────────────────────────────────────────────────

#[test]
fn a_list_is_of_its_kind_sorted_by_id() {
    let g = G::default()
        .n("z_feature", "feature")
        .n("a_feature", "feature")
        .n("b1", "behavior")
        .build();
    assert_eq!(
        listed(&g, &ListFilter::all(&FEATURES)),
        ["a_feature", "z_feature"]
    );
    assert_eq!(
        list::<FeatureListEntry>(&CommandGraph::default(), &ListFilter::all(&FEATURES)).total,
        0
    );
}

#[test]
fn list_filters_combine_and_an_absent_status_is_the_first() {
    let g = G::default()
        .node("f1", "feature", &[("status", "done"), ("priority", "high")])
        .node("f2", "feature", &[("priority", "high")])
        .node("f3", "feature", &[("status", "done"), ("priority", "low")])
        .build();
    let with = |equals: Vec<(&'static str, &'static str)>| {
        let mut filter = ListFilter::all(&FEATURES);
        filter.equals = equals;
        listed(&g, &filter)
    };
    assert_eq!(with(vec![("status", "done")]), ["f1", "f3"]);
    assert_eq!(with(vec![("status", "done"), ("priority", "high")]), ["f1"]);
    assert_eq!(with(vec![("status", "proposed")]), ["f2"]);
}

#[test]
fn a_reference_filter_matches_the_field_or_the_edge() {
    let g = G::default()
        .node("j1", "journey", &[("persona", "dev")])
        .n("j2", "journey")
        .n("j3", "journey")
        .edge("j2", "dev", "persona")
        .build();
    let mut filter = ListFilter::all(&JOURNEYS);
    filter.equals = vec![("persona", "dev")];
    let ids: Vec<String> = list::<JourneyListEntry>(&g, &filter)
        .items
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, ["j1", "j2"]);
}

#[test]
fn a_list_sorts_an_enum_by_its_order_and_ties_by_id() {
    let g = G::default()
        .node("c", "feature", &[("priority", "low")])
        .node("b", "feature", &[("priority", "critical")])
        .node("a", "feature", &[("priority", "low")])
        .n("d", "feature")
        .build();
    let mut filter = ListFilter::all(&FEATURES);
    filter.sort_by = "priority";
    assert_eq!(listed(&g, &filter), ["b", "a", "c", "d"]);
    filter.descending = true;
    // Ties stay by id ascending; an entity without the field stays last.
    assert_eq!(listed(&g, &filter), ["a", "c", "b", "d"]);
    assert!(sortable("feature", "priority"));
    assert!(sortable("feature", "tags"));
    assert!(!sortable("feature", "nope"));
    assert!(!sortable("term", "status"));
}

#[test]
fn a_page_counts_before_paging_and_clamps_its_limit() {
    let page = paginate((0..5).collect(), Some(1), Some(2));
    assert_eq!(
        (page.items, page.total, page.has_more),
        (vec![1, 2], 5, true)
    );
    let page = paginate((0..5).collect::<Vec<i32>>(), Some(3), Some(2));
    assert!(!page.has_more);
    assert_eq!(
        paginate((0..5).collect::<Vec<i32>>(), None, Some(0)).limit,
        1
    );
    assert_eq!(
        paginate((0..5).collect::<Vec<i32>>(), None, Some(5000)).limit,
        1000
    );
    assert_eq!(
        paginate((0..5).collect::<Vec<i32>>(), None, None).limit,
        100
    );
    let past = paginate((0..5).collect::<Vec<i32>>(), Some(9), None);
    assert_eq!((past.items.len(), past.total, past.has_more), (0, 5, false));
}

#[test]
fn list_entries_count_their_references() {
    let g = G::default()
        .node("j1", "journey", &[("persona", "dev")])
        .n("dev", "persona")
        .n("cli", "channel")
        .n("f1", "feature")
        .n("mod1", "module")
        .n("mod2", "module")
        .edge("j1", "dev", "persona")
        .edge("j1", "cli", "channels")
        .edge("j1", "f1", "features")
        .edge("mod1", "f1", "features")
        .edge("mod1", "mod2", "depends_on")
        .build();
    let j = &list::<JourneyListEntry>(&g, &ListFilter::all(&JOURNEYS)).items[0];
    assert_eq!((j.channel_count, j.feature_count), (1, 1));
    let p = &list::<PersonaListEntry>(&g, &ListFilter::all(&PERSONAS)).items[0];
    assert_eq!(p.journey_count, 1);
    let c = &list::<ChannelListEntry>(&g, &ListFilter::all(&CHANNELS)).items[0];
    assert_eq!(c.journey_count, 1);
    let m = &list::<ModuleListEntry>(&g, &ListFilter::all(&MODULES)).items[0];
    assert_eq!(
        (m.feature_count, m.depends_on.clone()),
        (1, vec!["mod2".to_string()])
    );
}

// ── milestones and journeys ────────────────────────────────────────────────

#[test]
fn milestone_completion_counts_done_features_in_declared_order() {
    let mut g = G::default()
        .node("f2", "feature", &[("status", "done")])
        .node("f1", "feature", &[("status", "draft")])
        .edge("ms1", "f1", "features")
        .edge("ms1", "f2", "features");
    g.nodes.push(GraphNode {
        id: "ms1".into(),
        kind: "milestone".into(),
        title: None,
        fields: [
            ("status".to_string(), serde_json::json!("active")),
            ("features".to_string(), serde_json::json!(["f2", "f1"])),
        ]
        .into_iter()
        .collect(),
    });
    let mc = milestone_completion(&g.build(), "ms1").unwrap();
    assert_eq!(mc.total_features, 2);
    assert_eq!(mc.done_count, 1);
    assert_eq!(mc.done_features, ["f2"]);
    assert!((mc.completion_ratio - 0.5).abs() < 1e-9);
    assert_eq!(mc.status.as_deref(), Some("active"));
    let ids: Vec<&str> = mc.features.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["f2", "f1"]);
}

#[test]
fn milestone_completion_of_a_milestone_only() {
    let g = G::default()
        .n("f1", "feature")
        .n("ms1", "milestone")
        .build();
    assert!(milestone_completion(&g, "nonexistent").is_none());
    assert!(milestone_completion(&g, "f1").is_none());
    let empty = milestone_completion(&g, "ms1").unwrap();
    assert_eq!(empty.total_features, 0);
    assert_eq!(empty.completion_ratio, 0.0);
}

#[test]
fn journey_coverage_counts_done_features_not_module_ownership() {
    let g = G::default()
        .node("j1", "journey", &[("persona", "dev")])
        .n("f1", "feature")
        .node("f2", "feature", &[("status", "done")])
        .n("mod1", "module")
        .edge("j1", "f1", "features")
        .edge("j1", "f2", "features")
        .edge("mod1", "f1", "features")
        .build();
    let jc = journey_coverage(&g, "j1").unwrap();
    assert_eq!(jc.total_features, 2);
    assert_eq!(jc.covered_count, 1);
    assert_eq!(jc.uncovered_features, ["f1"]);
    assert_eq!(jc.persona.as_deref(), Some("dev"));
    assert!(journey_coverage(&g, "f1").is_none());
    assert!(journey_coverage(&g, "nonexistent").is_none());
}

// ── features ───────────────────────────────────────────────────────────────

#[test]
fn feature_impact_groups_references_by_kind() {
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .n("j1", "journey")
        .n("ms1", "milestone")
        .n("mod1", "module")
        .edge("j1", "f1", "features")
        .edge("ms1", "f1", "features")
        .edge("mod1", "f1", "features")
        .edge("f1", "f2", "depends_on")
        .edge("f3", "f1", "depends_on")
        .build();
    let fi = feature_impact(&g, "f1").unwrap();
    assert_eq!(fi.referenced_by_journeys, ["j1"]);
    assert_eq!(fi.referenced_by_milestones, ["ms1"]);
    assert_eq!(fi.referenced_by_modules, ["mod1"]);
    assert_eq!(fi.depends_on, ["f2"]);
    assert_eq!(fi.depended_on_by, ["f3"]);
    assert!(feature_impact(&g, "j1").is_none());
    assert!(feature_impact(&g, "nonexistent").is_none());
}

#[test]
fn a_feature_that_only_relates_to_the_feature_is_not_a_dependent() {
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .edge("f2", "f1", "features")
        .edge("f3", "f1", "depends_on")
        .build();
    let fi = feature_impact(&g, "f1").unwrap();
    assert_eq!(fi.depended_on_by, ["f3"]);
}

#[test]
fn feature_dependents_are_the_depends_on_sources() {
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .edge("f2", "f1", "depends_on")
        .edge("f3", "f1", "depends_on")
        .build();
    let fd = feature_dependents(&g, "f1").unwrap();
    assert_eq!(
        (fd.dependents, fd.count),
        (vec!["f2".to_string(), "f3".into()], 2)
    );
    assert!(feature_dependents(&g, "nonexistent").is_none());
}

#[test]
fn persona_features_follow_the_field_or_the_reference() {
    let by_field = G::default()
        .n("p1", "persona")
        .node("j1", "journey", &[("persona", "p1")])
        .n("f1", "feature")
        .edge("j1", "f1", "features")
        .build();
    assert_eq!(persona_features(&by_field, "p1").unwrap().features, ["f1"]);
    let by_edge = G::default()
        .n("p1", "persona")
        .n("j1", "journey")
        .n("f1", "feature")
        .edge("j1", "p1", "persona")
        .edge("j1", "f1", "features")
        .build();
    let pf = persona_features(&by_edge, "p1").unwrap();
    assert_eq!(
        (pf.features, pf.via_journey_ids),
        (vec!["f1".to_string()], vec!["j1".into()])
    );
    assert!(persona_features(&by_edge, "f1").is_none());
    assert!(persona_features(&by_edge, "nonexistent").is_none());
}

#[test]
fn channel_features_deduplicate_across_journeys() {
    let g = G::default()
        .n("ch1", "channel")
        .n("j1", "journey")
        .n("j2", "journey")
        .n("f1", "feature")
        .edge("j1", "ch1", "channels")
        .edge("j2", "ch1", "channels")
        .edge("j1", "f1", "features")
        .edge("j2", "f1", "features")
        .build();
    let cf = channel_features(&g, "ch1").unwrap();
    assert_eq!(cf.features, ["f1"]);
    assert_eq!(cf.via_journey_ids, ["j1", "j2"]);
    assert_eq!(cf.count, 1);
    assert!(channel_features(&g, "f1").is_none());
    assert!(channel_features(&g, "nonexistent").is_none());
}

// ── project-wide ───────────────────────────────────────────────────────────

#[test]
fn bulk_status_aggregates_by_kind() {
    let g = G::default()
        .node("f1", "feature", &[("status", "done")])
        .node("f2", "feature", &[("status", "done")])
        .n("f3", "feature")
        .node("ms1", "milestone", &[("status", "active")])
        .build();
    let results = bulk_status(&g).kinds;
    let feat = results.iter().find(|r| r.kind == "feature").unwrap();
    assert_eq!(feat.total, 3);
    let counts: Vec<(&str, usize)> = feat
        .by_status
        .iter()
        .map(|s| (s.status.as_str(), s.count))
        .collect();
    assert_eq!(counts, [("(none)", 1), ("done", 2)]);
    assert_eq!(
        results
            .iter()
            .find(|r| r.kind == "milestone")
            .unwrap()
            .total,
        1
    );
    assert!(bulk_status(&CommandGraph::default()).kinds.is_empty());
}

#[test]
fn project_health_scores_coverage_and_completeness() {
    assert!((project_health(&CommandGraph::default()).score.overall - 100.0).abs() < 0.01);
    let g = G::default()
        .node("f1", "feature", &[("status", "done")])
        .n("f2", "feature")
        .n("j1", "journey")
        .n("ms1", "milestone")
        .n("ms2", "milestone")
        .edge("j1", "f1", "features")
        .edge("ms1", "f1", "features")
        .build();
    let report = project_health(&g);
    let count = |kind: &str| {
        report
            .entity_counts
            .iter()
            .find(|c| c.kind == kind)
            .unwrap()
            .count
    };
    assert_eq!((count("feature"), count("journey")), (2, 1));
    let orphans = report
        .orphan_counts
        .iter()
        .find(|c| c.kind == "feature")
        .unwrap();
    assert_eq!((orphans.orphans, orphans.total), (1, 2));
    assert_eq!(report.completeness.features_with_status, 1);
    assert_eq!(report.completeness.features_total, 2);
    assert_eq!(report.completeness.milestones_with_features, 1);
    assert_eq!(report.completeness.milestones_total, 2);
}

// ── commands ───────────────────────────────────────────────────────────────

fn input(args: serde_json::Value, graph: CommandGraph) -> CommandInput {
    CommandInput {
        args: args.as_object().unwrap().clone(),
        cwd: "/p".into(),
        graph,
        ..Default::default()
    }
}

/// [`input`], asked for json.
fn json_input(args: serde_json::Value, graph: CommandGraph) -> CommandInput {
    CommandInput {
        format: CommandFormat::Json,
        ..input(args, graph)
    }
}

fn sample() -> CommandGraph {
    G::default()
        .node(
            "f1",
            "feature",
            &[("status", "proposed"), ("priority", "high")],
        )
        .node("f2", "feature", &[("status", "done")])
        .build()
}

#[test]
fn a_list_command_renders_human_by_default_and_json_on_request() {
    let human = crate::commands::run(
        "cmd__product_features",
        &input(serde_json::json!({}), sample()),
    )
    .unwrap();
    assert_eq!(human.exit_code, 0);
    assert_eq!(
        human.stdout,
        "id  title  status    priority\nf1  f1     proposed  high\nf2  f2     done      -\n"
    );
    let paged = crate::commands::run(
        "cmd__product_features",
        &input(serde_json::json!({"limit": 1}), sample()),
    )
    .unwrap();
    assert!(
        paged
            .stdout
            .ends_with("1 of 2 features; --offset 1 for more\n"),
        "{}",
        paged.stdout
    );
    let json = crate::commands::run(
        "cmd__product_features",
        &json_input(serde_json::json!({"status": "done"}), sample()),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json.stdout).unwrap();
    assert_eq!(value["total"], 1);
    assert_eq!(value["features"][0]["id"], "f2");
    assert!(value["features"][0].get("priority").is_none());
}

#[test]
fn a_list_command_refuses_an_arg_it_cannot_use() {
    for (args, says) in [
        (
            serde_json::json!({"limit": -1}),
            "must be a non-negative integer",
        ),
        (
            serde_json::json!({"offset": "many"}),
            "must be a non-negative integer",
        ),
        (
            serde_json::json!({"status": "draft"}),
            "status must be one of proposed,",
        ),
        (
            serde_json::json!({"priority": "urgent"}),
            "priority must be one of",
        ),
        (
            serde_json::json!({"sort_by": "nope"}),
            "a feature has no field 'nope'",
        ),
        (
            serde_json::json!({"sort_order": "up"}),
            "sort_order must be one of asc, desc",
        ),
    ] {
        let out =
            crate::commands::run("cmd__product_features", &input(args.clone(), sample())).unwrap();
        assert_eq!(out.exit_code, 2, "{args}");
        assert_eq!(out.stdout, "", "{args}");
        assert!(out.stderr.contains(says), "{args}: {}", out.stderr);
    }
}

#[test]
fn a_query_about_a_missing_entity_fails_on_stderr() {
    let out = crate::commands::run(
        "cmd__product_milestone_completion",
        &input(serde_json::json!({"milestone": "nope"}), sample()),
    )
    .unwrap();
    assert_eq!(out.exit_code, 1);
    assert_eq!(out.stdout, "");
    assert_eq!(out.stderr, "error: milestone 'nope' not found\n");
}

#[test]
fn an_id_list_says_none_when_empty() {
    let out = crate::commands::run(
        "cmd__product_feature_dependents",
        &input(serde_json::json!({"feature": "f1"}), sample()),
    )
    .unwrap();
    assert_eq!(out.stdout, "Features depending on 'f1':\n  (none)\n");
}

#[test]
fn every_declared_command_has_its_export() {
    let surfaces: serde_json::Value =
        serde_json::from_slice(include_bytes!("describe_surfaces.json")).unwrap();
    let commands = surfaces["items"][0]["commands"].as_array().unwrap();
    assert_eq!(commands.len(), 17);
    for command in commands {
        let export = command["export"].as_str().unwrap();
        assert_eq!(
            export,
            format!("cmd__product_{}", command["id"].as_str().unwrap())
        );
        let args = serde_json::json!({"milestone": "x", "journey": "x",
            "feature": "x", "persona": "x", "channel": "x"});
        assert!(
            crate::commands::run(export, &json_input(args, sample())).is_some(),
            "{export} is declared but not exported"
        );
    }
    assert!(
        crate::commands::run("cmd__product_nope", &input(serde_json::json!({}), sample()))
            .is_none()
    );
}

// ── errors and tables ──────────────────────────────────────────────────────

#[test]
fn the_nearest_id_of_the_kind_within_two_edits_is_suggested() {
    let g = G::default()
        .n("launch", "milestone")
        .n("lunch", "milestone")
        .n("launchx", "feature")
        .build();
    assert_eq!(suggest(&g, "milestone", "launc").as_deref(), Some("launch"));
    // Equally near: the first by id.
    assert_eq!(
        suggest(&g, "milestone", "laanch").as_deref(),
        Some("launch")
    );
    assert_eq!(suggest(&g, "milestone", "lnch").as_deref(), Some("lunch"));
    assert_eq!(suggest(&g, "milestone", "dinner"), None);
    // Only ids of the kind asked about.
    assert_eq!(suggest(&g, "feature", "launch").as_deref(), Some("launchx"));
    assert_eq!(suggest(&g, "journey", "launch"), None);
    let error = not_found(&g, "milestone", "launc");
    assert_eq!(error.code, "ENTITY_NOT_FOUND");
    assert_eq!(error.entity_id.as_deref(), Some("launc"));
    assert_eq!(error.suggestion.as_deref(), Some("launch"));
    assert_eq!(invalid_input("bad").code, "INVALID_INPUT");
}

#[test]
fn an_error_is_json_on_stderr_when_json_was_asked_for() {
    let out = crate::commands::run(
        "cmd__product_milestone_completion",
        &json_input(serde_json::json!({"milestone": "nope"}), sample()),
    )
    .unwrap();
    assert_eq!((out.exit_code, out.stdout.as_str()), (1, ""));
    let error: serde_json::Value = serde_json::from_str(&out.stderr).unwrap();
    assert_eq!(
        error,
        serde_json::json!({"code": "ENTITY_NOT_FOUND", "message": "milestone 'nope' not found",
            "entity_id": "nope"})
    );
    let out = crate::commands::run(
        "cmd__product_features",
        &json_input(serde_json::json!({"limit": -1}), sample()),
    )
    .unwrap();
    assert_eq!(out.exit_code, 2);
    let error: serde_json::Value = serde_json::from_str(&out.stderr).unwrap();
    assert_eq!(error["code"], "INVALID_INPUT");
}

#[test]
fn a_table_aligns_its_columns_under_a_header() {
    let rows = vec![
        vec!["feature".to_string(), "done".into(), "12".into()],
        vec!["milestone".to_string(), "in_progress".into(), "3".into()],
    ];
    assert_eq!(
        crate::commands::table(&["kind", "status", "count"], &rows),
        "kind       status       count\nfeature    done         12\nmilestone  in_progress  3\n"
    );
    assert_eq!(crate::commands::table(&["id"], &[]), "id\n");
}
