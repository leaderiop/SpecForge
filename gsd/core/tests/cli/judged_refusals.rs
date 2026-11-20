//! What the host's E080 lets through (ADR 0297): a judged rule's finding is never an error a
//! write adds, so a failing verdict replaces a pass, a judged target is created with its verdict owed, and an edit that
//! stales a passing verdict is written. The finding is reported, and every gate still fails on it.

use super::harness::{Project, Run, project};
use super::judged::{PLAN, record_all, state, status_of};
use super::verify_verdicts::{bare, verification, verified};
use serde_json::json;
use specforge_extension_sdk::prelude::TestStatus;

const ITEMS: [&str; 6] = [
    "v1.0/01/01-01/TR1",
    "v1.0/01/01-01/AR1",
    "v1.0/01/01-01/KL1",
    "v1.0/01/01-02/TR1",
    "v1.0/01/01-02/AR1",
    "v1.0/01/01-02/KL1",
];

/// Phase 01 verified: six `verified` verdicts, status `passed`.
fn passed() -> Project {
    let p = bare();
    for item in ITEMS {
        verified(&p, item);
    }
    assert_eq!(verification(&p)["status"], "passed");
    p
}

#[test]
fn a_failing_verdict_replaces_a_pass() {
    let p = passed();
    let truth = "v1.0/01/01-01/TR1";
    let run = p.record(truth, "E501", "failed", &["--cited", &p.cites(truth)]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(verification(&p)["status"], "gaps_found");
    assert_eq!(
        state(&p, "v1.0/01/01-01/TR1", "E501"),
        ("fresh".to_string(), false)
    );
}

/// A verdict on a truth reads only what it cites (ADR 0377), so one that cites no file of the project and no test could
/// never go stale on a code edit: refused, whatever its outcome, before the host reads its basis.
#[test]
fn a_truth_verdict_that_cites_no_code_is_refused() {
    let p = bare();
    let truth = "v1.0/01/01-01/TR1";
    let basis = p.inputs(truth, "E501")["basis"].to_string();
    let spec = p.read("spec/plans.spec");
    let uncited: [(&str, &[&str]); 6] = [
        ("verified", &[]),
        ("verified", &["--cited", "[]"]),
        ("verified", &["--cited", r#"[{"file":"../outside.js"}]"#]),
        ("verified", &["--cited", r#"[{"file":"/etc/hosts"}]"#]),
        ("verified", &["--cited", r#"[{"entity":"v1.0/01"}]"#]),
        ("failed", &[]),
    ];
    for (outcome, extra) in uncited {
        let run = p.record_over(truth, "E501", outcome, &basis, extra);
        assert_ne!(run.code, 0, "{extra:?}: {}", run.all());
        assert!(
            run.all()
                .contains("a verdict on a truth must cite a file of the project or a test"),
            "{extra:?}: {}",
            run.all()
        );
    }
    assert_eq!(p.read("spec/plans.spec"), spec, "nothing is written");
}

/// The command reads the entity's spelling as the host does (ADR 0284, ADR 0315): a rooted path or a zero-padded number
/// names the truth too, so the uncited verdict is refused under every spelling, never recorded because the command could
/// not tell it was a truth. A cited one is recorded on the truth's path, whatever the spelling.
#[test]
fn every_spelling_of_a_truth_is_held_to_its_citation() {
    let p = bare();
    let truth = "v1.0/01/01-01/TR1";
    let basis = p.inputs(truth, "E501")["basis"].to_string();
    let spec = p.read("spec/plans.spec");
    let spellings = [
        "/v1.0/01/01-01/TR1",
        "v1.0/1/01-01/TR1",
        "/v1.0/001/01-01/TR1",
        "1/01-01/TR1",
    ];
    for spelling in spellings {
        let run = p.record_over(spelling, "E501", "verified", &basis, &[]);
        assert_ne!(run.code, 0, "{spelling}: {}", run.all());
        assert!(
            run.all()
                .contains("a verdict on a truth must cite a file of the project or a test"),
            "{spelling}: {}",
            run.all()
        );
    }
    assert_eq!(p.read("spec/plans.spec"), spec, "nothing is written");
    let run = p.record(
        "/v1.0/1/01-01/TR1",
        "E501",
        "verified",
        &["--cited", r#"[{"file":"src/tasks.js"}]"#],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        run.stdout
            .contains("verdict gsd/E501 verified on v1.0/01/01-01/TR1"),
        "{}",
        run.all()
    );
    assert_eq!(state(&p, truth, "E501"), ("fresh".to_string(), true));
}

/// A spelling that names no entity, several, or one the rule does not judge is the command's own refusal.
#[test]
fn a_verdict_on_an_entity_the_command_cannot_resolve_is_refused() {
    let p = bare();
    for (spelling, why) in [
        (
            "/01-01/TR1",
            "entity: '/01-01/TR1' names no entity of the project",
        ),
        ("TR1", "entity: 'TR1' is ambiguous: "),
        (
            "v1.0/01/01-01",
            "entity: 'v1.0/01/01-01' is a plan, which gsd/E501 does not judge (it judges: truth, artifact, key_link)",
        ),
    ] {
        let run = p.record_over(spelling, "E501", "verified", "{}", &[]);
        assert_ne!(run.code, 0, "{spelling}: {}", run.all());
        assert!(run.all().contains(why), "{spelling}: {}", run.all());
    }
}

/// A truth's verdict citing a file of the project, or only a collected test fact, records; an artifact's needs no
/// citation (it reads the file it names).
#[test]
fn a_truth_verdict_citing_a_file_or_a_test_records() {
    let truth = "v1.0/01/01-01/TR1";
    for (cited, collect) in [
        (r#"[{"file":"src/tasks.js"}]"#, false),
        (r#"[{"test":"test/tasks.test.js#a task is added"}]"#, true),
    ] {
        let p = bare();
        if collect {
            let run =
                p.node_tests(&[("test/tasks.test.js", "a task is added", TestStatus::Passed)]);
            assert_eq!(run.code, 0, "{}", run.all());
        }
        let run = p.record(truth, "E501", "verified", &["--cited", cited]);
        assert_eq!(run.code, 0, "{cited}: {}", run.all());
        assert_eq!(
            state(&p, truth, "E501"),
            ("fresh".to_string(), true),
            "{cited}"
        );
    }
    let p = bare();
    let run = p.record("v1.0/01/01-01/AR1", "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
}

/// What a verdict cites must exist: the host refuses a file the project does not hold, and the command a test that
/// names no collected test fact (with none collected, or another test collected), on any rule, so a citation is never
/// one that nothing reads and no edit stales. Nothing is written.
#[test]
fn a_cited_file_or_test_that_does_not_exist_is_refused() {
    let truth = "v1.0/01/01-01/TR1";
    let p = bare();
    let spec = p.read("spec/plans.spec");
    let run = p.record_over(
        truth,
        "E501",
        "verified",
        "{}",
        &["--cited", r#"[{"file":"src/missing.js"}]"#],
    );
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all()
            .contains("it cites 'src/missing.js', which the project does not hold"),
        "{}",
        run.all()
    );
    let made_up = r#"[{"test":"test/tasks.test.js#a task is added"}]"#;
    let run = p.record(truth, "E501", "verified", &["--cited", made_up]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains(
            "cited: 'test/tasks.test.js#a task is added' names no collected test fact; none is collected"
        ),
        "{}",
        run.all()
    );
    let run = p.node_tests(&[("test/tasks.test.js", "a task is added", TestStatus::Passed)]);
    assert_eq!(run.code, 0, "{}", run.all());
    for (rule, entity) in [("E501", truth), ("E501", "v1.0/01/01-01/AR1")] {
        let run = p.record(
            entity,
            rule,
            "verified",
            &[
                "--cited",
                r#"[{"test":"test/tasks.test.js#a task is removed"}]"#,
            ],
        );
        assert_ne!(run.code, 0, "{entity}: {}", run.all());
        assert!(
            run.all().contains(
                "cited: 'test/tasks.test.js#a task is removed' names no collected test fact; cite a test fact by its id"
            ),
            "{entity}: {}",
            run.all()
        );
    }
    assert_eq!(p.read("spec/plans.spec"), spec, "nothing is written");
}

/// A truth's inputs name no plan-wide file list: its code inputs are its verdict's citations. An artifact still reads
/// its own file.
#[test]
fn a_truths_inputs_are_its_citations_not_its_plans_files() {
    let p = bare();
    let names = |id: &str| -> Vec<String> {
        p.inputs(id, "E501")["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .filter_map(|i| Some(i["name"].as_str()?.to_string()))
            .collect()
    };
    let truth = names("v1.0/01/01-01/TR1");
    assert!(!truth.iter().any(|n| n == "files"), "{truth:?}");
    assert!(names("v1.0/01/01-01/AR1").iter().any(|n| n == "file"));
}

#[test]
fn a_plan_check_pass_is_replaced_by_issues() {
    let p = project("plan");
    p.settings(json!({ "profile": "gsd-strict" }));
    record_all(&p, PLAN);
    assert_eq!(status_of(&p, PLAN), "checked");
    let run = p.record(PLAN, "W311", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    // Fail closed: the failing verdict is fresh and passes nothing, and the plan is no longer checked.
    assert_eq!(state(&p, PLAN, "W311"), ("fresh".to_string(), false));
    assert_ne!(status_of(&p, PLAN), "checked");
}

const AUDIT: &str = "v1.0/A1";

#[test]
fn e518_issues_records() {
    let p = project("verify");
    let run = p.record(AUDIT, "E518", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(AUDIT)["integration"], "issues");
    assert_eq!(state(&p, AUDIT, "E518"), ("fresh".to_string(), false));
    assert_ne!(p.fields_of(AUDIT)["status"], "passed");
}

const SPEC: &str = "v1.0/02/02-SPEC";
const LOW: &str =
    r#"{"scores":{"goal":10,"boundary":5,"constraint":0,"acceptance":0,"ambiguity":95}}"#;

#[test]
fn e202_failed_records() {
    let p = project("discuss");
    let run = p.record(SPEC, "E202", "failed", &["--values", LOW]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(SPEC)["scores"]["goal"], 10);
}

#[test]
fn w203_failed_records_under_strict() {
    let p = project("discuss");
    p.settings(json!({ "profile": "gsd-strict" }));
    let req = p.nodes_of("locked_req")[0]["id"]
        .as_str()
        .expect("an id")
        .to_string();
    let run = p.record(&req, "W203", "failed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p, &req, "W203"), ("fresh".to_string(), false));
}

/// M5: creating a judged target adds a `no verdict` finding, which is owed a verdict and refuses nothing. The must-haves
/// are created in the executed plan 01-01: a must-have owes its verdict once its plan is complete.
// An artifact is not created here: `--path` is the CLI's own project option, so the CLI cannot spell its `path` field.
// Each creation, with the id it makes and the rule that owes it a verdict.
const CREATIONS: [(&str, &[&str]); 3] = [
    (
        "spec/plans.spec",
        &[
            "new",
            "truth",
            "TR9",
            "A new claim",
            "--plan",
            "v1.0/01/01-01",
            "--text",
            "A new claim",
        ],
    ),
    (
        "spec/plans.spec",
        &[
            "new",
            "key_link",
            "KL9",
            "Another link",
            "--plan",
            "v1.0/01/01-01",
            "--from",
            "test/tasks.test.js",
            "--to",
            "src/tasks.js",
            "--via",
            "x",
        ],
    ),
    (
        "spec/audit.spec",
        &[
            "new",
            "milestone_audit",
            "A2",
            "Second audit",
            "--milestone",
            "v1.0",
        ],
    ),
];

fn create(p: &Project, file: &str, args: &[&str]) -> Run {
    let mut all = args.to_vec();
    all.extend(["--in", file]);
    p.run(&all)
}

#[test]
fn an_agent_creates_a_judged_target() {
    let p = project("verify");
    for (file, args) in CREATIONS {
        let run = create(&p, file, args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.all());
    }
    // Each is owed a verdict and has none.
    for (id, rule) in [
        ("v1.0/01/01-01/TR9", "E501"),
        ("v1.0/01/01-01/KL9", "E501"),
        ("v1.0/A2", "E518"),
    ] {
        assert_eq!(state(&p, id, rule), ("missing".to_string(), false), "{id}");
    }
}

#[test]
fn an_edit_that_stales_a_pass_is_allowed() {
    let p = bare();
    verified(&p, "v1.0/01/01-01/TR1");
    let run = p.run(&[
        "set",
        "v1.0/01/01-01/TR1",
        "text",
        "A user can add a task and see it",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    // Fail closed: the verdict is stale, not passing, and the verification no longer reads passed.
    assert_eq!(
        state(&p, "v1.0/01/01-01/TR1", "E501"),
        ("stale".to_string(), false)
    );
    assert_ne!(verification(&p)["status"], "passed");
    let p = project("verify");
    assert_eq!(p.record(AUDIT, "E518", "passed", &[]).code, 0);
    assert_eq!(
        p.run(&["set", "TASK-01", "text", "Add a task, fast"]).code,
        0
    );
    assert_eq!(state(&p, AUDIT, "E518"), ("stale".to_string(), false));
    assert_ne!(p.fields_of(AUDIT)["status"], "passed");
}
