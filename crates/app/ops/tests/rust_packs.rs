//! The Rust packs end to end: `@rust/cargo`, `@rust/source` and `@rust/test` from their real blobs collect the
//! fixture crate `tests/fixtures/rust-packs` (ADR 0348), the host's own reader parsing its source and a scripted
//! machine writing nextest 0.9.146's recorded report of it.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::sync::Arc;

use specforge_effects::testing::MemoryConsent;
use specforge_effects::{EffectsMode, Ports};
use specforge_ops::analysis::collect::{Mode, Outcome, Request, RunnerOutput, collect};
use specforge_ops::asking::Asking;
use specforge_ops::testing::{ScriptedFailure, ScriptedMachine, ScriptedRun};
use specforge_ops::{Caller, Containment};
use specforge_project::testing::TestProject;
use specforge_protocol_types::Fact;

const PACKS: [&str; 5] = [
    "@specforge/software",
    "@code/core",
    "@rust/cargo",
    "@rust/source",
    "@rust/test",
];
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rust-packs");
/// nextest 0.9.146's JUnit report of the fixture (its `#[specforge_test]` written `#[test]` to build without the
/// macro): `tests::adds` in the lib, the bin and the integration test `api`; the ignored `tests::slow` absent.
const JUNIT: &str = include_str!("fixtures/rust-packs/junit.xml");

/// The fixture crate, the Rust packs enabled.
fn fixture() -> TestProject {
    TestProject::copy_of(FIXTURE).enabling(&PACKS)
}

/// A machine whose one run writes `report` and exits 0, or fails `failure`.
fn machine(failure: Option<ScriptedFailure>) -> ScriptedMachine {
    ScriptedMachine::new().answering(ScriptedRun {
        files: vec![("".into(), JUNIT.to_string())],
        exit_code: Some(0),
        fails: failure,
        ..ScriptedRun::default()
    })
}

/// Collect `runner` (every detected collector when `None`) of `p` through `machine`, everything approved.
fn collected(p: &TestProject, runner: Option<&str>, machine: &ScriptedMachine) -> Outcome {
    collect_or_refused(p, runner, machine).unwrap()
}

fn collect_or_refused(
    p: &TestProject,
    runner: Option<&str>,
    machine: &ScriptedMachine,
) -> Result<Outcome, String> {
    let compiled = p.compile();
    let view = specforge_ops::view::ProjectView::of(&compiled);
    // The disk, as the CLI lends it: rust-source's reads go through the host's own reader.
    let ports = Ports {
        fs: Arc::new(specforge_fs::LocalFs),
        ..specforge_effects::testing::ports()
    };
    let consent = MemoryConsent::approving_everything();
    let containment = Containment::unbounded();
    collect(
        &view,
        Request {
            runner,
            mode: Mode::Run(RunnerOutput::Discard),
            announce: &mut |_, _| {},
            caller: Caller::new(&containment, Asking::Nobody).lending(
                &ports,
                &consent,
                EffectsMode::Online,
            ),
        },
        machine,
        &consent,
    )
    .map_err(|refusal| format!("{}: {}", refusal.error.code, refusal.error.message))
}

/// The facts `collector` answered.
fn facts<'o>(outcome: &'o Outcome, collector: &str) -> &'o [Fact] {
    outcome
        .facts
        .stores
        .iter()
        .find(|s| s.collector == collector)
        .map_or(&[], |s| &s.facts)
}

fn fact<'o>(outcome: &'o Outcome, collector: &str, id: &str) -> &'o Fact {
    facts(outcome, collector)
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("{collector} answered no {id}"))
}

#[specforge_test_macros::test(
    behavior = "rc_map_manifests_to_package_facts",
    verify = "the rust packs run in fact order over a fixture crate"
)]
fn the_rust_packs_declare_their_collectors_and_run_in_fact_order() {
    let p = fixture();
    let outcome = collected(&p, None, &machine(None));
    let order: Vec<(&str, &str)> = outcome
        .runners
        .iter()
        .map(|r| (r.name.as_str(), r.extension.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("rust-cargo", "@rust/cargo"),
            ("rust-source", "@rust/source"),
            ("rust-test", "@rust/test"),
        ],
        "each runs after the collector whose facts it reads"
    );
    let ids: Vec<&str> = facts(&outcome, "rust-cargo")
        .iter()
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(ids, ["pkg:cargo/adder"]);
}

#[specforge_test_macros::test(
    behavior = "rs_mint_item_facts",
    verify = "a fixture crate's items are minted from its module tree by the host's reader"
)]
fn a_fixture_crate_collects_its_packages_items_and_tests() {
    let p = fixture();
    let outcome = collected(&p, None, &machine(None));

    let package = fact(&outcome, "rust-cargo", "pkg:cargo/adder");
    assert_eq!(
        package.fields["targets"],
        serde_json::json!([
            "lib:adder:src/lib.rs",
            "bin:adder:src/main.rs",
            "test:api:tests/api.rs"
        ])
    );
    let add = fact(&outcome, "rust-source", "rust:adder::ops::add");
    assert_eq!(add.fields["file"], "src/math/ops.rs", "the #[path] module");
    assert_eq!(add.fields["doc"], "Adds two numbers.");
    let adds = fact(&outcome, "rust-source", "rust:adder::tests::adds");
    assert_eq!(adds.fields["names"], serde_json::json!(["rs_mark_tests"]));
    assert_eq!(adds.fields["test_id"], "rust:adder#tests::adds");
    assert_eq!(adds.at[0].start_line, 14, "its fn line");

    let tests: Vec<(&str, &str, &str)> = facts(&outcome, "rust-test")
        .iter()
        .map(|f| {
            (
                f.kind.as_str(),
                f.id.as_str(),
                f.fields["status"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        tests,
        [
            ("test", "rust:adder#tests::adds", "passed"),
            ("test_run", "run:rust:adder#tests::adds", "passed"),
            ("test", "rust:adder#tests::slow", "skipped"),
            ("test", "rust:adder::api#tests::adds", "passed"),
            ("test_run", "run:rust:adder::api#tests::adds", "passed"),
            ("test", "rust:adder::bin/adder#tests::adds", "passed"),
            (
                "test_run",
                "run:rust:adder::bin/adder#tests::adds",
                "passed"
            ),
        ],
        "three tests::adds, and the ignored test nextest omits held as skipped"
    );
    let named = fact(&outcome, "rust-test", "rust:adder#tests::adds");
    assert_eq!(named.fields["names"], serde_json::json!(["rs_mark_tests"]));
    assert_eq!(named.fields["file"], "src/lib.rs");
    assert_eq!(named.at, adds.at);
}

#[specforge_test_macros::test(
    behavior = "rt_map_nextest_report",
    verify = "rust-test writes its nextest config and runs cargo-nextest with it"
)]
fn rust_test_writes_its_nextest_config_and_runs_cargo_nextest() {
    let p = fixture();
    let machine = machine(None);
    collected(&p, Some("rust-test"), &machine);

    let config = p.root().join(".specforge/reports/rust-test.nextest.toml");
    let argv = &machine.runs()[0].argv;
    assert_eq!(
        argv,
        &[
            "cargo-nextest".to_string(),
            "nextest".into(),
            "run".into(),
            "--workspace".into(),
            "--no-tests=warn".into(),
            "--tool-config-file".into(),
            format!("specforge:{}", config.display()),
            "--profile".into(),
            "specforge".into(),
        ]
    );
    let written = std::fs::read_to_string(&config).unwrap();
    let report = p.root().join(".specforge/reports/rust-test.xml");
    let escaped = report
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    assert!(
        written.contains(&format!("path = \"{escaped}\"")),
        "{written}"
    );
}

#[specforge_test_macros::test(
    behavior = "rt_declare_test_collector",
    verify = "a missing nextest names its install line and keeps the packages and items"
)]
fn a_missing_nextest_says_to_install_it() {
    let p = fixture();
    let said = "install it: `cargo install cargo-nextest --locked`";
    let outcome = collected(&p, None, &machine(Some(ScriptedFailure::NotFound)));
    let test = outcome
        .runners
        .iter()
        .find(|r| r.name == "rust-test")
        .unwrap();
    assert!(
        test.skip_reason.as_deref().unwrap().contains(said),
        "{:?}",
        test.skip_reason
    );
    assert!(!facts(&outcome, "rust-source").is_empty());

    let refused = collect_or_refused(
        &p,
        Some("rust-test"),
        &machine(Some(ScriptedFailure::NotFound)),
    )
    .unwrap_err();
    assert!(refused.starts_with("E045: "), "{refused}");
    assert!(refused.contains(said), "{refused}");
}

#[specforge_test_macros::test(
    behavior = "rs_mint_item_facts",
    verify = "a second collect answers the same facts"
)]
fn a_second_collect_reads_the_same_facts() {
    let p = fixture();
    let first = collected(&p, None, &machine(None));
    let second = collected(&p, None, &machine(None));
    for collector in ["rust-cargo", "rust-source"] {
        assert_eq!(facts(&first, collector), facts(&second, collector));
    }
    let ids = |o: &Outcome| -> Vec<String> {
        facts(o, "rust-test").iter().map(|f| f.id.clone()).collect()
    };
    assert_eq!(ids(&first), ids(&second));
}

/// `p` collected by every detected collector, its facts stored as `specforge collect` stores them.
fn stored(p: &TestProject) {
    let mut outcome = collected(p, None, &machine(None));
    let compiled = p.compile();
    let view = specforge_ops::view::ProjectView::of(&compiled);
    let dry_run = false;
    specforge_ops::authoring::apply::apply_collected(
        &view,
        Vec::new(),
        std::mem::take(&mut outcome.facts),
        dry_run,
        specforge_ops::testing::caller(),
    )
    .unwrap();
}

#[specforge_test_macros::test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "find_implementation lists every anchor of the entity, by file and line"
)]
fn a_test_naming_a_behavior_is_its_anchor() {
    let p = fixture();
    let anchors = |p: &TestProject| {
        let compiled = p.compile();
        let view = specforge_ops::view::ProjectView::of(&compiled);
        let (found, notices) =
            specforge_ops::read::navigate::anchors_of_entity(&view, "rs_mark_tests")
                .unwrap()
                .into_value_and_notices();
        let codes: Vec<String> = notices.iter().map(|n| n.code.clone()).collect();
        (found, codes)
    };
    // Nothing collected yet: no anchor, and I212 says there is nothing to anchor to.
    let (found, codes) = anchors(&p);
    assert_eq!((found.count, codes), (0, vec!["I212".to_string()]));

    stored(&p);
    // `tests::adds` names `rs_mark_tests`: the edge of its item fact and of its test fact to the behavior are its
    // anchors, both at the test fn. The file is as the collect read it: no W198.
    let (found, codes) = anchors(&p);
    assert_eq!(codes, Vec::<String>::new());
    let rows: Vec<_> = found
        .implementations
        .iter()
        .map(|a| {
            (
                a.fact_id.as_str(),
                a.file.as_str(),
                a.line,
                a.item_kind.as_str(),
                a.collector.as_str(),
                a.via.as_str(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "rust:adder#tests::adds",
                "src/lib.rs",
                14,
                "test",
                "rust-test",
                "test.names"
            ),
            (
                "rust:adder::tests::adds",
                "src/lib.rs",
                14,
                "function",
                "rust-source",
                "item.names"
            ),
        ]
    );
    let anchor = &found.implementations[1];
    assert!(anchor.end_line >= anchor.line, "{anchor:?}");
}

/// The syntax work `collector` reported in `outcome`: (parsed, cached), (0, 0) for none.
fn syntax(outcome: &Outcome, collector: &str) -> (usize, usize) {
    let runner = outcome
        .runners
        .iter()
        .find(|r| r.name == collector)
        .unwrap();
    runner
        .syntax
        .map_or((0, 0), |work| (work.parsed, work.cached))
}

/// The messages of `outcome`'s diagnostics with `code`.
fn said(outcome: &Outcome, code: &str) -> Vec<String> {
    outcome
        .diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| d.message.clone())
        .collect()
}

/// The fixture's Rust files `@rust/source` reads: its module tree from `src/lib.rs` and `src/main.rs`, and `tests/`.
const READ: usize = 4;

#[specforge_test_macros::test(
    behavior = "dispatch_collector",
    verify = "a second collect of an unchanged tree parses no file: each is read from the syntax cache"
)]
fn a_second_collect_parses_no_file() {
    let p = fixture();
    let first = collected(&p, None, &machine(None));
    assert_eq!(syntax(&first, "rust-source"), (READ, 0));
    assert_eq!(
        said(&first, "I214"),
        [format!(
            "collector 'rust-source' read {READ} source files: {READ} parsed, 0 from the syntax cache"
        )]
    );
    let second = collected(&p, None, &machine(None));
    assert_eq!(syntax(&second, "rust-source"), (0, READ));
    assert_eq!(syntax(&second, "rust-cargo"), (0, 0), "it parses no source");
    assert_eq!(said(&second, "W200"), Vec::<String>::new());
}

#[specforge_test_macros::test(
    behavior = "dispatch_collector",
    verify = "a collect after one file's edit parses that file alone"
)]
fn one_edit_parses_one_file() {
    let p = fixture();
    collected(&p, None, &machine(None));
    let ops = p.root().join("src/math/ops.rs");
    let mut text = std::fs::read_to_string(&ops).unwrap();
    text.push_str("// one edit\n");
    std::fs::write(&ops, text).unwrap();
    let edited = collected(&p, None, &machine(None));
    assert_eq!(syntax(&edited, "rust-source"), (1, READ - 1));
}

#[specforge_test_macros::test(
    behavior = "dispatch_collector",
    verify = "a syntax cache that cannot be written is W200 once, and the next collect parses again"
)]
fn an_unwritable_syntax_cache_is_w200() {
    let p = fixture();
    // A file where the cache's directory goes: no entry can be written under it.
    let cache = p.root().join(specforge_source::CACHE_DIR);
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::write(&cache, "in the way").unwrap();
    let first = collected(&p, None, &machine(None));
    assert_eq!(
        said(&first, "W200"),
        [format!(
            "the syntax cache at {} could not be written for {READ} parsed files: the next collect parses them again",
            specforge_source::CACHE_DIR
        )]
    );
    let second = collected(&p, None, &machine(None));
    assert_eq!(syntax(&second, "rust-source"), (READ, 0));
}
