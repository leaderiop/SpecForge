//! @javascript/playwright — runs Playwright and records its tests as `@code/core`'s test facts (ADR 0389).
//!
//! The collector `javascript-playwright` is selected by a `playwright.config.*` at the project root. The host runs,
//! with consent, `npx --no -- playwright test --reporter=junit`, its artifacts kept under
//! `.specforge/reports/javascript-playwright-results` instead of the project's `test-results`. Playwright writes its
//! JUnit to standard output unless told a file, so the host captures it into
//! `.specforge/reports/javascript-playwright/javascript-playwright.stdout.txt` and passes it as
//! `CollectInput::stdout`; `specforge collect --report <file>` passes a saved report as the one report instead. The
//! command runs in the caller's environment less credentials (ADR 0360 D13): its `env` list is what a guest-run tool
//! would be lent, not a filter, so every `PLAYWRIGHT_*` variable the caller set reaches the run.
//! `PLAYWRIGHT_JUNIT_OUTPUT_NAME` and its kin move the report off standard output, where the collect reads it; unset
//! them before `specforge collect`. `npx --no` never downloads anything: `@playwright/test`
//! must be a dev dependency of the project, its browsers installed.
//!
//! Each case's `classname` is its file and its `name` the describe blocks then the test title joined by `" › "`
//! (U+203A). Each test is named `js:<package>/<file>#<describe > … > test>`, as `@javascript/node-test` names it (ADR
//! 0329 D2), `<package>` the nearest named `package.json` at or above the file. Playwright's file names are relative
//! to the config's `testDir`, not to the project: a test in `tests/a.spec.ts` with `testDir: "tests"` is named
//! `a.spec.ts`, and its package is the one that owns `a.spec.ts` read from the project root (ADR 0389 D9, accepted).
//!
//! A caller narrows the run with a filter: the host appends `--grep <TEXT>`, and Playwright matches the text against
//! test titles, not SpecForge ids (ADR 0389 D9).

use specforge_code_facts::{relative, test_facts, TestCase, TestId};
use specforge_code_junit::{cases, Case};
use specforge_extension_sdk::prelude::*;
use specforge_javascript_workspace::{packages_input, Workspace};

const COLLECTOR: &str = "javascript-playwright";
/// The report directory, relative to the project root: the host keeps the captured standard output in it.
const REPORT: &str = ".specforge/reports/javascript-playwright";
/// Where the host keeps the captured standard output: `<report dir>/<collector>.stdout.txt`.
const CAPTURE: &str = ".specforge/reports/javascript-playwright/javascript-playwright.stdout.txt";

#[specforge_extension_sdk::extension(
    name = "@javascript/playwright",
    version = "1.0.0",
    description = "Runs Playwright and records its tests as test facts (@code/core)"
)]
struct JavascriptPlaywright;

impl Contributions for JavascriptPlaywright {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its package.json reads name the packages a test id starts with.
        c.peer("@javascript/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("junit-xml")
                .detect_files(&["playwright.config.*"])
                .run(&[
                    "npx",
                    "--no",
                    "--",
                    "playwright",
                    "test",
                    "--reporter=junit",
                    "--output=.specforge/reports/javascript-playwright-results",
                ])
                .filter(&["--grep", "{filter}"])
                .env(&["NODE_*", "PLAYWRIGHT_BROWSERS_PATH", "CI"])
                .install("npm install --save-dev @playwright/test")
                .report(REPORT)
                .capture_stdout()
                .input(packages_input())
                .answers(&["test", "test_run"])
                .collect(collect);
        });
    }
}

/// The id of `case` and its file, the file being its `classname` (relative to Playwright's `testDir`). The path is
/// `name` split on `" › "`.
fn id(workspace: &Workspace, cwd: &str, case: &Case) -> Result<(TestId, String), String> {
    if case.classname.is_empty() {
        return Err(format!(
            "Playwright's JUnit reporter wrote no classname (the test file) for '{}'",
            case.name
        ));
    }
    let file = relative(&case.classname, cwd).ok_or_else(|| {
        format!(
            "{}: the test file is outside the project {cwd}",
            case.classname
        )
    })?;
    let path: Vec<String> = case.name.split(" \u{203a} ").map(str::to_string).collect();
    let (package, under) = workspace.owner(&file).ok_or_else(|| {
        format!(
            "{file}: no package.json with a name at or above it; {COLLECTOR} names a test \
             js:<package name>/<file>#<test>, so give the package a name"
        )
    })?;
    let id = TestId::js(&package.segment(), under, &path);
    Ok((id, file))
}

/// Map Playwright's captured JUnit (else the one report given) to a `test` and a `test_run` fact per test. A text
/// that is not JUnit, a file outside the project and a test with no named package are the collector's error: a test
/// is never dropped silently.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let (path, text) = match (&input.stdout, &input.reports[..]) {
        (Some(stdout), _) => (CAPTURE, stdout.as_str()),
        (None, [report]) => (report.path.as_str(), report.content.as_str()),
        (None, reports) => {
            return Err(format!(
                "{COLLECTOR}: Playwright's standard output was not captured, and it reads one report in its \
                 place; it was given {}",
                reports.len()
            ))
        }
    };
    let found = cases(text).map_err(|why| format!("{path}: {why}"))?;
    let cwd = input
        .cwd
        .as_deref()
        .ok_or("the host sent no cwd: a Playwright test file is read against the project root")?;
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
