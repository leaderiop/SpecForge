//! @javascript/vitest — runs vitest and records its tests as `@code/core`'s test facts (ADR 0389).
//!
//! The collector `javascript-vitest` is selected by a `vitest.config.*` or `vitest.workspace.*` at the project root.
//! The host runs, with consent, `npx --no -- vitest run` with two reporters: `default` to standard output, so the
//! person sees vitest's usual output, and `junit` to `.specforge/reports/javascript-vitest.xml`, the report this pack
//! reads. `npx --no` never downloads anything: vitest must be a dev dependency of the project.
//!
//! vitest writes one `<testsuite>` per file, named by the file, and each case's `classname` is that project-relative
//! file; its `name` is the describe blocks then the test title joined by `" > "`. Each test is named
//! `js:<package>/<file>#<describe > … > test>`, as `@javascript/node-test` names it (ADR 0329 D2): `<package>` is the
//! nearest named `package.json` at or above the file (`specforge-javascript-workspace`), `<file>` the path under it.
//!
//! A caller narrows the run with a filter: the host appends `-t <TEXT>`, and vitest matches the text against test
//! names, not SpecForge ids (ADR 0389 D9).
//!
//! The legacy `@specforge/vitest` stays a separate pack with the collector `vitest`, reading vitest's JSON report for
//! entity results (owner decision D4); this pack's collector is `javascript-vitest`, so the two never clash.

use specforge_code_facts::{relative, test_facts, TestCase, TestId};
use specforge_code_junit::{cases, Case};
use specforge_extension_sdk::prelude::*;
use specforge_javascript_workspace::{packages_input, Workspace};

const COLLECTOR: &str = "javascript-vitest";
/// The JUnit report vitest writes, relative to the project root.
const REPORT: &str = ".specforge/reports/javascript-vitest.xml";

#[specforge_extension_sdk::extension(
    name = "@javascript/vitest",
    version = "1.0.0",
    description = "Runs vitest and records its tests as test facts (@code/core)"
)]
struct JavascriptVitest;

impl Contributions for JavascriptVitest {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its package.json reads name the packages a test id starts with.
        c.peer("@javascript/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("junit-xml")
                .detect_files(&["vitest.config.*", "vitest.workspace.*"])
                .run(&[
                    "npx",
                    "--no",
                    "--",
                    "vitest",
                    "run",
                    "--reporter=default",
                    "--reporter=junit",
                    "--outputFile.junit={report}",
                ])
                .filter(&["-t", "{filter}"])
                .env(&["NODE_*", "VITEST_*", "CI"])
                .install("npm install --save-dev vitest")
                .report(REPORT)
                .input(packages_input())
                .answers(&["test", "test_run"])
                .collect(collect);
        });
    }
}

/// The id of `case` and its project-relative file, the file being its `classname` read against `cwd`. The path is
/// `name` split on `" > "`: vitest names the suite after the file, so `Case::path` would repeat it.
fn id(workspace: &Workspace, cwd: &str, case: &Case) -> Result<(TestId, String), String> {
    if case.classname.is_empty() {
        return Err(format!(
            "vitest's JUnit reporter wrote no classname (the test file) for '{}'",
            case.name
        ));
    }
    let file = relative(&case.classname, cwd).ok_or_else(|| {
        format!(
            "{}: the test file is outside the project {cwd}",
            case.classname
        )
    })?;
    let path: Vec<String> = case.name.split(" > ").map(str::to_string).collect();
    let (package, under) = workspace.owner(&file).ok_or_else(|| {
        format!(
            "{file}: no package.json with a name at or above it; {COLLECTOR} names a test \
             js:<package name>/<file>#<test>, so give the package a name"
        )
    })?;
    let id = TestId::js(&package.segment(), under, &path);
    Ok((id, file))
}

/// Map vitest's JUnit report to a `test` and a `test_run` fact per test. A report that is not JUnit, a file outside
/// the project and a test with no named package are the collector's error: a test is never dropped silently.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "{COLLECTOR} reads one report, {REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let found = cases(&report.content).map_err(|why| format!("{}: {why}", report.path))?;
    let cwd = input
        .cwd
        .as_deref()
        .ok_or("the host sent no cwd: a vitest test file is read against the project root")?;
    let workspace = Workspace::of(input);
    let named = found
        .into_iter()
        .map(|case| {
            let (id, file) = id(&workspace, cwd, &case)?;
            Ok(TestCase::of(id, &case, Some(file)))
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
