//! The ledger fixture: a Rust project whose CI is red. Phase 01 was verified (`01-VERIFICATION.md` says 4/4) before
//! quick task 261105-tcl folded the total in an `i32`, which fails `total_beyond_i32_is_exact`. The twin carries the
//! verifier's verdicts, recorded over the run before the quick task (`goldens/ledger/rust-test-before-quick.xml`);
//! over the red run (`goldens/ledger/rust-test.xml`) exactly the truth that test proves goes stale, where the
//! Markdown still says passed.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::harness::{
    Project, RUST_TEST_COLLECTORS, ledger, planning_truths_and_summaries, unjudged,
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures/ledger")
}

fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity/goldens/ledger")
        .join(name)
}

fn collected(report: &str) -> Project {
    let p = ledger();
    let run = p.collect_report(&RUST_TEST_COLLECTORS, "rust-test", &golden(report));
    assert_eq!(run.code, 0, "{}", run.all());
    p
}

/// The `gsd/E501` diagnostics of the copy that are not `no verdict`, as `(code, message)`.
fn judged_not_pending(p: &Project) -> Vec<(String, String)> {
    p.report_all()
        .into_iter()
        .filter(|(code, message)| code == "gsd/E501" && !message.ends_with(": no verdict"))
        .collect()
}

/// The verifier's verdicts on phase 01 are committed into the twin's `spec/` and its harness ledger, fresh over the
/// run before the quick task and over no run. `GSD_BLESS=1` records them on a copy collected over that run and writes
/// the files back, the ledger without its `facts:` lines: the fact store is not committed.
#[test]
fn the_twin_holds_its_verifier_verdicts() {
    if std::env::var_os("GSD_BLESS").is_some() {
        let p = collected("rust-test-before-quick.xml");
        p.strip_verdicts();
        let recorded = p.verify_phase("v1.0/01");
        assert_eq!(
            recorded, 8,
            "phase 01's four truths, two artifacts and two key links"
        );
        for file in std::fs::read_dir(fixture().join("spec")).expect("the spec dir") {
            let name = format!(
                "spec/{}",
                file.expect("an entry").file_name().to_string_lossy()
            );
            std::fs::write(fixture().join(&name), p.read(&name)).expect("the fixture file");
        }
        let ledger_text = p.read("specforge-harness");
        let kept: Vec<&str> = ledger_text
            .lines()
            .filter(|l| {
                l.split(' ')
                    .nth(1)
                    .is_none_or(|key| !key.starts_with("facts:"))
            })
            .collect();
        std::fs::write(
            fixture().join("specforge-harness"),
            format!("{}\n", kept.join("\n")),
        )
        .expect("the fixture's ledger");
        eprintln!("recorded {recorded} verdicts into the twin");
    }
    assert_eq!(
        ledger().verify_phase("v1.0/01"),
        0,
        "no E501 judgement is pending in phase 01: GSD_BLESS=1 records them"
    );
    let green = collected("rust-test-before-quick.xml");
    assert_eq!(
        green.verify_phase("v1.0/01"),
        0,
        "the verdicts are fresh over the run before the quick task"
    );
    let verification = green.fields_of("v1.0/01/01-VERIFICATION");
    assert_eq!(
        (
            verification["status"].clone(),
            verification["score"].clone()
        ),
        ("passed".into(), "4/4".into()),
        "as 01-VERIFICATION.md says"
    );
    assert_eq!(unjudged(&green.report_all()), Vec::new());
}

/// Over the red run, one verdict is no longer fresh: phase 01's truth that `total_beyond_i32_is_exact` proves, its
/// tests changed. The failing test is also named on that truth (`code/W401`) and on the deliverable of the summary
/// that claimed it passes (`gsd/W408`), and nothing else is reported.
#[test]
fn the_red_run_stales_exactly_the_truth_its_failing_test_proves() {
    let p = collected("rust-test.xml");
    let stale = judged_not_pending(&p);
    assert_eq!(stale.len(), 1, "{stale:#?}");
    let (_, message) = &stale[0];
    assert!(
        message.starts_with("truth 'v1.0/01/01-02/TR2' ")
            && message.ends_with("stale: tests changed"),
        "{message}"
    );
    let others: Vec<String> = unjudged(&p.report_all())
        .into_iter()
        .map(|(code, message)| {
            let quoted = message
                .split('`')
                .nth(1)
                .or_else(|| message.split('\'').nth(1));
            format!("{code} {}", quoted.unwrap_or_default())
        })
        .collect();
    assert_eq!(
        others,
        [
            "code/W401 v1.0/01/01-02/TR2",
            "gsd/W408 v1.0/01/01-02/01-02-SUMMARY/D1"
        ],
        "{:#?}",
        p.report_all()
    );
    // Where 01-VERIFICATION.md still says passed, 4/4.
    let verification = p.fields_of("v1.0/01/01-VERIFICATION");
    assert_eq!(
        (
            verification["status"].clone(),
            verification["score"].clone()
        ),
        ("pending".into(), "3/4".into())
    );
}

/// `specforge trace LEDG-02 --partial` (the fixture has no commit) follows the requirement to the failing test: its
/// run shows `(failed)`.
#[test]
fn the_trace_of_ledg_02_reaches_the_failed_test() {
    let p = collected("rust-test.xml");
    let trace = p.run(&["trace", "LEDG-02", "--partial"]);
    assert_eq!(trace.code, 0, "{}", trace.all());
    let failed: Vec<&str> = trace
        .stdout
        .lines()
        .filter(|l| l.contains("(failed)"))
        .collect();
    assert_eq!(failed.len(), 1, "{}", trace.stdout);
    assert!(
        failed[0].contains("total_beyond_i32_is_exact"),
        "{}",
        trace.stdout
    );
}

/// The twin holds what `.planning/` holds: the requirements and their checkboxes' ids, the roadmap's phases, every
/// plan's (and the quick task's) truths, and the summaries.
#[test]
fn the_twin_holds_what_its_planning_holds() {
    let planning = fixture().join(".planning");
    let read = |path: &Path| std::fs::read_to_string(path).expect("a planning file");
    let mut requirements: Vec<String> = read(&planning.join("REQUIREMENTS.md"))
        .lines()
        .filter_map(|l| l.split("**").nth(1).filter(|id| id.contains('-')))
        .filter(|id| id.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
        .map(|id| id.trim_end_matches(':').to_string())
        .collect();
    requirements.sort();
    requirements.dedup();
    let mut phases: Vec<String> = read(&planning.join("ROADMAP.md"))
        .lines()
        .filter_map(|l| l.strip_prefix("### Phase "))
        .map(|l| l.split_once(": ").expect("Phase N: Name").1.to_string())
        .collect();
    phases.sort();
    let (truths, summaries) = planning_truths_and_summaries(&planning);

    let p = ledger();
    let text = |n: &Value, field: &str| n["fields"][field].as_str().expect(field).to_string();
    let plan_of = |n: &Value| {
        text(n, "plan")
            .rsplit('/')
            .next()
            .expect("a plan")
            .to_string()
    };
    let mut twin_requirements: Vec<String> = p
        .nodes_of("requirement")
        .iter()
        .filter(|n| n["fields"]["tier"] != "out_of_scope")
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect();
    twin_requirements.sort();
    let mut twin_phases: Vec<String> = p
        .nodes_of("phase")
        .iter()
        .map(|n| text(n, "name"))
        .collect();
    twin_phases.sort();
    let mut twin_truths: Vec<(String, String)> = p
        .nodes_of("truth")
        .iter()
        .map(|n| (plan_of(n), text(n, "text")))
        .collect();
    twin_truths.sort();
    let mut twin_summaries: Vec<String> = p.nodes_of("summary").iter().map(plan_of).collect();
    twin_summaries.sort();
    assert_eq!(
        (
            requirements.len(),
            phases.len(),
            truths.len(),
            summaries.len()
        ),
        (5, 3, 7, 3)
    );
    assert_eq!(
        (twin_requirements, twin_phases, twin_truths, twin_summaries),
        (requirements, phases, truths, summaries)
    );
}
