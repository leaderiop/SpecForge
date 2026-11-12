//! `@javascript/source` end to end: its real blob collects the fixture workspace `tests/fixtures/javascript-source`
//! (ADR 0347), the host's own reader parsing its source, beside `@javascript/node-test` and a scripted machine writing
//! node's JUnit report of the workspace's one test.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::sync::Arc;

use serde_json::json;
use specforge_effects::testing::MemoryConsent;
use specforge_effects::{EffectsMode, Ports};
use specforge_ops::analysis::collect::{Mode, Outcome, Request, RunnerOutput, collect};
use specforge_ops::asking::Asking;
use specforge_ops::testing::{ScriptedMachine, ScriptedRun};
use specforge_ops::{Caller, Containment};
use specforge_project::testing::TestProject;
use specforge_protocol_types::Fact;

const COLLECTOR: &str = "javascript-source";
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/javascript-source"
);
const UNOWNED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/javascript-source-unowned"
);

/// `fixture`, `@code/core` and `@javascript/source` enabled, and `@javascript/node-test` when `node_test`.
fn project(fixture: &str, node_test: bool) -> TestProject {
    let mut packs = vec!["@code/core", "@javascript/source"];
    if node_test {
        packs.push("@javascript/node-test");
    }
    TestProject::copy_of(fixture).enabling(&packs)
}

/// node's JUnit report of the workspace's one test, `adds` in `packages/core/test/board.test.ts`, its file absolute.
fn junit(p: &TestProject) -> String {
    let file = p.root().join("packages/core/test/board.test.ts");
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><testsuites><testcase name="adds" classname="test" file="{}"/></testsuites>"#,
        file.display()
    )
}

/// A machine whose one run writes `report` and exits 0.
fn machine(report: String) -> ScriptedMachine {
    ScriptedMachine::new().answering(ScriptedRun {
        files: vec![("".into(), report)],
        exit_code: Some(0),
        ..ScriptedRun::default()
    })
}

/// Collect `runner` (every detected collector when `None`) of `p` through `machine`, `consent` answering.
fn collected_with(
    p: &TestProject,
    runner: Option<&str>,
    machine: &ScriptedMachine,
    consent: &MemoryConsent,
) -> Outcome {
    let compiled = p.compile();
    let view = specforge_ops::view::ProjectView::of(&compiled);
    // The disk, as the CLI lends it: the syntax import reads through the host's own reader.
    let ports = Ports {
        fs: Arc::new(specforge_fs::LocalFs),
        ..specforge_effects::testing::ports()
    };
    let containment = Containment::unbounded();
    collect(
        &view,
        Request {
            runner,
            mode: Mode::Run(RunnerOutput::Discard),
            announce: &mut |_, _| {},
            caller: Caller::new(&containment, Asking::Nobody).lending(
                &ports,
                consent,
                EffectsMode::Online,
            ),
        },
        machine,
        consent,
    )
    .map_err(|refusal| format!("{}: {}", refusal.error.code, refusal.error.message))
    .unwrap()
}

/// The pack's collect of `p` alone, everything approved.
fn collected(p: &TestProject) -> Outcome {
    collected_with(
        p,
        Some(COLLECTOR),
        &ScriptedMachine::new(),
        &MemoryConsent::approving_everything(),
    )
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

fn ids<'o>(outcome: &'o Outcome, kind: &str) -> Vec<&'o str> {
    facts(outcome, COLLECTOR)
        .iter()
        .filter(|f| f.kind == kind)
        .map(|f| f.id.as_str())
        .collect()
}

fn fact<'o>(outcome: &'o Outcome, id: &str) -> &'o Fact {
    facts(outcome, COLLECTOR)
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("{COLLECTOR} answered no {id}"))
}

#[specforge_test_macros::test(
    behavior = "js_declare_source_collector",
    verify = "javascript-source runs without a command or consent"
)]
fn javascript_source_runs_without_a_command_or_consent() {
    let p = project(FIXTURE, false);
    let machine = ScriptedMachine::new();
    let outcome = collected_with(&p, None, &machine, &MemoryConsent::default());
    let runners: Vec<(&str, &str)> = outcome
        .runners
        .iter()
        .map(|r| (r.name.as_str(), r.extension.as_str()))
        .collect();
    assert_eq!(runners, [(COLLECTOR, "@javascript/source")]);
    assert!(machine.runs().is_empty(), "nothing is run");
    assert!(!ids(&outcome, "item").is_empty());
}

#[specforge_test_macros::test(
    behavior = "js_mint_item_facts",
    verify = "a fixture workspace's items are minted by the host's reader with ids, kinds and places"
)]
fn a_fixture_workspace_collects_its_items_with_ids_kinds_and_locations() {
    let outcome = collected(&project(FIXTURE, false));
    let core = |path: &str| format!("js:@acme/core/src/index.ts::{path}");
    let mut expected: Vec<String> = [
        "Board",
        "Board.cells",
        "Board.empty",
        "Board.place",
        "Geo",
        "Geo.distance",
        "Id",
        "Shape",
        "Shape.area",
        "add",
        "helper",
        "parse",
    ]
    .iter()
    .map(|path| core(path))
    .collect();
    expected.extend(
        [
            "js:@acme/core/src/view.tsx::View",
            "js:app/packages/cli/bin/cli.mjs::main",
            "js:app/scripts/legacy.cjs::build",
            "js:app/scripts/legacy.cjs::clean",
            "js:app/scripts/legacy.cjs::version",
        ]
        .map(str::to_string),
    );
    assert_eq!(
        ids(&outcome, "item"),
        expected,
        "nothing from test/, coverage/ or *.min.js"
    );

    let add = fact(&outcome, &core("add"));
    assert_eq!(add.fields["kind"], json!(["function"]));
    assert_eq!(add.fields["visibility"], "public");
    assert_eq!(add.fields["doc"], "Adds two numbers.");
    assert_eq!(add.fields["file"], "packages/core/src/index.ts");
    assert_eq!(add.fields["package"], "pkg:npm/%40acme/core");
    assert_eq!(
        (
            add.at[0].file.as_str(),
            add.at[0].start_line,
            add.at[0].end_line
        ),
        ("packages/core/src/index.ts", 2, 4)
    );
    let parse = fact(&outcome, &core("parse"));
    let lines: Vec<usize> = parse.at.iter().map(|at| at.start_line).collect();
    assert_eq!(lines, [6, 7, 8], "the overloads and the body are one item");
    assert_eq!(
        fact(&outcome, &core("Id")).fields["kind"],
        json!(["type", "const"])
    );
    assert_eq!(
        fact(&outcome, &core("Board.cells")).fields["visibility"],
        "private"
    );
    assert_eq!(
        fact(&outcome, &core("helper")).fields["visibility"],
        "private"
    );
    let legacy = |name: &str| {
        fact(&outcome, &format!("js:app/scripts/legacy.cjs::{name}")).fields["visibility"].clone()
    };
    assert_eq!(
        [legacy("build"), legacy("clean"), legacy("version")],
        ["public", "public", "public"].map(|v| json!(v))
    );
    assert_eq!(
        ids(&outcome, "package"),
        ["pkg:npm/%40acme/core", "pkg:npm/app"]
    );
}

#[specforge_test_macros::test(
    behavior = "js_select_source_files",
    verify = "a test file mints no item beside its runner's test"
)]
fn a_test_file_mints_no_item_beside_its_runners_test() {
    let p = project(FIXTURE, true);
    let outcome = collected_with(
        &p,
        None,
        &machine(junit(&p)),
        &MemoryConsent::approving_everything(),
    );
    let tests: Vec<&str> = facts(&outcome, "node-test")
        .iter()
        .filter(|f| f.kind == "test")
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(tests, ["js:@acme/core/test/board.test.ts#adds"]);
    let from_tests: Vec<&str> = ids(&outcome, "item")
        .into_iter()
        .filter(|id| id.contains("/test/"))
        .collect();
    assert!(from_tests.is_empty(), "{from_tests:?}");
}

#[specforge_test_macros::test(
    behavior = "js_select_source_files",
    verify = "a file under no named package mints nothing"
)]
fn a_file_under_no_named_package_mints_nothing() {
    let outcome = collected(&project(UNOWNED, false));
    assert_eq!(facts(&outcome, COLLECTOR), &[] as &[Fact]);
    let runner = outcome
        .runners
        .iter()
        .find(|r| r.name == COLLECTOR)
        .unwrap();
    assert_eq!(runner.skip_reason, None, "it ran and found nothing to read");
}

#[specforge_test_macros::test(
    behavior = "js_mint_item_facts",
    verify = "the items point at their package fact"
)]
fn the_items_point_at_their_package_fact() {
    let p = project(FIXTURE, false);
    let mut outcome = collected(&p);
    let packages = ids(&outcome, "package");
    for item in facts(&outcome, COLLECTOR)
        .iter()
        .filter(|f| f.kind == "item")
    {
        let package = item.fields["package"].as_str().unwrap();
        assert!(
            packages.contains(&package),
            "{} points at {package}",
            item.id
        );
    }
    let core = fact(&outcome, "pkg:npm/%40acme/core");
    assert_eq!(
        core.fields["depends_on"],
        json!(["pkg:npm/app"]),
        "never lodash"
    );
    assert_eq!(core.fields["version"], "1.0.0");
    assert_eq!(core.fields["engines"], json!(["node >=20"]));
    assert_eq!(
        fact(&outcome, "pkg:npm/app").fields["depends_on"],
        json!(["pkg:npm/%40acme/core"])
    );

    // Stored and served, every item's package reference resolves: @code/core finds none unheld or dangling.
    let compiled = p.compile();
    let view = specforge_ops::view::ProjectView::of(&compiled);
    let stored = std::mem::take(&mut outcome.facts);
    let dry_run = false;
    specforge_ops::authoring::apply::apply_collected(
        &view,
        Vec::new(),
        stored,
        dry_run,
        specforge_ops::testing::caller(),
    )
    .unwrap();
    let compiled = p.compile();
    let items = compiled.graph().nodes_by_kind("item");
    assert_eq!(items.len(), 17);
    let code: Vec<String> = compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code.starts_with("code/"))
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect();
    assert!(code.is_empty(), "{code:#?}");
}

#[specforge_test_macros::test(
    behavior = "js_mint_item_facts",
    verify = "a second collect writes the same facts"
)]
fn a_second_collect_writes_the_same_facts() {
    let p = project(FIXTURE, false);
    let first = collected(&p);
    let second = collected(&p);
    assert_eq!(facts(&first, COLLECTOR), facts(&second, COLLECTOR));
}
