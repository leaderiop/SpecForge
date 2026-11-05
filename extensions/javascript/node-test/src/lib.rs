//! @javascript/node-test — runs the project's own `node --test` and records its tests as `@code/core`'s test facts
//! (ADR 0327, ADR 0329).
//!
//! The collector `node-test` is selected by a `package.json` at the project root. The host runs, with consent,
//! `node --test` with two reporters: `spec` to standard output, so the person sees node's usual output, and `junit` to
//! `.specforge/reports/node-test.xml`, the report this pack reads. Node needs 20.10 or later for the JUnit reporter.
//!
//! Each test is named `js:<package>/<file>#<suite > … > name>` (ADR 0329 D2): `<package>` is the `name` of the
//! nearest `package.json` at or above the test's file, read through the collector's `reads` (a guest has no file
//! system, ADR 0293), and `<file>` is the path under that package's directory. The answer is facts only, a `test` and
//! a `test_run` per test, minted by `specforge-code-junit` so every runner pack names a test alike: an entity cites a
//! test by its id, and `@code/core` checks the reference.

use specforge_code_junit::{cases, facts, relative, Case, TestId};
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "node-test";
/// The JUnit report node writes, relative to the project root: a file, since a report directory is read for its
/// `*.json` only.
const REPORT: &str = ".specforge/reports/node-test.xml";
/// The read holding every `package.json` with its text.
const PACKAGES: &str = "packages";

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
                .input(Input::files(
                    PACKAGES,
                    FileReadSpec::glob("**/package.json").text(),
                ))
                .collect(collect);
        });
    }
}

/// A package of the project: the directory of a read `package.json` (`""` at the root) and the `name` it gives.
struct Package {
    dir: String,
    name: String,
}

impl Package {
    /// The path of the project-relative `file` under this package's directory, when the file is in it.
    fn holds<'f>(&self, file: &'f str) -> Option<&'f str> {
        if self.dir.is_empty() {
            Some(file)
        } else {
            file.strip_prefix(self.dir.as_str())?.strip_prefix('/')
        }
    }
}

/// The packages the host read for `input`: each `package.json` with a string `name`. A file that could not be read,
/// or whose text is not JSON naming the package, is no package.
fn packages(input: &CollectInput) -> Vec<Package> {
    input
        .files
        .get(PACKAGES)
        .into_iter()
        .flat_map(|read| &read.facts)
        .filter(|file| file.error.is_none())
        .filter_map(|file| {
            let json: serde_json::Value = serde_json::from_str(file.text.as_deref()?).ok()?;
            let name = json.get("name")?.as_str()?.to_string();
            let dir = file
                .path
                .strip_suffix("package.json")?
                .trim_end_matches('/');
            Some(Package {
                dir: dir.to_string(),
                name,
            })
        })
        .collect()
}

/// The id of `case`, whose project-relative file is `file`: named after the nearest of `packages` at or above it.
fn id(packages: &[Package], file: &str, case: &Case) -> Result<TestId, String> {
    packages
        .iter()
        .filter_map(|package| Some((package, package.holds(file)?)))
        .max_by_key(|(package, _)| package.dir.len())
        .map(|(package, under)| TestId::js(&package.name, under, case))
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
    let packages = packages(input);
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
            Ok((id(&packages, &file, &case)?, case))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CollectOutput {
        facts: facts(named),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
