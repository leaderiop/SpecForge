//! @javascript/jest — runs jest with jest-junit and records its tests as `@code/core`'s test facts (ADR 0389).
//!
//! The collector `javascript-jest` is selected by a `jest.config.*` at the project root. jest writes no JUnit of its
//! own and jest-junit is configured only by a `package.json` key or `JEST_JUNIT_*` variables, and the host runs one
//! argv with no variables of its choosing. So the pack owns a config file the host writes for the run
//! (`.specforge/reports/javascript-jest.jest-junit.env.toml`), and the host runs, with consent, `node
//! --env-file={config} node_modules/jest/bin/jest.js --reporters=default --reporters=jest-junit`: node loads the file
//! into jest's environment. Its six lines are both TOML and dotenv. The host substitutes only `{report}` (escaped for
//! a quoted string, ADR 0348 D2), so `{filepath}`, `{classname}` and `{title}` reach jest-junit as its own
//! placeholders. `JEST_*` is not in the environment the run inherits: a variable already set there would override the
//! file's.
//!
//! Prerequisites: jest and jest-junit are dev dependencies of the project (a missing jest-junit shows only at run
//! time); `--env-file` needs node 20.6 or later. The route is unverified on Windows (ADR 0389 D12).
//!
//! jest-junit writes each case's project-relative `file` (the config turns the attribute on), its `classname` the
//! describe blocks joined by `" > "` and its `name` the test title. Each test is named
//! `js:<package>/<file>#<describe > … > test>`, as `@javascript/node-test` names it (ADR 0329 D2): `<package>` is the
//! nearest named `package.json` at or above the file (`specforge-javascript-workspace`).
//!
//! A caller narrows the run with a filter: the host appends `-t <TEXT>`, and jest matches the text against test
//! names, not SpecForge ids (ADR 0389 D9).

use specforge_code_facts::{relative, test_facts, TestCase, TestId};
use specforge_code_junit::{cases, Case};
use specforge_extension_sdk::prelude::*;
use specforge_javascript_workspace::{packages_input, Workspace};

const COLLECTOR: &str = "javascript-jest";
/// The JUnit report jest-junit writes, relative to the project root.
const REPORT: &str = ".specforge/reports/javascript-jest.xml";
/// The env file the host writes before the run, `{report}` the report's absolute path (ADR 0348 D2). A config name
/// ends `.toml` or `.json`; node's `--env-file` reads any name.
const CONFIG: &str = "jest-junit.env.toml";
const TEMPLATE: &str = r#"JEST_JUNIT_OUTPUT_FILE = "{report}"
JEST_JUNIT_ADD_FILE_ATTRIBUTE = "true"
JEST_JUNIT_ANCESTOR_SEPARATOR = " > "
JEST_JUNIT_SUITE_NAME = "{filepath}"
JEST_JUNIT_CLASSNAME = "{classname}"
JEST_JUNIT_TITLE = "{title}"
"#;

#[specforge_extension_sdk::extension(
    name = "@javascript/jest",
    version = "1.0.0",
    description = "Runs jest with jest-junit and records its tests as test facts (@code/core)"
)]
struct JavascriptJest;

impl Contributions for JavascriptJest {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.2");
        // Its package.json reads name the packages a test id starts with.
        c.peer("@javascript/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("junit-xml")
                .detect_files(&["jest.config.*"])
                .run(&[
                    "node",
                    "--env-file={config}",
                    "node_modules/jest/bin/jest.js",
                    "--reporters=default",
                    "--reporters=jest-junit",
                ])
                .config(CONFIG, TEMPLATE)
                .filter(&["-t", "{filter}"])
                .env(&["NODE_*", "CI"])
                .install("npm install --save-dev jest jest-junit")
                .report(REPORT)
                .input(packages_input())
                .answers(&["test", "test_run"])
                .collect(collect);
        });
    }
}

/// The id of `case` and its project-relative file, the file being its `file` attribute read against `cwd`. The path
/// is `classname` split on `" > "` (nothing for a top-level test), then `name`.
fn id(workspace: &Workspace, cwd: &str, case: &Case) -> Result<(TestId, String), String> {
    let written = case.file.as_deref().ok_or_else(|| {
        format!(
            "jest-junit wrote no file for '{}': its file attribute is off (the pack's config turns \
             JEST_JUNIT_ADD_FILE_ATTRIBUTE on)",
            case.name
        )
    })?;
    let file = relative(written, cwd)
        .ok_or_else(|| format!("{written}: the test file is outside the project {cwd}"))?;
    let path: Vec<String> = case
        .classname
        .split(" > ")
        .filter(|_| !case.classname.is_empty())
        .chain([case.name.as_str()])
        .map(str::to_string)
        .collect();
    let (package, under) = workspace.owner(&file).ok_or_else(|| {
        format!(
            "{file}: no package.json with a name at or above it; {COLLECTOR} names a test \
             js:<package name>/<file>#<test>, so give the package a name"
        )
    })?;
    let id = TestId::js(&package.segment(), under, &path);
    Ok((id, file))
}

/// Map jest-junit's report to a `test` and a `test_run` fact per test. A report that is not JUnit, a case with no
/// file, a file outside the project and a test with no named package are the collector's error: a test is never
/// dropped silently.
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
        .ok_or("the host sent no cwd: a jest test file is read against the project root")?;
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
