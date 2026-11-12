//! @javascript/node-test — runs the project's own `node --test` and records its tests as `@code/core`'s test facts
//! (ADR 0327, ADR 0329).
//!
//! The collector `node-test` is selected by a `package.json` at the project root. The host runs, with consent,
//! `node --test` with two reporters: `spec` to standard output, so the person sees node's usual output, and `junit` to
//! `.specforge/reports/node-test.xml`, the report this pack reads. Node needs 20.10 or later for the JUnit reporter.
//!
//! Each test is named `js:<package>/<file>#<suite > … > name>` (ADR 0329 D2): `<package>` is the `name` of the
//! nearest `package.json` at or above the test's file (`<name>@<dir>` for a second package of that name;
//! `specforge-javascript-workspace`, which `@javascript/source` names its items by too), and `<file>` is the path
//! under that package's directory. The answer is facts only, a
//! `test` and a `test_run` per test, read by `specforge-code-junit` and minted by `specforge-code-facts` so every
//! runner pack names a test alike: an entity cites a test by its id, and `@code/core` checks the reference. A test
//! fact's `file` is the project-relative path (ADR 0347 D8).

use specforge_code_facts::{test_facts, TestCase, TestId};
use specforge_code_junit::{cases, relative, Case};
use specforge_extension_sdk::prelude::*;
use specforge_javascript_workspace::{packages_input, Workspace};

const COLLECTOR: &str = "node-test";
/// The JUnit report node writes, relative to the project root: a file, since a report directory is read for its
/// `*.json` only.
const REPORT: &str = ".specforge/reports/node-test.xml";

#[specforge_extension_sdk::extension(
    name = "@javascript/node-test",
    version = "1.0.0",
    description = "Runs node --test and records its tests as test facts (@code/core)"
)]
struct NodeTest;

impl Contributions for NodeTest {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("junit-xml")
                .detect_files(&["package.json"])
                // `node` itself, not `npm test`: the project's script may set reporters of its own.
                .run(&[
                    "node",
                    "--test",
                    "--test-reporter=spec",
                    "--test-reporter-destination=stdout",
                    "--test-reporter=junit",
                    "--test-reporter-destination={report}",
                ])
                .report(REPORT)
                .answers(&["test", "test_run"])
                .input(packages_input())
                .collect(collect);
        });
    }
}

/// The id of `case`, whose project-relative file is `file`: named after the package of `workspace` that owns it.
fn id(workspace: &Workspace, file: &str, case: &Case) -> Result<TestId, String> {
    workspace
        .owner(file)
        .map(|(package, under)| TestId::js(&package.segment(), under, &case.path()))
        .ok_or_else(|| {
            format!(
                "{file}: no package.json with a name at or above it; node-test names a test \
                 js:<package name>/<file>#<test>, so give the package a name"
            )
        })
}

/// Map node's JUnit report to a `test` and a `test_run` fact per test. A report that is not JUnit, a test with no
/// file, a file outside the project and a test with no named package are the collector's error: a test is never
/// dropped silently.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "node-test reads one report, {REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let found = cases(&report.content).map_err(|why| format!("{}: {why}", report.path))?;
    let cwd = input.cwd.as_deref().ok_or(
        "the host sent no cwd: node writes absolute test files, read against the project root",
    )?;
    let workspace = Workspace::of(input);
    let named = found
        .into_iter()
        .map(|case| {
            let written = case.file.as_deref().ok_or_else(|| {
                format!(
                    "node's JUnit reporter wrote no file for '{}'; node 20.10 and later do",
                    case.name
                )
            })?;
            let file = relative(written, cwd)
                .ok_or_else(|| format!("{written}: the test file is outside the project {cwd}"))?;
            Ok(TestCase::of(
                id(&workspace, &file, &case)?,
                &case,
                Some(file),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CollectOutput {
        facts: test_facts(named),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
