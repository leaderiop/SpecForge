//! `specforge gsd edge-probe`: the edge coverage of a phase (the taskboard twin's requirements) or of a phase spec (the
//! discuss project's locked requirements and its recorded edges), proposed and rolled up by the probe engine.

use super::commands_support::*;
use super::harness::{Project, project, taskboard};

const SPEC: &str = "v1.0/02/02-SPEC";

/// `edge-probe <subject>` under `--format json`, which must answer.
fn probe(p: &Project, subject: &str) -> serde_json::Value {
    let run = gsd(p, "edge-probe", &[subject, "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    json(&run)
}

/// `(requirement, category)` of each item, in the answer's order.
fn pairs(report: &serde_json::Value) -> Vec<(String, String)> {
    report["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            let text = |k: &str| i[k].as_str().unwrap_or_default().to_string();
            (text("requirement_id"), text("category"))
        })
        .collect()
}

fn pair(requirement: &str, category: &str) -> (String, String) {
    (requirement.to_string(), category.to_string())
}

/// The discuss project with a second locked requirement, whose prose names `every` and `each` (a collection).
fn with_lr2() -> Project {
    let p = project("discuss");
    let run = p.run(&[
        "new",
        "locked_req",
        "LR2",
        "Tasks are saved after every change",
        "--in",
        "spec/discuss.spec",
        "--spec",
        SPEC,
        "--title",
        "Tasks are saved after every change",
        "--current",
        "Tasks live in memory",
        "--target",
        "Tasks are saved to disk after every change",
        "--acceptance",
        "Each change is on disk before the next one starts",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    p
}

#[test]
fn a_phase_reports_its_requirements_edges() {
    let p = taskboard();
    let report = probe(&p, "v1.0/02");
    assert_eq!(report["kind"], "phase");
    // `saved` is not `save`: the stateful cue's `\b` holds, so STORE-01 is a collection only (`every`)
    assert_eq!(
        pairs(&report),
        [
            pair("STORE-01", "adjacency"),
            pair("STORE-01", "empty"),
            pair("STORE-01", "ordering"),
            pair("STORE-02", "unclassified"),
        ]
    );
    assert_eq!(
        report["coverage"],
        serde_json::json!({"applicable": 4, "resolved": 0, "unresolved": 4, "unclassified": 1,
            "byVerification": {"explicit": 0, "backstop": 0}})
    );
    assert!(
        report["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|i| i["edge"].is_null() && i["proposed"] == true)
    );
    assert_eq!(pairs(&probe(&p, "v1.0/03")).len(), 6);
    let first = pairs(&probe(&p, "v1.0/01"));
    assert_eq!(first.len(), 2, "{first:?}");
    assert!(first.iter().all(|(_, c)| c == "unclassified"), "{first:?}");
    let human = gsd(&p, "edge-probe", &["v1.0/02"]);
    assert!(
        human
            .stdout
            .contains("Coverage: 0/4 resolved · 4 unresolved"),
        "{}",
        human.all()
    );
}

#[test]
fn a_spec_merges_its_recorded_edges() {
    let p = with_lr2();
    for args in [
        &[
            "new",
            "edge",
            "EC1",
            "LR2 adjacency",
            "--in",
            "spec/discuss.spec",
            "--spec",
            SPEC,
            "--requirement",
            "LR2",
            "--category",
            "adjacency",
            "--status",
            "resolved",
            "--verification",
            "explicit",
            "--resolution",
            "Two changes in one tick are both saved",
        ][..],
        &[
            "new",
            "edge",
            "EC2",
            "LR1 precision",
            "--in",
            "spec/discuss.spec",
            "--spec",
            SPEC,
            "--requirement",
            "LR1",
            "--category",
            "precision",
            "--status",
            "unresolved",
        ][..],
    ] {
        let run = p.run(args);
        assert_eq!(run.code, 0, "{}", run.all());
    }
    let report = probe(&p, SPEC);
    assert_eq!(report["kind"], "phase_spec");
    let items = report["items"].as_array().expect("items");
    assert_eq!(items.len(), 5, "{report:#}");
    let lr1 = format!("{SPEC}/LR1");
    let lr2 = format!("{SPEC}/LR2");
    let raised = items
        .iter()
        .find(|i| i["category"] == "precision")
        .expect("the raised row");
    assert_eq!(
        (
            &raised["requirement_id"],
            &raised["proposed"],
            &raised["edge"],
            &raised["status"]
        ),
        (
            &lr1.clone().into(),
            &false.into(),
            &format!("{SPEC}/EC2").into(),
            &"unresolved".into()
        )
    );
    let resolved = items
        .iter()
        .find(|i| i["requirement_id"] == lr2.as_str() && i["category"] == "adjacency")
        .expect("LR2 adjacency");
    assert_eq!(
        (
            &resolved["status"],
            &resolved["verification"],
            &resolved["edge"],
            &resolved["proposed"]
        ),
        (
            &"resolved".into(),
            &"explicit".into(),
            &format!("{SPEC}/EC1").into(),
            &true.into()
        )
    );
    let c = &report["coverage"];
    assert_eq!(
        (
            &c["applicable"],
            &c["resolved"],
            &c["byVerification"]["explicit"]
        ),
        (&5.into(), &1.into(), &1.into())
    );
}

#[test]
fn a_written_shape_overrides_the_classifier() {
    let p = project("discuss");
    let run = p.run(&["set", &format!("{SPEC}/LR1"), "shapes", "text"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let lr1 = format!("{SPEC}/LR1");
    assert_eq!(
        pairs(&probe(&p, SPEC)),
        [pair(&lr1, "empty"), pair(&lr1, "encoding")]
    );
}

#[test]
fn text_en_is_classified_first() {
    let p = project("discuss");
    let lr1 = format!("{SPEC}/LR1");
    assert_eq!(pairs(&probe(&p, SPEC)), [pair(&lr1, "unclassified")]);
    let run = p.run(&["set", &lr1, "text_en", "Each item is kept"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        pairs(&probe(&p, SPEC)),
        [
            pair(&lr1, "adjacency"),
            pair(&lr1, "empty"),
            pair(&lr1, "ordering")
        ]
    );
}

#[test]
fn the_probe_refuses_what_upstream_refuses() {
    let p = taskboard();
    for (subject, needle) in [
        ("NOPE", "no phase spec or phase 'NOPE'"),
        ("v1.0/02.1", "has no requirements: nothing to probe"),
    ] {
        let run = gsd(&p, "edge-probe", &[subject]);
        assert_eq!(run.code, 1, "{subject}: {}", run.all());
        assert!(run.all().contains(needle), "{subject}: {}", run.all());
    }
    let p = project("discuss");
    let lr1 = format!("{SPEC}/LR1");
    let run = p.run(&["set", &lr1, "text_en", " "]);
    assert_eq!(run.code, 0, "{}", run.all());
    let run = gsd(&p, "edge-probe", &[SPEC]);
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(
        run.all().contains(&format!(
            "requirement {lr1} text_en must be a non-empty string when present"
        )),
        "{}",
        run.all()
    );
}

/// `resolve-edge` under `--format json`: the envelope's `output`.
fn resolve(p: &Project, args: &[&str]) -> serde_json::Value {
    let mut all = args.to_vec();
    all.extend(["--format", "json"]);
    let run = gsd(p, "resolve-edge", &all);
    assert_eq!(run.code, 0, "{}", run.all());
    json(&run)["output"].clone()
}

#[test]
fn a_resolution_moves_a_row_in_one_edit() {
    let p = project("discuss");
    let lr1 = format!("{SPEC}/LR1");
    let ec1 = format!("{SPEC}/EC1");
    let first = resolve(&p, &[&lr1, "unclassified", "unresolved"]);
    assert_eq!(
        (&first["created"], &first["wrote"][0]),
        (&true.into(), &ec1.clone().into())
    );
    let resolved = resolve(
        &p,
        &[
            &lr1,
            "unclassified",
            "resolved",
            "--verification",
            "explicit",
            "--resolution",
            "AC#1 restarts it",
        ],
    );
    assert_eq!(
        (&resolved["created"], &resolved["edge"]["status"]),
        (&false.into(), &"resolved".into())
    );
    let report = probe(&p, SPEC);
    assert_eq!(report["coverage"]["resolved"], 1, "{report:#}");
    resolve(
        &p,
        &[
            &lr1,
            "unclassified",
            "dismissed",
            "--reason",
            "a restart is the test",
        ],
    );
    let fields = p.fields_of(&ec1);
    assert_eq!(fields["status"], "dismissed", "{fields:#}");
    assert!(
        fields.get("verification").is_none() && fields.get("resolution").is_none(),
        "{fields:#}"
    );
    p.check_exact(&[]);
}

#[test]
fn a_bad_resolution_writes_nothing() {
    let p = project("discuss");
    let lr1 = format!("{SPEC}/LR1");
    let before = bytes(&p);
    for (args, needle) in [
        (
            vec![lr1.as_str(), "boundary", "resolved", "--resolution", "x"],
            "resolved requires a verification tier",
        ),
        (
            vec![lr1.as_str(), "boundary", "dismissed"],
            "dismissed requires a reason",
        ),
        (
            vec![lr1.as_str(), "boundary", "unresolved", "--reason", "why"],
            "unresolved must not carry a reason",
        ),
        (
            vec![lr1.as_str(), "corner", "unresolved"],
            "unknown category \"corner\"",
        ),
    ] {
        let run = gsd(&p, "resolve-edge", &args);
        assert_eq!(run.code, 1, "{args:?}: {}", run.all());
        assert!(run.all().contains(needle), "{args:?}: {}", run.all());
        unchanged(&p, &before);
    }
}

#[test]
fn a_prohibition_leaves_its_check_when_it_stops_being_tested() {
    let p = project("discuss");
    let run = p.run(&[
        "new",
        "spec_prohibition",
        "SP1",
        "No plaintext",
        "--in",
        "spec/discuss.spec",
        "--spec",
        SPEC,
        "--requirement",
        "LR1",
        "--statement",
        "MUST NOT store tasks in plaintext",
        "--status",
        "resolved",
        "--verification",
        "test",
        "--check_kind",
        "node_test",
        "--check_target",
        "tests/no-plaintext.test.cjs",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let sp1 = format!("{SPEC}/SP1");
    let run = gsd(
        &p,
        "resolve-prohibition",
        &[&sp1, "dismissed", "--reason", "no secrets in a task"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of(&sp1);
    assert_eq!(fields["status"], "dismissed", "{fields:#}");
    for gone in ["verification", "check_kind", "check_target"] {
        assert!(fields.get(gone).is_none(), "{gone}: {fields:#}");
    }
    let run = gsd(
        &p,
        "resolve-prohibition",
        &[&sp1, "resolved", "--verification", "judgment"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(&sp1)["verification"], "judgment");
    p.check_exact(&[]);
}

/// Why the commands exist: a resolution is three fields, and gsd/E523 refuses each edit that writes one of them alone.
#[test]
fn two_set_calls_cannot_resolve_an_edge() {
    let p = project("discuss");
    let lr1 = format!("{SPEC}/LR1");
    resolve(&p, &[&lr1, "unclassified", "unresolved"]);
    let before = bytes(&p);
    let run = p.run(&["set", &format!("{SPEC}/EC1"), "status", "resolved"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains("E080") && run.all().contains("gsd/E523"),
        "{}",
        run.all()
    );
    unchanged(&p, &before);
}
