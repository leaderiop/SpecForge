//! The multi-hop traceability commands and `feature_impact`.

use super::host::*;
use serde_json::json;
use specforge_test::prelude::*;

// ── multi-hop traceability ────────────────────────────────────────────────

/// [`shipping`] with its references declared in the opposite order.
fn shipping_reversed() -> G {
    let mut g = shipping();
    g.edges.reverse();
    g.nodes.reverse();
    g
}

#[specforge_test(
    behavior = "surface_deliverable_traceability",
    verify = "deliverable-traceability returns DeliverableTraceabilityPayload JSON"
)]
fn deliverable_traceability_answers_its_payload() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(
        dt,
        json!({"deliverable_id": "d1", "transitive_features": ["f1", "f2", "f3"],
            "journey_path_count": 2, "module_path_count": 2})
    );
}

#[specforge_test(
    behavior = "surface_deliverable_traceability",
    verify = "missing deliverable ID returns error with suggestion"
)]
fn deliverable_traceability_of_a_mistyped_deliverable_suggests_the_nearest() {
    let error = run_in(
        &runtime(),
        "deliverable_traceability",
        json!({"deliverable": "dd1"}),
        &shipping(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "d1");
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with journeys and modules returns union of features"
)]
fn a_deliverables_features_are_its_journeys_and_its_modules() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(dt["transitive_features"], json!(["f1", "f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with only journeys returns journey features"
)]
fn a_deliverable_with_only_journeys_has_their_features() {
    let g = shipping()
        .n("d5", "deliverable")
        .edge("d5", "j2", "journeys");
    let dt = json_of("deliverable_traceability", json!({"deliverable": "d5"}), &g);
    assert_eq!(
        (
            dt["transitive_features"].clone(),
            dt["module_path_count"].clone()
        ),
        (json!(["f2"]), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with no journeys or modules returns empty feature set"
)]
fn a_deliverable_with_nothing_reaches_no_feature() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d3"}),
        &shipping(),
    );
    assert_eq!(
        dt,
        json!({"deliverable_id": "d3", "transitive_features": [], "journey_path_count": 0,
            "module_path_count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable with overlapping journey and module features deduplicates"
)]
fn a_feature_reached_both_ways_is_listed_once() {
    let dt = json_of(
        "deliverable_traceability",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    // f1 is on both paths: counted on each, listed once.
    let features = dt["transitive_features"].as_array().unwrap();
    assert_eq!(features.iter().filter(|f| **f == "f1").count(), 1);
    assert_eq!(
        dt["journey_path_count"].as_u64().unwrap() + dt["module_path_count"].as_u64().unwrap(),
        features.len() as u64 + 1
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_traceability",
    verify = "deliverable traceability is deterministic across repeated queries"
)]
fn deliverable_traceability_is_deterministic() {
    let args = json!({"deliverable": "d1"});
    let first = json_of("deliverable_traceability", args.clone(), &shipping());
    assert_eq!(
        json_of("deliverable_traceability", args.clone(), &shipping()),
        first
    );
    assert_eq!(
        json_of("deliverable_traceability", args, &shipping_reversed()),
        first
    );
}

#[specforge_test(
    behavior = "surface_feature_deliverables",
    verify = "feature-deliverables returns FeatureDeliverablePayload JSON"
)]
fn feature_deliverables_answers_its_payload() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f1"}),
        &shipping(),
    );
    assert_eq!(
        fd,
        json!({"feature_id": "f1", "deliverables": ["d1"], "via_journey_count": 1,
            "via_module_count": 1})
    );
}

#[specforge_test(
    behavior = "surface_feature_deliverables",
    verify = "missing feature ID returns error with suggestion"
)]
fn feature_deliverables_of_a_mistyped_feature_suggests_the_nearest() {
    let error = run_in(
        &runtime(),
        "feature_deliverables",
        json!({"feature": "f11"}),
        &shipping(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "f1");
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature reachable via journey path returns deliverable"
)]
fn a_feature_in_a_deliverables_journey_is_in_the_deliverable() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f2"}),
        &shipping(),
    );
    assert_eq!(
        (fd["deliverables"].clone(), fd["via_journey_count"].clone()),
        (json!(["d1"]), json!(1))
    );
    assert_eq!(fd["via_module_count"], 0);
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature reachable via module path returns deliverable"
)]
fn a_feature_in_a_deliverables_module_is_in_the_deliverable() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f3"}),
        &shipping(),
    );
    assert_eq!(
        (fd["deliverables"].clone(), fd["via_module_count"].clone()),
        (json!(["d1", "d2"]), json!(2))
    );
    assert_eq!(fd["via_journey_count"], 0);
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature reachable via both paths deduplicates deliverables"
)]
fn a_deliverable_reached_both_ways_is_listed_once() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f1"}),
        &shipping(),
    );
    assert_eq!(fd["deliverables"], json!(["d1"]));
    assert_eq!(
        (
            fd["via_journey_count"].clone(),
            fd["via_module_count"].clone()
        ),
        (json!(1), json!(1))
    );
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature with no incoming edges returns empty deliverables"
)]
fn a_feature_nothing_holds_is_in_no_deliverable() {
    let fd = json_of(
        "feature_deliverables",
        json!({"feature": "f7"}),
        &shipping(),
    );
    assert_eq!(
        fd,
        json!({"feature_id": "f7", "deliverables": [], "via_journey_count": 0,
            "via_module_count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_deliverables",
    verify = "feature deliverable query is deterministic across repeated queries"
)]
fn feature_deliverables_is_deterministic() {
    let args = json!({"feature": "f3"});
    let first = json_of("feature_deliverables", args.clone(), &shipping());
    assert_eq!(
        json_of("feature_deliverables", args.clone(), &shipping()),
        first
    );
    assert_eq!(
        json_of("feature_deliverables", args, &shipping_reversed()),
        first
    );
}

#[specforge_test(
    behavior = "surface_persona_channels",
    verify = "persona-channels returns PersonaChannelPayload JSON"
)]
fn persona_channels_answers_its_payload() {
    let pc = json_of("persona_channels", json!({"persona": "dev"}), &shipping());
    assert_eq!(
        pc,
        json!({"persona_id": "dev", "channels": ["cli", "web"], "count": 2})
    );
}

#[specforge_test(
    behavior = "surface_persona_channels",
    verify = "missing persona ID returns error with suggestion"
)]
fn persona_channels_of_a_mistyped_persona_suggests_the_nearest() {
    let error = run_in(
        &runtime(),
        "persona_channels",
        json!({"persona": "dve"}),
        &shipping(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "dev");
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona with one journey returns that journey's channels"
)]
fn a_personas_one_journey_gives_its_channels() {
    let pc = json_of("persona_channels", json!({"persona": "ops"}), &shipping());
    assert_eq!(pc["channels"], json!(["web"]));
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona with multiple journeys returns deduplicated channels"
)]
fn a_personas_journeys_give_each_channel_once() {
    // j1 and j2 both use cli.
    let pc = json_of("persona_channels", json!({"persona": "dev"}), &shipping());
    assert_eq!(
        (pc["channels"].clone(), pc["count"].clone()),
        (json!(["cli", "web"]), json!(2))
    );
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona with no journeys returns empty channel list"
)]
fn a_persona_without_journeys_uses_no_channel() {
    let pc = json_of("persona_channels", json!({"persona": "loner"}), &shipping());
    assert_eq!(
        pc,
        json!({"persona_id": "loner", "channels": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_persona_channels",
    verify = "persona channels query is deterministic across repeated queries"
)]
fn persona_channels_is_deterministic() {
    let args = json!({"persona": "dev"});
    let first = json_of("persona_channels", args.clone(), &shipping());
    assert_eq!(
        json_of("persona_channels", args.clone(), &shipping()),
        first
    );
    assert_eq!(
        json_of("persona_channels", args, &shipping_reversed()),
        first
    );
}

#[specforge_test(
    behavior = "surface_deliverable_personas",
    verify = "deliverable-personas returns DeliverablePersonaPayload JSON"
)]
fn deliverable_personas_answers_its_payload() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(
        dp,
        json!({"deliverable_id": "d1", "personas": ["dev"], "via_journey_ids": ["j1", "j2"],
            "count": 1})
    );
}

#[specforge_test(
    behavior = "surface_deliverable_personas",
    verify = "missing deliverable ID returns error with suggestion"
)]
fn deliverable_personas_of_a_mistyped_deliverable_suggests_the_nearest() {
    let error = run_in(
        &runtime(),
        "deliverable_personas",
        json!({"deliverable": "d9"}),
        &shipping(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "d1");
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with one journey and one persona returns that persona"
)]
fn a_deliverables_journey_gives_its_persona() {
    let g = shipping()
        .n("d6", "deliverable")
        .edge("d6", "j3", "journeys");
    let dp = json_of("deliverable_personas", json!({"deliverable": "d6"}), &g);
    assert_eq!(
        (dp["personas"].clone(), dp["count"].clone()),
        (json!(["ops"]), json!(1))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with multiple journeys sharing a persona deduplicates"
)]
fn a_persona_shared_by_journeys_is_listed_once() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d1"}),
        &shipping(),
    );
    assert_eq!(dp["personas"], json!(["dev"]));
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with no journeys returns empty list"
)]
fn a_deliverable_without_journeys_serves_no_persona() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d2"}),
        &shipping(),
    );
    assert_eq!(
        dp,
        json!({"deliverable_id": "d2", "personas": [], "via_journey_ids": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "deliverable with journeys without personas returns empty list"
)]
fn a_deliverable_whose_journeys_target_no_one_serves_no_persona() {
    let dp = json_of(
        "deliverable_personas",
        json!({"deliverable": "d4"}),
        &shipping(),
    );
    assert_eq!(
        (dp["personas"].clone(), dp["count"].clone()),
        (json!([]), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_deliverable_personas",
    verify = "result includes via_journey_ids for traceability"
)]
fn deliverable_personas_name_the_journeys_between() {
    let g = shipping().edge("d1", "j3", "journeys");
    let dp = json_of("deliverable_personas", json!({"deliverable": "d1"}), &g);
    assert_eq!(dp["personas"], json!(["dev", "ops"]));
    assert_eq!(dp["via_journey_ids"], json!(["j1", "j2", "j3"]));
}

#[specforge_test(
    behavior = "product_deliverable_persona_correctness",
    verify = "via_journey_ids includes all intermediate journeys"
)]
fn every_journey_between_a_deliverable_and_a_persona_is_named() {
    // d1's journeys j1 and j2 both reach dev; j5 (on d4) reaches no one.
    let g = shipping().edge("d1", "j5", "journeys");
    let dp = json_of("deliverable_personas", json!({"deliverable": "d1"}), &g);
    assert_eq!(dp["via_journey_ids"], json!(["j1", "j2"]));
}

#[specforge_test(
    behavior = "product_deliverable_persona_correctness",
    verify = "traversal order does not affect result"
)]
fn deliverable_personas_do_not_depend_on_declaration_order() {
    let args = json!({"deliverable": "d1"});
    assert_eq!(
        json_of("deliverable_personas", args.clone(), &shipping_reversed()),
        json_of("deliverable_personas", args, &shipping())
    );
}

// ── feature impact ────────────────────────────────────────────────────────

#[specforge_test(
    behavior = "surface_feature_impact",
    verify = "feature-impact returns FeatureImpactPayload JSON"
)]
fn feature_impact_answers_its_payload() {
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &shipping());
    assert_eq!(
        fi,
        json!({"feature_id": "f1", "affected_journeys": ["j1"], "affected_milestones": ["ms1"],
            "affected_deliverables": ["d1"], "affected_modules": ["mod1"],
            "dependent_features": ["f4", "f5"], "total_affected_entities": 6})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "feature in one journey and one milestone returns both as affected"
)]
fn a_feature_in_a_journey_and_a_milestone_affects_both() {
    let g = G::default()
        .n("f1", "feature")
        .n("j1", "journey")
        .n("ms1", "milestone")
        .edge("j1", "f1", "features")
        .edge("ms1", "f1", "features");
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &g);
    assert_eq!(fi["affected_journeys"], json!(["j1"]));
    assert_eq!(fi["affected_milestones"], json!(["ms1"]));
    assert_eq!(fi["total_affected_entities"], 2);
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "feature with dependent features includes transitive dependents"
)]
fn a_features_dependents_include_their_dependents() {
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &shipping());
    // f5 depends on f4, which depends on f1; f6 only relates to f1.
    assert_eq!(fi["dependent_features"], json!(["f4", "f5"]));
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "feature with no references returns zero affected entities"
)]
fn a_feature_nothing_references_affects_nothing() {
    let fi = json_of("feature_impact", json!({"feature": "f7"}), &shipping());
    assert_eq!(
        fi,
        json!({"feature_id": "f7", "affected_journeys": [], "affected_milestones": [],
            "affected_deliverables": [], "affected_modules": [], "dependent_features": [],
            "total_affected_entities": 0})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "affected deliverables found via both journey and module paths"
)]
fn a_features_deliverables_come_through_journeys_and_modules() {
    // d7 holds f3 only through its journey j2, d2 only through mod2.
    let g = shipping()
        .n("d7", "deliverable")
        .edge("d7", "j2", "journeys")
        .edge("j2", "f3", "features");
    let fi = json_of("feature_impact", json!({"feature": "f3"}), &g);
    assert_eq!(fi["affected_deliverables"], json!(["d1", "d2", "d7"]));
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "total_affected_entities is deduplicated count"
)]
fn the_total_counts_each_affected_entity_once() {
    // d1 holds f1 through j1 and through mod1: one deliverable.
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &shipping());
    let listed: usize = [
        "affected_journeys",
        "affected_milestones",
        "affected_deliverables",
        "affected_modules",
        "dependent_features",
    ]
    .iter()
    .map(|k| fi[*k].as_array().unwrap().len())
    .sum();
    assert_eq!(fi["total_affected_entities"], listed);
    assert_eq!(fi["affected_deliverables"], json!(["d1"]));
}

#[specforge_test(
    behavior = "product_impact_query_correctness",
    verify = "feature impact returns complete transitive closure"
)]
fn feature_impact_follows_dependencies_to_the_end() {
    // A chain f1 <- c1 <- c2 <- c3, and a cycle back to c1.
    let g = shipping()
        .n("c1", "feature")
        .n("c2", "feature")
        .n("c3", "feature")
        .edge("c1", "f1", "depends_on")
        .edge("c2", "c1", "depends_on")
        .edge("c3", "c2", "depends_on")
        .edge("c1", "c3", "depends_on");
    let fi = json_of("feature_impact", json!({"feature": "f1"}), &g);
    assert_eq!(
        fi["dependent_features"],
        json!(["c1", "c2", "c3", "f4", "f5"])
    );
}

#[specforge_test(
    behavior = "product_impact_query_correctness",
    verify = "traversal order does not affect results"
)]
fn impact_and_persona_features_do_not_depend_on_declaration_order() {
    for (id, args) in [
        ("feature_impact", json!({"feature": "f1"})),
        ("persona_features", json!({"persona": "dev"})),
    ] {
        assert_eq!(
            json_of(id, args.clone(), &shipping_reversed()),
            json_of(id, args, &shipping()),
            "{id}"
        );
    }
}
