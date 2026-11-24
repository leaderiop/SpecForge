//! The Rust report packs end to end (ADR 0389): `@rust/coverage`, `@rust/lint`, `@rust/deps`, `@rust/api` and
//! `@rust/proof` from their real blobs collect the fixture crate `tests/fixtures/rust-packs`, a scripted machine
//! standing in for each tool. Each report is a short inline text in the format of its pack's own fixture: LCOV for
//! cargo-llvm-cov, clippy's JSON messages, a CycloneDX BOM for cargo-cyclonedx, cargo-public-api's text and Kani's
//! regular output.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::sync::Arc;
use std::time::Duration;

use specforge_effects::testing::MemoryConsent;
use specforge_effects::{EffectsMode, Ports};
use specforge_ops::analysis::collect::{Mode, Outcome, Request, RunnerOutput, collect};
use specforge_ops::asking::Asking;
use specforge_ops::testing::{ScriptedMachine, ScriptedRun};
use specforge_ops::{Caller, Containment};
use specforge_project::testing::TestProject;
use specforge_protocol_types::Fact;

/// The five report packs, the packs whose facts they read, and `@specforge/software`, whose behaviors the fixture's
/// tests name.
const PACKS: [&str; 9] = [
    "@specforge/software",
    "@code/core",
    "@rust/cargo",
    "@rust/source",
    "@rust/coverage",
    "@rust/lint",
    "@rust/deps",
    "@rust/api",
    "@rust/proof",
];
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rust-packs");

/// The fixture crate, the report packs enabled, and the packages and items of `@rust/cargo` and `@rust/source`
/// collected and stored, as a collect of every detected collector stores them before a report pack runs.
fn fixture() -> TestProject {
    let p = TestProject::copy_of(FIXTURE).enabling(&PACKS);
    for collector in ["rust-cargo", "rust-source"] {
        // Neither runs a command: nothing is scripted.
        let mut outcome = collected(&p, collector, &ScriptedMachine::new());
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
    p
}

/// A machine whose one run does `run` and exits 0.
fn machine(run: ScriptedRun) -> ScriptedMachine {
    ScriptedMachine::new().answering(ScriptedRun {
        exit_code: Some(0),
        ..run
    })
}

/// A run that writes `report` at the report location.
fn writing(report: &str) -> ScriptedRun {
    ScriptedRun {
        files: vec![("".into(), report.to_string())],
        ..ScriptedRun::default()
    }
}

/// A run that prints `stdout`, which a capture pack reads.
fn printing(stdout: &str) -> ScriptedRun {
    ScriptedRun {
        stdout: Some(stdout.to_string()),
        ..ScriptedRun::default()
    }
}

/// Collect `runner` of `p` through `machine`, everything approved; a refusal is its code and message.
fn collect_or_refused(
    p: &TestProject,
    runner: &str,
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
            runner: Some(runner),
            mode: Mode::Run(RunnerOutput::Discard),
            announce: &mut |_, _| {},
            filter: None,
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

fn collected(p: &TestProject, runner: &str, machine: &ScriptedMachine) -> Outcome {
    collect_or_refused(p, runner, machine).unwrap()
}

/// The facts `collector` answered of `kind`.
fn facts<'o>(outcome: &'o Outcome, collector: &str, kind: &str) -> Vec<&'o Fact> {
    outcome
        .facts
        .stores
        .iter()
        .filter(|s| s.collector == collector)
        .flat_map(|s| &s.facts)
        .filter(|f| f.kind == kind)
        .collect()
}

fn ids(facts: &[&Fact]) -> Vec<String> {
    facts.iter().map(|f| f.id.clone()).collect()
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

/// cargo-llvm-cov's LCOV of the fixture: two lines of `tests::adds` in `src/lib.rs`, and one record of the standard
/// library under `/rustc/`, outside the project.
const LCOV: &str = "TN:\n\
SF:src/lib.rs\n\
FN:14,_RNvNtCs1_5adder5tests4adds\n\
FNDA:1,_RNvNtCs1_5adder5tests4adds\n\
DA:14,1\n\
DA:15,1\n\
end_of_record\n\
SF:/rustc/0123456789abcdef/library/core/src/ops/arith.rs\n\
DA:1,1\n\
end_of_record\n";

#[specforge_test_macros::test(requirement = "FEATURE-143")]
#[specforge_test_macros::test(requirement = "FEATURE-128")]
fn rust_coverage_records_coverage_and_warns_of_a_file_outside_the_project() {
    let p = fixture();
    let outcome = collected(&p, "rust-coverage", &machine(writing(LCOV)));
    let coverage = facts(&outcome, "rust-coverage", "coverage");
    assert_eq!(ids(&coverage), ["cov:rust:adder::tests::adds"]);
    assert_eq!(coverage[0].fields["file"], "src/lib.rs");
    assert_eq!(coverage[0].fields["lines_hit"], 2);
    let warned = said(&outcome, "W203");
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert!(
        warned[0].contains("'rust-coverage'")
            && warned[0].contains("/rustc/0123456789abcdef/library/core/src/ops/arith.rs"),
        "{warned:?}"
    );
}

/// One clippy JSON message: `rule` at `line` of `src/lib.rs`, the primary span.
fn clippy(rule: &str, line: u32) -> String {
    serde_json::json!({
        "reason": "compiler-message",
        "package_id": "path+file:///project#adder@0.1.0",
        "target": {"kind": ["lib"], "name": "adder", "src_path": "src/lib.rs"},
        "message": {
            "$message_type": "diagnostic",
            "level": "warning",
            "message": format!("{rule} fired"),
            "code": {"code": rule, "explanation": null},
            "spans": [{
                "file_name": "src/lib.rs",
                "is_primary": true,
                "line_start": line,
                "line_end": line,
                "column_start": 5,
                "column_end": 9
            }],
            "children": []
        }
    })
    .to_string()
}

#[specforge_test_macros::test(requirement = "FEATURE-146")]
fn rust_lint_records_each_clippy_diagnostic() {
    let p = fixture();
    let stdout = format!(
        "{}\n{}\n{{\"reason\":\"build-finished\",\"success\":true}}\n",
        clippy("clippy::unwrap_used", 15),
        clippy("unused_variables", 20)
    );
    let outcome = collected(&p, "rust-lint", &machine(printing(&stdout)));
    let findings = facts(&outcome, "rust-lint", "static_finding");
    let cited: Vec<&serde_json::Value> = findings.iter().map(|f| &f.fields["item"]).collect();
    assert_eq!(
        cited,
        ["rust:adder::tests::adds", "rust:adder::tests::slow"],
        "each finding cites the innermost item holding its line: {:?}",
        ids(&findings)
    );
    assert!(said(&outcome, "W203").is_empty());
}

/// A cargo-cyclonedx BOM of the fixture crate whose one component is `name`.
fn bom(name: &str) -> String {
    serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": "path+file:///project#adder@0.1.0",
                "name": "adder",
                "version": "0.1.0",
                "purl": "pkg:cargo/adder@0.1.0?download_url=file://."
            }
        },
        "components": [{
            "type": "library",
            "bom-ref": format!("registry+https://github.com/rust-lang/crates.io-index#{name}@1.0.0"),
            "name": name,
            "version": "1.0.0",
            "scope": "required",
            "purl": format!("pkg:cargo/{name}@1.0.0")
        }]
    })
    .to_string()
}

/// Write `text` at `path` of `p`, stamped one second before the run starts: a file an earlier run left or the project
/// committed.
fn earlier(p: &TestProject, path: &str, text: &str) {
    p.write(path, text);
    std::fs::File::options()
        .write(true)
        .open(p.root().join(path))
        .unwrap()
        .set_modified(ScriptedMachine::started() - Duration::from_secs(1))
        .unwrap();
}

#[specforge_test_macros::test(requirement = "FEATURE-148")]
fn rust_sbom_reads_only_the_bom_the_run_wrote() {
    let p = fixture();
    // A BOM an earlier run left: the report glob reaches it, freshness does not.
    earlier(&p, "stale/rust-sbom.cdx.json", &bom("stale"));
    // The glob's base is the root: cargo-cyclonedx writes the BOM beside the root manifest.
    let run = ScriptedRun {
        files: vec![("rust-sbom.cdx.json".into(), bom("serde"))],
        ..ScriptedRun::default()
    };
    let outcome = collected(&p, "rust-sbom", &machine(run));
    let dependencies = ids(&facts(&outcome, "rust-sbom", "dependency"));
    assert_eq!(
        dependencies,
        ["dep:pkg:cargo/adder/pkg%3Acargo%2Fserde%401.0.0"]
    );
}

/// cargo-public-api's text of the fixture's library: its one public function, re-exported at the root.
const PUBLIC_API: &str = "pub mod adder\npub fn adder::add(a: i32, b: i32) -> i32\n";

#[specforge_test_macros::test(requirement = "FEATURE-150")]
fn rust_api_records_the_public_api() {
    let p = fixture();
    let outcome = collected(&p, "rust-api", &machine(printing(PUBLIC_API)));
    let api = facts(&outcome, "rust-api", "api_item");
    assert_eq!(ids(&api), ["api:rust:adder", "api:rust:adder::add"]);
    assert_eq!(api[1].fields["kind"], "function");
}

/// Each fact's id and its `change`, sorted by id.
fn changes(api: &[&Fact]) -> Vec<(String, Option<String>)> {
    let mut changes: Vec<_> = api
        .iter()
        .map(|f| {
            let change = f.fields.get("change").and_then(|c| c.as_str());
            (f.id.clone(), change.map(str::to_string))
        })
        .collect();
    changes.sort();
    changes
}

#[specforge_test_macros::test(requirement = "FEATURE-150")]
fn rust_api_reads_the_committed_accepted_api() {
    let p = fixture();
    // Committed before the run: the accepted side is read whatever its age, unlike a report.
    let accepted = format!("{PUBLIC_API}pub fn adder::sub(a: i32, b: i32) -> i32\n");
    earlier(&p, "tests/public-api.txt", &accepted);
    let outcome = collected(&p, "rust-api", &machine(printing(PUBLIC_API)));
    let change = |id: &str, change: &str| (id.to_string(), Some(change.to_string()));
    assert_eq!(
        changes(&facts(&outcome, "rust-api", "api_item")),
        [
            change("api:rust:adder", "unchanged"),
            change("api:rust:adder::add", "unchanged"),
            change("api:rust:adder::sub", "removed"),
        ]
    );
    assert_eq!(said(&outcome, "W203"), Vec::<String>::new());
}

#[specforge_test_macros::test(requirement = "FEATURE-150")]
fn an_accepted_report_past_64_kib_is_read() {
    let p = fixture();
    let mut accepted = PUBLIC_API.to_string();
    for n in 0..2700 {
        accepted.push_str(&format!("pub fn adder::generated_{n:04}() -> i32\n"));
    }
    assert!(accepted.len() > 96 * 1024, "{}", accepted.len());
    earlier(&p, "tests/public-api.txt", &accepted);
    let outcome = collected(&p, "rust-api", &machine(printing(PUBLIC_API)));
    let changes = changes(&facts(&outcome, "rust-api", "api_item"));
    assert!(
        changes.contains(&(
            "api:rust:adder::add".to_string(),
            Some("unchanged".to_string())
        )),
        "{:?}",
        &changes[..3]
    );
    let warned = said(&outcome, "W203");
    assert!(
        !warned.iter().any(|w| w.contains("too_large")),
        "{warned:?}"
    );
}

/// Kani's regular output: the harness `tests::slow`, which only the fixture's library holds, verified.
const KANI: &str = "Kani Rust Verifier 0.68.0 (cargo plugin)\n\
   Compiling adder v0.1.0 (/project)\n\
Checking harness tests::slow...\n\
\n\
RESULTS:\n\
Check 1: tests::slow.assertion.1\n\
\t - Status: SUCCESS\n\
\t - Description: \"assertion failed\"\n\
\n\
SUMMARY:\n\
 ** 0 of 1 failed\n\
\n\
VERIFICATION:- SUCCESSFUL\n\
Verification Time: 0.25s\n\
\n\
Manual Harness Summary:\n\
Complete - 1 successfully verified harnesses, 0 failures, 1 total.\n";

#[specforge_test_macros::test(requirement = "FEATURE-154")]
fn rust_proof_records_a_verified_harness_at_its_item() {
    let p = fixture();
    let outcome = collected(&p, "rust-proof", &machine(printing(KANI)));
    let tests = facts(&outcome, "rust-proof", "test");
    assert_eq!(ids(&tests), ["rust:adder#tests::slow"]);
    assert_eq!(tests[0].fields["status"], "passed");
    assert_eq!(tests[0].fields["file"], "src/lib.rs");
    assert!(!tests[0].at.is_empty(), "placed at the harness fn");
}

#[specforge_test_macros::test(requirement = "FEATURE-154")]
fn a_kani_output_cut_before_complete_is_the_collectors_error() {
    let p = fixture();
    let cut = &KANI[..KANI.find("Manual Harness Summary").unwrap()];
    let refused = collect_or_refused(&p, "rust-proof", &machine(printing(cut))).unwrap_err();
    // The run is refused whole: no proof result is stored, none dropped silently.
    assert!(refused.starts_with("E028: "), "{refused}");
    assert!(refused.contains("rust-proof"), "{refused}");
}
