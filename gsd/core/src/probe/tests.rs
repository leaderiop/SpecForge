//! Ported from gsd-core v1.19.0 tests/probe-core.test.cjs and tests/edge-probe.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! One test per upstream describe block, each upstream case a row of it; edge-probe's blocks are `edge/tests.rs`, on
//! the helpers here. `UPSTREAM_CASES` names each ported case of both suites by
//! its title, and `every_ported_case_is_in_the_vendored_suites` finds each title in the vendored suites. The golden
//! fixtures (edge-probe.test.cjs's last block) are `the_reference_examples_are_the_engines_answers`: the six inputs are
//! constants here, and each answer is the expected block `references/edge-probe.md` keeps.
//!
//! Left out, and where each lives: edge-probe's `CLI (built artifact)` and `CLI JSON.parse error handling` blocks, and
//! `CLI exits 2 (handled) on an invalid authored shape`, are the command's (`tests/cli/cmd_edge_probe.rs`); probe-core's
//! `runProbeCli` block is the command itself; `projectProhibitions` (backward-compat and descriptor projection) is the
//! planner's lift into `prohibition`; the truth normalizers, `dispositionForUnverifiableTruth` and `projectTruths` are
//! `gsd/E501` and `gsd/E502`. The cases that hand the engine a value of another JSON type (`text: 42`, `shapes:
//! 'numeric-range'`, `shapes: [42]`, `text_en: 42`, a requirements or items list that is not an array) have no Rust
//! twin: the types refuse them.

use serde_json::{Value, json};

use super::edge::EdgeRequirement;
use super::*;

const UPSTREAM_CASES: &[&str] = &[
    // probe-core.test.cjs
    "exposes exactly resolved | dismissed | unresolved (no covered/backstop)",
    "rejects an unknown status",
    "rejects a former covered status (re-cut: no longer a valid status)",
    "rejects dismissed without a reason",
    "accepts dismissed with a reason",
    "rejects resolved with a missing verification tier",
    "rejects resolved with an unknown verification tier",
    "rejects resolved/explicit with empty resolution text",
    "rejects resolved/backstop with missing resolution note",
    "accepts resolved/explicit with a resolution",
    "accepts resolved/backstop with a resolution note",
    "rejects a dismissed resolution carrying a verification tier (null unless resolved)",
    "rejects an unresolved resolution carrying a verification tier",
    "rejects an unresolved resolution carrying a resolution payload",
    "rejects an unresolved resolution carrying a reason payload",
    "accepts a bare unresolved resolution (no payload — a harmless no-op merge)",
    "rejects a missing id",
    "rejects an empty id",
    "accepts a valid requirement",
    "no resolutions → every item unresolved; resolved 0; byVerification zeroed per tier",
    "merges a resolved/explicit resolution and counts byVerification.explicit",
    "a dismissed item counts toward coverage.resolved (closed set) but NOT byVerification",
    "mixed resolved/explicit + backstop + dismissed: resolved = closed = applicable - unresolved",
    "an all-dismissed run is CLOSED but NOT affirmatively covered (byVerification is the honest gate)",
    "rejects a duplicate (requirement_id, category) resolution",
    "rejects an orphan resolution (no matching proposed item)",
    "propagates an invalid-resolution throw from validateResolution",
    "rejects a proposed item whose category is not in validators.categories",
    "m1: rejects a verbatim item carrying an out-of-enum status (e.g. the dropped \"covered\")",
    "m1: rejects a verbatim item dismissed without a reason",
    "m1: rejects a verbatim resolved item missing its verification tier",
    "m1: a matching resolution still governs — item validation targets VERBATIM items only",
    "the issue defect case: all requirements unclassified reports unclassified === applicable",
    "a classified spec reports unclassified 0 (control 1 — text_en populated)",
    "an edge-free requirement (no items) reports applicable 0 and unclassified 0 (control 2)",
    "mixed spec: unclassified is a strict subset of applicable; the counting identity is preserved",
    // edge-probe.test.cjs
    "detects numeric-range from rounding/threshold cues",
    "detects collection from interval/merge cues",
    "detects text from truncate/string cues",
    "returns [] when no cue matches",
    "TAXONOMY has the 8 documented categories in order",
    "every category has name, shapes[], probe",
    "numeric-range raises boundary + precision only",
    "collection raises adjacency, empty, ordering",
    "text raises empty + encoding",
    "no shapes raises nothing",
    "rounding requirement proposes boundary + precision, all unresolved (verification null)",
    "authored shapes override prose classification",
    "rejects a former covered status (re-cut: covered is no longer a status)",
    "rejects resolved with a verification tier outside {explicit, backstop}",
    "with no resolutions, every applicable edge is unresolved (byVerification zeroed)",
    "merges a resolved/explicit resolution and counts it resolved",
    "throws if a resolution is invalid (dismissed w/o reason)",
    "shapes: [] returns zero edges (explicit empty-shapes override)",
    "absent shapes key classifies from prose (no override)",
    "shapes: [collection] overrides prose and proposes collection categories",
    "prose with no shape cue surfaces a single unclassified candidate",
    "UNCLASSIFIED_CATEGORY is a valid item category but NOT a taxonomy category",
    "explicit shapes: [] stays silent (deliberate opt-out — NOT unclassified)",
    "prose that DOES classify proposes real edges, never an unclassified candidate",
    "analyzeCoverage surfaces the unclassified candidate as unresolved (no throw)",
    "an unclassified candidate can be dismissed with a reason (edge-probe parity)",
    "rejects an unknown shape value (typo for a locked shape)",
    "rejects a mixed array where one entry is invalid",
    "analyzeCoverage propagates the invalid-shape throw",
    "a valid locked shape still proposes its categories (no false rejection)",
    "shapes: [] remains a valid zero-edge override (RR-06 intact)",
    "rejects an orphan resolution (typo category — no matching proposed edge)",
    "rejects a resolution for a valid-but-non-applicable category",
    "a matching resolution still resolves (no false orphan rejection)",
    "rejects a duplicate requirement id",
    "rejects a missing requirement id",
    "rejects an empty requirement id",
    "rejects a missing requirement text when no shapes override (M2 fail-open)",
    "rejects an empty/whitespace requirement text when no shapes override (M2)",
    "allows missing/empty text WHEN an explicit shapes override is provided (M2 legitimate path)",
    "rejects resolved/explicit with empty resolution string",
    "rejects resolved/explicit with whitespace-only resolution",
    "rejects resolved/explicit with missing resolution",
    "accepts resolved/explicit with a non-empty resolution",
    "rejects resolved/backstop with empty resolution string",
    "rejects resolved/backstop with whitespace-only resolution",
    "rejects resolved/backstop with missing resolution",
    "accepts resolved/backstop with a non-empty resolution note",
    "rejects duplicate (requirement_id, category) resolution",
    "distinct pairs still analyze without throwing",
    "proposeEdges: text_en absent falls back to text (back-compat)",
    "proposeEdges: text_en present is used for classification instead of text",
    "#3717: a non-English requirement with text_en classifies identically to its English equivalent",
    "validateRequirement: text_en: null is treated as absent (no throw)",
    "proposeEdges: text_en: null falls back to text",
    r"validateRequirement: rejects empty-string text_en (?? does not catch \'\')",
    "validateRequirement: rejects whitespace-only text_en",
    "proposeEdges: authored shapes override still wins when text_en is also present",
    "validateRequirement: rejects empty text_en even when shapes override makes it unused",
    "SHAPE_CUES keys match the locked 5-shape vocabulary exactly",
    "VALID_SHAPES matches SHAPE_CUES keys exactly (single source of truth)",
    "VALID_SHAPES matches the locked 5-shape vocabulary exactly",
    "fixture ${name} matches its golden coverage",
];

const PROBE_CORE_SUITE: &str = include_str!(gsd_kit::snapshot!("tests/probe-core.test.cjs"));
const EDGE_PROBE_SUITE: &str = include_str!(gsd_kit::snapshot!("tests/edge-probe.test.cjs"));

#[test]
fn every_ported_case_is_in_the_vendored_suites() {
    for title in UPSTREAM_CASES {
        assert!(
            PROBE_CORE_SUITE.contains(title) || EDGE_PROBE_SUITE.contains(title),
            "the vendored suites have no case '{title}'"
        );
    }
}

/// `result` is refused with a message holding one of `needles`, ignoring case (upstream's `/a|b/i`).
pub(super) fn refused<T: std::fmt::Debug>(result: Result<T, String>, needles: &[&str]) {
    let why = result.expect_err("refused").to_lowercase();
    assert!(
        needles.iter().any(|n| why.contains(&n.to_lowercase())),
        "'{why}' holds none of {needles:?}"
    );
}

/// The validators probe-core's suite exercises the engine with.
pub(super) const VALIDATORS: Validators = Validators {
    categories: &["adjacency", "empty", "ordering"],
    verification: &["explicit", "backstop"],
    required: &[("explicit", &["resolution"]), ("backstop", &["resolution"])],
};

/// A resolution of `R1`: `(category, status, verification, resolution, reason)`.
pub(super) fn res(
    category: &str,
    status: &str,
    verification: Option<&str>,
    resolution: Option<&str>,
    reason: Option<&str>,
) -> Resolution {
    Resolution {
        requirement_id: "R1".into(),
        category: category.into(),
        status: status.into(),
        verification: verification.map(String::from),
        resolution: resolution.map(String::from),
        reason: reason.map(String::from),
    }
}

/// probe-core's `item(category)`: an unresolved item of `R1`.
pub(super) fn item(category: &str) -> Item {
    Item {
        requirement_id: "R1".into(),
        category: category.into(),
        status: "unresolved".into(),
        verification: None,
        resolution: None,
        reason: None,
        probe: format!("probe-for-{category}"),
    }
}

pub(super) fn unresolved_items() -> Vec<Item> {
    vec![item("adjacency"), item("empty"), item("ordering")]
}

pub(super) fn req(id: &str, text: &str) -> EdgeRequirement {
    EdgeRequirement {
        id: id.into(),
        text: Some(text.into()),
        ..Default::default()
    }
}

pub(super) fn shaped(id: &str, text: Option<&str>, shapes: &[&str]) -> EdgeRequirement {
    EdgeRequirement {
        id: id.into(),
        text: text.map(String::from),
        shapes: Some(shapes.iter().map(|s| s.to_string()).collect()),
        ..Default::default()
    }
}

pub(super) fn categories(items: &[Item]) -> Vec<&str> {
    let mut out: Vec<&str> = items.iter().map(|i| i.category.as_str()).collect();
    out.sort_unstable();
    out
}

pub(super) fn coverage(report: &CoverageReport) -> Value {
    report.coverage.to_json()
}

#[test]
fn valid_status_is_the_lifecycle_enum() {
    let mut sorted = VALID_STATUS;
    sorted.sort_unstable();
    assert_eq!(sorted, ["dismissed", "resolved", "unresolved"]);
    assert!(!VALID_STATUS.contains(&"covered") && !VALID_STATUS.contains(&"backstop"));
}

#[test]
fn validate_resolution_judges_status_and_verification() {
    let v = |r: Resolution| validate_resolution(&r, &VALIDATORS);
    refused(
        v(res("adjacency", "maybe", None, None, None)),
        &["invalid status"],
    );
    refused(
        v(res("adjacency", "covered", None, Some("x"), None)),
        &["invalid status"],
    );
    refused(
        v(res("adjacency", "dismissed", None, None, Some(""))),
        &["dismissed requires a reason"],
    );
    assert_eq!(
        v(res(
            "adjacency",
            "dismissed",
            None,
            None,
            Some("bounded enum")
        )),
        Ok(())
    );
    refused(
        v(res("adjacency", "resolved", None, Some("AC"), None)),
        &["verification"],
    );
    refused(
        v(res(
            "adjacency",
            "resolved",
            Some("judgment"),
            Some("AC"),
            None,
        )),
        &["invalid verification"],
    );
    refused(
        v(res(
            "adjacency",
            "resolved",
            Some("explicit"),
            Some("   "),
            None,
        )),
        &["explicit requires a resolution"],
    );
    refused(
        v(res("adjacency", "resolved", Some("backstop"), None, None)),
        &["backstop requires a resolution"],
    );
    assert_eq!(
        v(res(
            "adjacency",
            "resolved",
            Some("explicit"),
            Some("AC#6"),
            None
        )),
        Ok(())
    );
    assert_eq!(
        v(res(
            "adjacency",
            "resolved",
            Some("backstop"),
            Some("held-out PBT suite"),
            None
        )),
        Ok(())
    );
    refused(
        v(res(
            "adjacency",
            "dismissed",
            Some("explicit"),
            None,
            Some("n/a"),
        )),
        &["verification must be null"],
    );
    refused(
        v(res("adjacency", "unresolved", Some("backstop"), None, None)),
        &["verification must be null"],
    );
    refused(
        v(res("adjacency", "unresolved", None, Some("AC#1"), None)),
        &["unresolved must not carry"],
    );
    refused(
        v(res("adjacency", "unresolved", None, None, Some("because"))),
        &["unresolved must not carry"],
    );
    assert_eq!(v(res("adjacency", "unresolved", None, None, None)), Ok(()));
}

#[test]
fn validate_requirement_checks_the_generic_id() {
    refused(
        validate_requirement("", Some("x")),
        &["requirement id must be a non-empty string"],
    );
    refused(
        validate_requirement("   ", Some("x")),
        &["requirement id must be a non-empty string"],
    );
    assert_eq!(
        validate_requirement("R1", Some("a testable statement")),
        Ok(())
    );
}

#[test]
fn analyze_coverage_merges_and_rolls_up() {
    let rep = analyze_coverage(unresolved_items(), &[], &VALIDATORS).expect("a report");
    assert_eq!(
        coverage(&rep),
        json!({"applicable": 3, "resolved": 0, "unresolved": 3, "unclassified": 0,
               "byVerification": {"explicit": 0, "backstop": 0}})
    );

    let touching = "AC#6: touching intervals merge";
    let rep = analyze_coverage(
        unresolved_items(),
        &[res(
            "adjacency",
            "resolved",
            Some("explicit"),
            Some(touching),
            None,
        )],
        &VALIDATORS,
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

    let rep = analyze_coverage(
        unresolved_items(),
        &[res(
            "ordering",
            "dismissed",
            None,
            None,
            Some("canonically sorted; no tie"),
        )],
        &VALIDATORS,
    )
    .expect("a report");
    assert_eq!((rep.coverage.resolved, rep.coverage.unresolved), (1, 2));
    assert_eq!(
        coverage(&rep)["byVerification"],
        json!({"explicit": 0, "backstop": 0})
    );
    let ord = rep
        .items
        .iter()
        .find(|i| i.category == "ordering")
        .expect("ordering");
    assert_eq!(
        (
            ord.status.as_str(),
            ord.verification.as_deref(),
            ord.reason.as_deref()
        ),
        ("dismissed", None, Some("canonically sorted; no tie"))
    );

    let rep = analyze_coverage(
        unresolved_items(),
        &[
            res(
                "adjacency",
                "resolved",
                Some("explicit"),
                Some("AC#6"),
                None,
            ),
            res(
                "empty",
                "resolved",
                Some("backstop"),
                Some("held-out empty-input PBT"),
                None,
            ),
            res(
                "ordering",
                "dismissed",
                None,
                None,
                Some("canonically sorted"),
            ),
        ],
        &VALIDATORS,
    )
    .expect("a report");
    assert_eq!(
        (
            rep.coverage.applicable,
            rep.coverage.unresolved,
            rep.coverage.resolved
        ),
        (3, 0, 3)
    );
    assert_eq!(
        coverage(&rep)["byVerification"],
        json!({"explicit": 1, "backstop": 1})
    );

    let rep = analyze_coverage(
        unresolved_items(),
        &[
            res("adjacency", "dismissed", None, None, Some("bounded enum")),
            res(
                "empty",
                "dismissed",
                None,
                None,
                Some("guaranteed non-empty"),
            ),
            res(
                "ordering",
                "dismissed",
                None,
                None,
                Some("canonically sorted"),
            ),
        ],
        &VALIDATORS,
    )
    .expect("a report");
    assert_eq!((rep.coverage.unresolved, rep.coverage.resolved), (0, 3));
    assert_eq!(
        coverage(&rep)["byVerification"],
        json!({"explicit": 0, "backstop": 0})
    );

    refused(
        analyze_coverage(
            unresolved_items(),
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
            &VALIDATORS,
        ),
        &["duplicate resolution"],
    );
    refused(
        analyze_coverage(
            unresolved_items(),
            &[res(
                "boundary",
                "resolved",
                Some("explicit"),
                Some("AC"),
                None,
            )],
            &VALIDATORS,
        ),
        &["unknown resolution", "no matching proposed"],
    );
    refused(
        analyze_coverage(
            unresolved_items(),
            &[res("empty", "dismissed", None, None, None)],
            &VALIDATORS,
        ),
        &["dismissed requires a reason"],
    );
    refused(
        analyze_coverage(vec![item("bogus-category")], &[], &VALIDATORS),
        &["unknown category"],
    );

    let verbatim = |status: &str, resolution: Option<&str>| Item {
        status: status.into(),
        resolution: resolution.map(String::from),
        ..item("adjacency")
    };
    refused(
        analyze_coverage(vec![verbatim("covered", None)], &[], &VALIDATORS),
        &["invalid status"],
    );
    refused(
        analyze_coverage(vec![verbatim("dismissed", None)], &[], &VALIDATORS),
        &["dismissed requires a reason"],
    );
    refused(
        analyze_coverage(vec![verbatim("resolved", Some("AC"))], &[], &VALIDATORS),
        &["requires a verification tier"],
    );
    let rep = analyze_coverage(
        vec![verbatim("covered", None)],
        &[res(
            "adjacency",
            "resolved",
            Some("explicit"),
            Some("AC#6"),
            None,
        )],
        &VALIDATORS,
    )
    .expect("a report");
    assert_eq!(
        (
            rep.items[0].status.as_str(),
            rep.items[0].verification.as_deref()
        ),
        ("resolved", Some("explicit"))
    );
    assert_eq!(rep.coverage.resolved, 1);
}

#[test]
fn unclassified_is_a_sibling_count() {
    const V: Validators = Validators {
        categories: &["idempotency", "ordering", "unclassified"],
        verification: &["explicit", "backstop"],
        required: &[("explicit", &["resolution"]), ("backstop", &["resolution"])],
    };
    let of = |req: &str, category: &str| Item {
        requirement_id: req.into(),
        ..item(category)
    };
    let rep = analyze_coverage(
        vec![
            of("CONN-01", "unclassified"),
            of("CONN-02", "unclassified"),
            of("PJL-01", "unclassified"),
        ],
        &[],
        &V,
    )
    .expect("a report");
    let c = &rep.coverage;
    assert_eq!(
        (c.applicable, c.unclassified, c.resolved, c.unresolved),
        (3, 3, 0, 3)
    );

    let rep = analyze_coverage(
        vec![of("CONN-01", "idempotency"), of("CONN-02", "ordering")],
        &[],
        &V,
    )
    .expect("a report");
    assert_eq!((rep.coverage.applicable, rep.coverage.unclassified), (2, 0));

    let rep = analyze_coverage(vec![], &[], &V).expect("a report");
    assert_eq!((rep.coverage.applicable, rep.coverage.unclassified), (0, 0));

    let rep = analyze_coverage(
        vec![of("CONN-01", "idempotency"), of("CONN-02", "unclassified")],
        &[],
        &V,
    )
    .expect("a report");
    let c = &rep.coverage;
    assert_eq!((c.applicable, c.unclassified), (2, 1));
    assert_eq!(c.resolved, c.applicable - c.unresolved);
}
