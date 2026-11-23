//! The JavaScript report and runner packs end to end (ADR 0389): `@javascript/coverage`, `@javascript/lint`,
//! `@javascript/deps`, `@javascript/api`, `@javascript/vitest`, `@javascript/jest` and `@javascript/playwright` from
//! their real blobs collect the fixture workspace `tests/fixtures/javascript-source`, a scripted machine standing in
//! for each tool, beside the legacy `@specforge/vitest` (owner decision D4). Each report is a short inline text in the
//! format of its pack's own fixture (the recordings of c8, ESLint's SARIF formatter, cyclonedx-npm, osv-scanner,
//! api-extractor, vitest, jest-junit and Playwright).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::sync::Arc;

use specforge_effects::testing::MemoryConsent;
use specforge_effects::{EffectsMode, Ports};
use specforge_ops::analysis::collect::{Mode, Outcome, Request, RunnerOutput, collect};
use specforge_ops::asking::Asking;
use specforge_ops::testing::{ScriptedMachine, ScriptedRun};
use specforge_ops::{Caller, Containment};
use specforge_project::testing::TestProject;
use specforge_protocol_types::Fact;

/// `@javascript/source`, the seven packs, the legacy vitest pack with the peer it requires, and
/// `@specforge/software`, whose behaviors the fixture's spec declares.
const PACKS: [&str; 12] = [
    "@specforge/software",
    "@code/core",
    "@javascript/source",
    "@javascript/coverage",
    "@javascript/lint",
    "@javascript/deps",
    "@javascript/api",
    "@javascript/vitest",
    "@javascript/jest",
    "@javascript/playwright",
    "@specforge/testing",
    "@specforge/vitest",
];
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/javascript-source"
);
/// The workspace's one test file, in the package `@acme/core`.
const TEST_FILE: &str = "packages/core/test/board.test.ts";

/// The fixture workspace with the files each pack is detected by, the packs enabled, and `@javascript/source`'s
/// packages and items collected and stored, as a collect of every detected collector stores them first.
fn fixture() -> TestProject {
    let p = TestProject::copy_of(FIXTURE)
        .files(&[
            (
                "package-lock.json",
                r#"{"name": "app", "lockfileVersion": 3, "packages": {}}"#,
            ),
            ("eslint.config.js", "export default [];\n"),
            ("tsconfig.json", "{}\n"),
            ("vitest.config.ts", "export default {};\n"),
            ("jest.config.js", "module.exports = {};\n"),
            ("playwright.config.ts", "export default {};\n"),
        ])
        .enabling(&PACKS);
    // It runs no command: nothing is scripted.
    let mut outcome = collected(&p, "javascript-source", &ScriptedMachine::new());
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

/// Collect `runner` of `p` through `machine`, everything approved.
fn collected(p: &TestProject, runner: &str, machine: &ScriptedMachine) -> Outcome {
    let compiled = p.compile();
    let view = specforge_ops::view::ProjectView::of(&compiled);
    // The disk, as the CLI lends it: the syntax import reads through the host's own reader.
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
    .unwrap()
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

#[specforge_test_macros::test(requirement = "FEATURE-151")]
fn the_legacy_vitest_is_detected_beside_javascript_vitest() {
    let p = fixture();
    let compiled = p.compile();
    let errors: Vec<String> = compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code.starts_with('E'))
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    let view = specforge_ops::view::ProjectView::of(&compiled);
    let detected = specforge_ops::analysis::collect::detected_here(
        view.registries().declarations(),
        p.root(),
        None,
    );
    let named = |name: &str| {
        detected
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.extension.clone())
    };
    assert_eq!(named("vitest").as_deref(), Some("@specforge/vitest"));
    assert_eq!(
        named("javascript-vitest").as_deref(),
        Some("@javascript/vitest")
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-144")]
fn javascript_coverage_records_the_lines_of_an_item() {
    let p = fixture();
    // c8's LCOV: the two body lines of `add`.
    let lcov =
        "TN:\nSF:packages/core/src/index.ts\nFN:2,add\nFNDA:1,add\nDA:2,1\nDA:3,1\nend_of_record\n";
    let outcome = collected(&p, "javascript-coverage", &machine(writing(lcov)));
    let coverage = facts(&outcome, "javascript-coverage", "coverage");
    assert_eq!(ids(&coverage), ["cov:js:@acme/core/src/index.ts::add"]);
    assert_eq!(coverage[0].fields["lines_hit"], 2);
}

/// ESLint's SARIF: one `no-unused-vars` result at line 2 of `view.tsx`.
const SARIF: &str = r#"{
  "version": "2.1.0",
  "runs": [{
    "tool": {"driver": {"name": "ESLint", "version": "10.15.0", "rules": [{"id": "no-unused-vars"}]}},
    "results": [{
      "level": "error",
      "message": {"text": "'title' is defined but never used."},
      "ruleId": "no-unused-vars",
      "locations": [{"physicalLocation": {
        "artifactLocation": {"uri": "packages/core/src/view.tsx"},
        "region": {"startLine": 2, "startColumn": 3, "endLine": 2, "endColumn": 9}
      }}]
    }]
  }]
}"#;

#[specforge_test_macros::test(requirement = "FEATURE-145")]
fn javascript_lint_records_a_finding_at_its_item() {
    let p = fixture();
    let outcome = collected(&p, "javascript-lint", &machine(writing(SARIF)));
    let findings = facts(&outcome, "javascript-lint", "static_finding");
    assert_eq!(findings.len(), 1, "{:?}", ids(&findings));
    assert_eq!(
        findings[0].fields["item"],
        "js:@acme/core/src/view.tsx::View"
    );
}

/// cyclonedx-npm's BOM of the root package `app`: its one dependency, lodash.
const BOM: &str = r#"{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 1,
  "metadata": {"component": {"type": "application", "name": "app", "version": "0.0.0",
    "bom-ref": "app@0.0.0", "purl": "pkg:npm/app@0.0.0"}},
  "components": [{"type": "library", "name": "lodash", "version": "4.17.15",
    "bom-ref": "lodash@4.17.15", "purl": "pkg:npm/lodash@4.17.15"}],
  "dependencies": [{"ref": "app@0.0.0", "dependsOn": ["lodash@4.17.15"]}, {"ref": "lodash@4.17.15"}]
}"#;

/// osv-scanner's report: one advisory of lodash in the npm lockfile, and a PyPI package of another lockfile, an
/// ecosystem no pack collects.
const OSV: &str = r#"{
  "results": [
    {
      "source": {"path": "package-lock.json", "type": "lockfile"},
      "packages": [{
        "package": {"name": "lodash", "version": "4.17.15", "ecosystem": "npm"},
        "vulnerabilities": [{
          "id": "GHSA-35jh-r3h4-6jhm",
          "aliases": ["CVE-2021-23337"],
          "summary": "Command Injection in lodash",
          "affected": [{"package": {"ecosystem": "npm", "name": "lodash", "purl": "pkg:npm/lodash"}}],
          "database_specific": {"severity": "HIGH"}
        }],
        "groups": [{"ids": ["GHSA-35jh-r3h4-6jhm"]}]
      }]
    },
    {
      "source": {"path": "tools/requirements.txt", "type": "lockfile"},
      "packages": [{
        "package": {"name": "requests", "version": "2.0.0", "ecosystem": "PyPI"},
        "vulnerabilities": [{"id": "PYSEC-2099-1", "summary": "An invented advisory"}],
        "groups": [{"ids": ["PYSEC-2099-1"]}]
      }]
    }
  ]
}"#;

#[specforge_test_macros::test(requirement = "FEATURE-147")]
#[specforge_test_macros::test(requirement = "FEATURE-128")]
fn javascript_deps_records_dependencies_and_vulnerabilities_and_warns_of_a_skipped_entry() {
    let p = fixture();
    let sbom = collected(&p, "javascript-sbom", &machine(writing(BOM)));
    assert_eq!(
        ids(&facts(&sbom, "javascript-sbom", "dependency")),
        ["dep:pkg:npm/app/pkg%3Anpm%2Flodash%404.17.15"]
    );

    let osv = collected(&p, "javascript-osv", &machine(writing(OSV)));
    let vulnerabilities = facts(&osv, "javascript-osv", "vulnerability");
    assert_eq!(vulnerabilities.len(), 1, "{:?}", ids(&vulnerabilities));
    assert_eq!(vulnerabilities[0].fields["package"], "pkg:npm/app");
    assert_eq!(vulnerabilities[0].fields["osv_id"], "GHSA-35jh-r3h4-6jhm");
    let warned = said(&osv, "W203");
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert!(
        warned[0].contains("'javascript-osv'")
            && warned[0].contains("'requests' 2.0.0 in 'tools/requirements.txt'"),
        "{warned:?}"
    );
}

/// api-extractor's doc model of `@acme/core`: its function `add`.
const DOC_MODEL: &str = r#"{
  "metadata": {"toolPackage": "@microsoft/api-extractor", "toolVersion": "7.60.1", "schemaVersion": 1011},
  "kind": "Package",
  "canonicalReference": "@acme/core!",
  "name": "@acme/core",
  "members": [{
    "kind": "EntryPoint",
    "canonicalReference": "@acme/core!",
    "name": "",
    "members": [{
      "kind": "Function",
      "canonicalReference": "@acme/core!add:function(1)",
      "excerptTokens": [{"kind": "Content", "text": "export declare function add(a: number, b: number): number;"}],
      "fileUrlPath": "dist/index.d.ts",
      "releaseTag": "Public",
      "name": "add"
    }]
  }]
}"#;

#[specforge_test_macros::test(requirement = "FEATURE-149")]
fn javascript_api_writes_its_config_and_records_the_doc_model() {
    let p = fixture();
    let outcome = collected(&p, "javascript-api", &machine(writing(DOC_MODEL)));
    let config = p.read(".specforge/reports/javascript-api.api-extractor.json");
    assert!(
        config.contains(r#""apiJsonFilePath": ""#)
            && config.contains("javascript-api.api.json")
            && !config.contains("{report}"),
        "{config}"
    );
    let api = facts(&outcome, "javascript-api", "api_item");
    assert_eq!(ids(&api), ["api:js:%40acme%2Fcore::add"]);
    assert_eq!(api[0].fields["item"], "js:@acme/core/src/index.ts::add");
}

/// The `test` facts `collector` answered: (id, status).
fn tests_of(outcome: &Outcome, collector: &str) -> Vec<(String, String)> {
    facts(outcome, collector, "test")
        .iter()
        .map(|f| {
            (
                f.id.clone(),
                f.fields["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// Two cases of `board`, `adds` passed and `fails` failed, each test id under the workspace's test file.
fn assert_two_cases(outcome: &Outcome, collector: &str) {
    let tests = tests_of(outcome, collector);
    assert_eq!(tests.len(), 2, "{tests:?}");
    for (id, _) in &tests {
        assert!(
            id.starts_with("js:@acme/core/test/board.test.ts#"),
            "{tests:?}"
        );
    }
    let status = |name: &str| {
        tests
            .iter()
            .find(|(id, _)| id.ends_with(name))
            .map(|(_, status)| status.as_str())
    };
    assert_eq!(status("adds"), Some("passed"), "{tests:?}");
    assert_eq!(status("fails"), Some("failed"), "{tests:?}");
}

#[specforge_test_macros::test(requirement = "FEATURE-151")]
fn javascript_vitest_records_its_junit_cases() {
    let p = fixture();
    let junit = format!(
        r#"<?xml version="1.0" encoding="UTF-8" ?>
<testsuites name="vitest tests" tests="2" failures="1" errors="0" time="0.003">
    <testsuite name="{TEST_FILE}" tests="2" failures="1" errors="0" skipped="0" time="0.003">
        <testcase classname="{TEST_FILE}" name="board &gt; adds" time="0.001">
        </testcase>
        <testcase classname="{TEST_FILE}" name="board &gt; fails" time="0.002">
            <failure message="expected 1 to be 2" type="AssertionError">AssertionError: expected 1 to be 2</failure>
        </testcase>
    </testsuite>
</testsuites>
"#
    );
    let outcome = collected(&p, "javascript-vitest", &machine(writing(&junit)));
    assert_two_cases(&outcome, "javascript-vitest");
}

#[specforge_test_macros::test(requirement = "FEATURE-152")]
fn javascript_jest_writes_its_env_file_and_records_its_junit_cases() {
    let p = fixture();
    let junit = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="jest tests" tests="2" failures="1" errors="0" time="0.1">
  <testsuite name="{TEST_FILE}" errors="0" failures="1" skipped="0" time="0.05" tests="2">
    <testcase classname="board" name="adds" time="0.001" file="{TEST_FILE}">
    </testcase>
    <testcase classname="board" name="fails" time="0.001" file="{TEST_FILE}">
      <failure>Error: expect(received).toBe(expected)</failure>
    </testcase>
  </testsuite>
</testsuites>
"#
    );
    let outcome = collected(&p, "javascript-jest", &machine(writing(&junit)));
    let env = p.read(".specforge/reports/javascript-jest.jest-junit.env.toml");
    assert!(
        env.contains("JEST_JUNIT_OUTPUT_FILE = \"")
            && env.contains("javascript-jest.xml")
            && env.contains("JEST_JUNIT_ADD_FILE_ATTRIBUTE = \"true\""),
        "{env}"
    );
    assert_two_cases(&outcome, "javascript-jest");
}

#[specforge_test_macros::test(requirement = "FEATURE-153")]
fn javascript_playwright_records_its_captured_junit_cases() {
    let p = fixture();
    let junit = format!(
        "<testsuites id=\"\" name=\"\" tests=\"2\" failures=\"1\" skipped=\"0\" errors=\"0\" time=\"0.5\">\n\
<testsuite name=\"{TEST_FILE}\" tests=\"2\" failures=\"1\" skipped=\"0\" time=\"0.005\" errors=\"0\">\n\
<testcase name=\"board \u{203a} adds\" classname=\"{TEST_FILE}\" time=\"0.002\">\n</testcase>\n\
<testcase name=\"board \u{203a} fails\" classname=\"{TEST_FILE}\" time=\"0.001\">\n\
<failure message=\"expect(received).toBe(expected)\" type=\"expect.toBe\">Error</failure>\n</testcase>\n\
</testsuite>\n</testsuites>\n"
    );
    let run = ScriptedRun {
        stdout: Some(junit),
        ..ScriptedRun::default()
    };
    let outcome = collected(&p, "javascript-playwright", &machine(run));
    assert_two_cases(&outcome, "javascript-playwright");
}
