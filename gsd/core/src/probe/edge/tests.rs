//! Ported from gsd-core v1.19.0 tests/edge-probe.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! edge-probe.test.cjs's describe blocks, one test each, on the helpers and the case roster of the engine's tests
//! (`super::super::tests`).

use serde_json::json;

use crate::probe::edge::{
    EDGE_VALIDATORS, EdgeRequirement, SHAPE_CUES, Shape, TAXONOMY, UNCLASSIFIED,
    UNCLASSIFIED_PROBE, applicable_categories, classify_shape, propose_edges,
};
use crate::probe::tests::*;
use crate::probe::*;

const EDGE_PROBE_SOURCE: &str = include_str!(gsd_kit::snapshot!("src/edge-probe.cts"));
const REFERENCE: &str = include_str!("../../../assets/gsd-core/references/edge-probe.md");

#[test]
fn classify_shape_reads_the_cues() {
    assert_eq!(
        classify_shape("Round a number to N decimal places"),
        [Shape::NumericRange]
    );
    assert!(classify_shape("Merge a list of overlapping intervals").contains(&Shape::Collection));
    assert!(classify_shape("Truncate a string to a maximum length").contains(&Shape::Text));
    assert_eq!(classify_shape("Display the company logo"), []);
}

#[test]
fn the_taxonomy_raises_its_categories() {
    let ids: Vec<&str> = TAXONOMY.iter().map(|c| c.id).collect();
    assert_eq!(
        ids,
        [
            "boundary",
            "adjacency",
            "empty",
            "encoding",
            "ordering",
            "precision",
            "idempotency",
            "concurrency"
        ]
    );
    assert!(
        TAXONOMY
            .iter()
            .all(|c| !c.name.is_empty() && !c.shapes.is_empty() && !c.probe.is_empty())
    );
    let sorted = |shapes: &[Shape]| {
        let mut ids = applicable_categories(shapes);
        ids.sort_unstable();
        ids
    };
    assert_eq!(sorted(&[Shape::NumericRange]), ["boundary", "precision"]);
    assert_eq!(
        sorted(&[Shape::Collection]),
        ["adjacency", "empty", "ordering"]
    );
    assert_eq!(sorted(&[Shape::Text]), ["empty", "encoding"]);
    assert_eq!(sorted(&[]), Vec::<&str>::new());
}

#[test]
fn propose_edges_proposes_unresolved_rows() {
    let edges = propose_edges(&req("R1", "Round a number to N decimal places")).expect("edges");
    assert_eq!(categories(&edges), ["boundary", "precision"]);
    for e in &edges {
        assert_eq!(e.requirement_id, "R1");
        assert_eq!(e.status, "unresolved");
        assert_eq!(
            (&e.verification, &e.resolution, &e.reason),
            (&None, &None, &None)
        );
    }
    let edges = propose_edges(&shaped("R9", Some("opaque label"), &["collection"])).expect("edges");
    assert_eq!(categories(&edges), ["adjacency", "empty", "ordering"]);
}

#[test]
fn edge_validate_resolution_takes_the_edge_tiers() {
    let v = |r: Resolution| validate_resolution(&r, &EDGE_VALIDATORS);
    refused(
        v(res("boundary", "maybe", None, None, None)),
        &["invalid status"],
    );
    refused(
        v(res("boundary", "covered", None, Some("x"), None)),
        &["invalid status"],
    );
    refused(
        v(res("boundary", "dismissed", None, None, Some(""))),
        &["dismissed requires a reason"],
    );
    assert_eq!(
        v(res(
            "boundary",
            "dismissed",
            None,
            None,
            Some("bounded enum")
        )),
        Ok(())
    );
    refused(
        v(res("boundary", "resolved", None, Some("AC"), None)),
        &["verification"],
    );
    refused(
        v(res(
            "boundary",
            "resolved",
            Some("judgment"),
            Some("AC"),
            None,
        )),
        &["invalid verification"],
    );
}

const INTERVALS: &str = "Merge a list of overlapping intervals";

#[test]
fn edge_analyze_coverage_merges_resolutions() {
    let reqs = [req("R1", INTERVALS)];
    let rep = edge::analyze_coverage(&reqs, &[]).expect("a report");
    assert_eq!(
        coverage(&rep),
        json!({"applicable": 3, "resolved": 0, "unresolved": 3, "unclassified": 0,
               "byVerification": {"explicit": 0, "backstop": 0}})
    );
    let touching = "AC#6: touching intervals merge";
    let rep = edge::analyze_coverage(
        &reqs,
        &[res(
            "adjacency",
            "resolved",
            Some("explicit"),
            Some(touching),
            None,
        )],
    )
    .expect("a report");
    let adj = rep
        .items
        .iter()
        .find(|i| i.category == "adjacency")
        .expect("adjacency");
    assert_eq!(
        (
            adj.status.as_str(),
            adj.verification.as_deref(),
            adj.resolution.as_deref()
        ),
        ("resolved", Some("explicit"), Some(touching))
    );
    assert_eq!((rep.coverage.resolved, rep.coverage.unresolved), (1, 2));
    assert_eq!(
        coverage(&rep)["byVerification"],
        json!({"explicit": 1, "backstop": 0})
    );
    refused(
        edge::analyze_coverage(&reqs, &[res("empty", "dismissed", None, None, None)]),
        &["dismissed requires a reason"],
    );
}

#[test]
fn an_empty_shapes_list_proposes_nothing() {
    assert_eq!(
        propose_edges(&shaped("R1", Some("merge intervals"), &[])),
        Ok(vec![])
    );
    assert!(
        !propose_edges(&req("R1", "merge intervals"))
            .expect("edges")
            .is_empty()
    );
    let edges = propose_edges(&shaped(
        "R9",
        Some("opaque text with no cues"),
        &["collection"],
    ))
    .expect("edges");
    assert_eq!(categories(&edges), ["adjacency", "empty", "ordering"]);
}

#[test]
fn prose_with_no_cue_is_one_unclassified_row() {
    let logo = "Display the company logo";
    let edges = propose_edges(&req("R1", logo)).expect("edges");
    assert_eq!(
        Value::Array(edges.iter().map(Item::to_json).collect()),
        json!([{"requirement_id": "R1", "category": "unclassified", "status": "unresolved",
                "verification": null, "resolution": null, "reason": null,
                "probe": "unclassified — review manually"}])
    );
    assert_eq!(
        (UNCLASSIFIED, UNCLASSIFIED_PROBE),
        ("unclassified", "unclassified — review manually")
    );
    assert!(EDGE_VALIDATORS.categories.contains(&"unclassified"));
    assert!(!TAXONOMY.iter().any(|c| c.id == "unclassified"));
    assert_eq!(propose_edges(&shaped("R1", Some(logo), &[])), Ok(vec![]));
    let edges = propose_edges(&req("R1", "Round a number to N decimal places")).expect("edges");
    assert!(!edges.is_empty() && edges.iter().all(|e| e.category != "unclassified"));
    let rep = edge::analyze_coverage(&[req("R1", logo)], &[]).expect("a report");
    let c = &rep.coverage;
    assert_eq!((c.applicable, c.unresolved, c.unclassified), (1, 1, 1));
    assert_eq!(rep.items[0].category, "unclassified");
    let rep = edge::analyze_coverage(
        &[req("R1", logo)],
        &[res(
            "unclassified",
            "dismissed",
            None,
            None,
            Some("genuinely edge-free — static asset"),
        )],
    )
    .expect("a report");
    assert_eq!(rep.items[0].status, "dismissed");
}

#[test]
fn an_invalid_authored_shape_fails_closed() {
    refused(
        propose_edges(&shaped("R1", Some("Round a number"), &["numeric"])),
        &["invalid shape"],
    );
    refused(
        propose_edges(&shaped(
            "R1",
            Some("Round a number"),
            &["numeric-range", "bogus"],
        )),
        &["invalid shape"],
    );
    refused(
        edge::analyze_coverage(&[shaped("R1", Some("Round a number"), &["numeric"])], &[]),
        &["invalid shape"],
    );
    let edges = propose_edges(&shaped("R1", Some("opaque"), &["numeric-range"])).expect("edges");
    assert_eq!(categories(&edges), ["boundary", "precision"]);
    assert_eq!(
        propose_edges(&shaped("R1", Some("merge intervals"), &[])),
        Ok(vec![])
    );
}

#[test]
fn inputs_are_checked_and_an_orphan_is_refused() {
    let round = [req("R1", "Round a number to N decimal places")];
    refused(
        edge::analyze_coverage(
            &round,
            &[res(
                "precison",
                "resolved",
                Some("explicit"),
                Some("AC: precision handled"),
                None,
            )],
        ),
        &["unknown resolution", "no matching proposed edge"],
    );
    refused(
        edge::analyze_coverage(
            &round,
            &[res(
                "encoding",
                "resolved",
                Some("explicit"),
                Some("AC"),
                None,
            )],
        ),
        &["unknown resolution", "no matching proposed edge"],
    );
    let rep = edge::analyze_coverage(
        &round,
        &[res(
            "precision",
            "resolved",
            Some("explicit"),
            Some("AC: precision tested"),
            None,
        )],
    )
    .expect("a report");
    assert_eq!(rep.coverage.resolved, 1);
    refused(
        edge::analyze_coverage(&[req("R1", "a"), req("R1", "b")], &[]),
        &["duplicate requirement"],
    );
    let no_id = EdgeRequirement {
        text: Some("x".into()),
        ..Default::default()
    };
    refused(
        propose_edges(&no_id),
        &["requirement id must be a non-empty string"],
    );
    refused(
        propose_edges(&req("   ", "x")),
        &["requirement id must be a non-empty string"],
    );
    let bare = EdgeRequirement {
        id: "R1".into(),
        ..Default::default()
    };
    let no_text = "text must be a non-empty string when no shapes override";
    refused(propose_edges(&bare), &[no_text]);
    refused(edge::analyze_coverage(&[bare], &[]), &[no_text]);
    refused(propose_edges(&req("R1", "")), &[no_text]);
    refused(propose_edges(&req("R1", "   ")), &[no_text]);
    assert_eq!(propose_edges(&shaped("R1", None, &[])), Ok(vec![]));
    assert!(
        !propose_edges(&shaped("R1", None, &["numeric-range"]))
            .expect("edges")
            .is_empty()
    );
}

#[test]
fn a_resolved_edge_needs_its_resolution() {
    let v = |tier: &str, resolution: Option<&str>| {
        validate_resolution(
            &res("boundary", "resolved", Some(tier), resolution, None),
            &EDGE_VALIDATORS,
        )
    };
    for tier in ["explicit", "backstop"] {
        let needle = format!("{tier} requires a resolution");
        refused(v(tier, Some("")), &[&needle]);
        refused(v(tier, Some("   ")), &[&needle]);
        refused(v(tier, None), &[&needle]);
    }
    assert_eq!(
        v("explicit", Some("AC#3: boundary tested in suite")),
        Ok(())
    );
    assert_eq!(
        v(
            "backstop",
            Some("held-out: covered by integration fuzz suite")
        ),
        Ok(())
    );
}

#[test]
fn a_second_resolution_of_a_pair_is_refused() {
    let reqs = [req("R1", INTERVALS)];
    refused(
        edge::analyze_coverage(
            &reqs,
            &[
                res(
                    "adjacency",
                    "resolved",
                    Some("explicit"),
                    Some("AC#1"),
                    None,
                ),
                res(
                    "adjacency",
                    "resolved",
                    Some("explicit"),
                    Some("AC#2"),
                    None,
                ),
            ],
        ),
        &["duplicate resolution"],
    );
    assert!(
        edge::analyze_coverage(
            &reqs,
            &[
                res(
                    "adjacency",
                    "resolved",
                    Some("explicit"),
                    Some("AC#6: touching intervals merge"),
                    None
                ),
                res(
                    "ordering",
                    "dismissed",
                    None,
                    None,
                    Some("output is canonically sorted; no tie possible")
                ),
            ],
        )
        .is_ok()
    );
}

#[test]
fn text_en_is_classified_instead_of_text() {
    let pt = "O sistema mescla intervalos sobrepostos em uma lista ordenada";
    let en = "The system merges overlapping intervals in a sorted list";
    let with_en = |text: &str, text_en: Option<&str>| EdgeRequirement {
        text_en: text_en.map(String::from),
        ..req("R1", text)
    };
    let round = "Round a number to N decimal places";
    assert_eq!(
        categories(&propose_edges(&req("R1", round)).expect("edges")),
        ["boundary", "precision"]
    );
    assert_eq!(
        categories(&propose_edges(&req("R1", pt)).expect("edges")),
        ["unclassified"]
    );
    let translated = propose_edges(&with_en(pt, Some(en))).expect("edges");
    assert_eq!(categories(&translated), ["adjacency", "empty", "ordering"]);
    assert_eq!(
        categories(&translated),
        categories(&propose_edges(&req("R1", en)).expect("edges"))
    );
    assert_eq!(
        edge::validate_requirement(&with_en("Round a number", None)),
        Ok(())
    );
    assert_eq!(
        categories(&propose_edges(&with_en(round, None)).expect("edges")),
        ["boundary", "precision"]
    );
    let blank = "text_en must be a non-empty string when present";
    refused(
        edge::validate_requirement(&with_en("Round a number", Some(""))),
        &[blank],
    );
    refused(
        edge::validate_requirement(&with_en("Round a number", Some("   "))),
        &[blank],
    );
    let r = EdgeRequirement {
        shapes: Some(vec!["numeric-range".into()]),
        ..with_en(pt, Some(en))
    };
    assert_eq!(
        categories(&propose_edges(&r).expect("edges")),
        ["boundary", "precision"]
    );
    let r = EdgeRequirement {
        shapes: Some(vec!["collection".into()]),
        ..with_en("x", Some(""))
    };
    refused(edge::validate_requirement(&r), &[blank]);
}

#[test]
fn the_shape_vocabulary_is_single_sourced() {
    let mut locked = ["numeric-range", "collection", "text", "stateful", "io"];
    locked.sort_unstable();
    let mut cues: Vec<&str> = SHAPE_CUES.iter().map(|(s, _)| s.upstream()).collect();
    cues.sort_unstable();
    let mut valid: Vec<&str> = Shape::ALL.iter().map(|s| s.upstream()).collect();
    valid.sort_unstable();
    assert_eq!(cues, locked);
    assert_eq!(valid, cues);
    assert_eq!(valid, locked);
    for shape in Shape::ALL {
        assert_eq!(Shape::from_upstream(shape.upstream()), Some(shape));
        assert_eq!(Shape::from_value(shape.value()), Some(shape));
    }
}

/// The lines of the vendored `edge-probe.cts` that start (after their indent) with `start`.
fn source_lines(start: &str) -> Vec<&'static str> {
    EDGE_PROBE_SOURCE
        .lines()
        .map(str::trim_start)
        .filter(|l| l.starts_with(start))
        .collect()
}

#[test]
fn shape_cues_match_upstream() {
    let upstream: Vec<(String, String)> = source_lines("'")
        .into_iter()
        .filter_map(|l| {
            let (name, rest) = l.strip_prefix('\'')?.split_once("': /")?;
            let body = rest.strip_suffix("/i,")?;
            Some((name.to_string(), body.to_string()))
        })
        .collect();
    let ours: Vec<(String, String)> = SHAPE_CUES
        .iter()
        .map(|(s, body)| (s.upstream().to_string(), body.to_string()))
        .collect();
    assert_eq!(ours, upstream);
}

#[test]
fn taxonomy_matches_upstream() {
    let upstream: Vec<String> = source_lines("{ id: '")
        .into_iter()
        .map(String::from)
        .collect();
    let ours: Vec<String> = TAXONOMY
        .iter()
        .map(|c| {
            let shapes: Vec<String> = c
                .shapes
                .iter()
                .map(|s| format!("'{}'", s.upstream()))
                .collect();
            format!(
                "{{ id: '{}', name: '{}', shapes: [{}], probe: '{}' }},",
                c.id,
                c.name,
                shapes.join(", "),
                c.probe
            )
        })
        .collect();
    assert_eq!(ours, upstream);
    let ids: Vec<&str> = TAXONOMY
        .iter()
        .map(|c| c.id)
        .chain([UNCLASSIFIED])
        .collect();
    assert_eq!(EDGE_VALIDATORS.categories, ids.as_slice());
}

/// The six worked examples of `references/edge-probe.md`: each fixture's input, as upstream's
/// `references/edge-probe-fixtures/<name>/` holds it.
fn reference_inputs() -> Vec<(&'static str, &'static str, Vec<Resolution>)> {
    let intervals = "Merge a list of overlapping intervals into the minimal set";
    vec![
        (
            "01-round-half-even",
            "Round a number to N decimal places, rounding half to nearest",
            vec![],
        ),
        ("02-merge-intervals", intervals, vec![]),
        (
            "03-truncate-graphemes",
            "Truncate a display string to its first N characters",
            vec![],
        ),
        (
            "04-money-rounding",
            "Compute the total price as an amount rounded to two decimals",
            vec![],
        ),
        (
            "05-list-dedupe",
            "Return all items in the list with duplicates removed",
            vec![],
        ),
        (
            "06-resolved-mixed",
            intervals,
            vec![
                res(
                    "adjacency",
                    "resolved",
                    Some("explicit"),
                    Some("AC#6: touching intervals merge"),
                    None,
                ),
                res(
                    "ordering",
                    "dismissed",
                    None,
                    None,
                    Some("output is canonically sorted; no tie possible"),
                ),
            ],
        ),
    ]
}

#[test]
fn the_reference_examples_are_the_engines_answers() {
    let lines: Vec<&str> = REFERENCE.lines().collect();
    let fences = gsd_kit::markdown::fences(REFERENCE);
    for (name, text, resolutions) in reference_inputs() {
        let mark = format!("edge-probe:{name}/expected-coverage.json");
        let blocks: Vec<&gsd_kit::markdown::Fence> = fences
            .iter()
            .filter(|f| f.language == "json" && lines[f.open].contains(&mark))
            .collect();
        assert_eq!(blocks.len(), 1, "{name}: {} expected blocks", blocks.len());
        let expected: Value = serde_json::from_str(&blocks[0].text).expect("the block is JSON");
        let report = edge::analyze_coverage(&[req("R1", text)], &resolutions).expect("a report");
        assert_eq!(report.to_json(), expected, "{name}");
    }
}
