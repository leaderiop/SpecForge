//! @rust/test — runs the project's tests with cargo-nextest and records them as `@code/core`'s test facts
//! (ADR 0348 D8).
//!
//! The collector `rust-test` is selected by a `Cargo.toml` at the project root. The host runs, with consent,
//! `cargo-nextest nextest run` under the profile `specforge`, which the config file it writes for the run
//! (`.specforge/reports/rust-test.nextest.toml`, layered under the project's own config as a tool config) defines:
//! nextest writes JUnit only through a config's `[profile.<name>.junit] path`. The program is `cargo-nextest`, so a
//! missing nextest is "not found on PATH" with its install line, never a failed build.
//!
//! Each case sits in exactly one `<testsuite>`, named by its binary id (`toy`, `toy::bin/toy`, `toy::api`): the test
//! is `rust:<binary id>#<name>`. It is served `@rust/source`'s items; a test whose id an item's `test_id` holds takes
//! the item's places, file and names. nextest omits an ignored test from its report, so every item marked `test` and
//! `ignore` whose id the report lacks becomes a `skipped` test without a run: a cited ignored test is held.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use specforge_code_facts::{test_facts, Names, TestCase, TestId};
use specforge_code_junit::cases;
use specforge_extension_sdk::prelude::*;

const COLLECTOR: &str = "rust-test";
/// The JUnit report nextest writes, relative to the project root.
const REPORT: &str = ".specforge/reports/rust-test.xml";
/// The config file the host writes before the run, `{report}` the report's absolute path (ADR 0348 D2).
const CONFIG: &str = "nextest.toml";
const TEMPLATE: &str = r#"[profile.specforge]
fail-fast = false

[profile.specforge.junit]
path = "{report}"
store-success-output = false
store-failure-output = false
"#;

#[specforge_extension_sdk::extension(
    name = "@rust/test",
    version = "1.0.0",
    description = "Runs cargo-nextest and records its tests as test facts (@code/core)"
)]
struct RustTest;

impl Contributions for RustTest {
    fn contribute(c: &mut ContributionsBuilder) {
        c.peer("@code/core", "^1.1");
        // Without its items an ignored test, which nextest does not report, would be a false code/E101.
        c.peer("@rust/source", "^1.0");
        c.collector(COLLECTOR, |k| {
            k.input_format("junit-xml")
                .detect_files(&["Cargo.toml"])
                .run(&[
                    "cargo-nextest",
                    "nextest",
                    "run",
                    "--workspace",
                    "--no-tests=warn",
                    "--tool-config-file",
                    "specforge:{config}",
                    "--profile",
                    "specforge",
                ])
                .config(CONFIG, TEMPLATE)
                .install("cargo install cargo-nextest --locked")
                .report(REPORT)
                .input(Input::facts(&["item"]))
                .answers(&["test", "test_run"])
                .collect(collect);
        });
    }
}

/// An item `@rust/source` answered for a test function: where it is written and what it names.
struct Item<'f> {
    fact: &'f Fact,
    ignored: bool,
}

impl Item<'_> {
    /// `case` placed at this item: its places, file and names.
    fn place(&self, case: TestCase) -> TestCase {
        let names: Names = self
            .fact
            .fields
            .get("names")
            .and_then(|names| names.as_array())
            .into_iter()
            .flatten()
            .filter_map(|name| name.as_str())
            .map(|name| {
                let places = self
                    .fact
                    .named_at
                    .get("names")
                    .and_then(|named| named.get(name))
                    .cloned()
                    .unwrap_or_default();
                (name.to_string(), places)
            })
            .collect();
        TestCase {
            file: case.file.or_else(|| {
                self.fact
                    .fields
                    .get("file")
                    .and_then(|f| f.as_str())
                    .map(str::to_string)
            }),
            names,
            at: self.fact.at.clone(),
            ..case
        }
    }
}

/// The served items by the test id they hold.
fn items(facts: &[Fact]) -> BTreeMap<&str, Item<'_>> {
    facts
        .iter()
        .filter(|fact| fact.kind == "item")
        .filter_map(|fact| {
            let test_id = fact.fields.get("test_id")?.as_str()?;
            let attrs: Vec<&str> = fact
                .fields
                .get("attrs")
                .and_then(|a| a.as_array())
                .into_iter()
                .flatten()
                .filter_map(|a| a.as_str())
                .collect();
            let ignored = attrs.contains(&"test") && attrs.contains(&"ignore");
            Some((test_id, Item { fact, ignored }))
        })
        .collect()
}

/// Map nextest's JUnit report to a `test` and a `test_run` fact per case, and a `skipped` test per ignored test item
/// the report lacks. A report whose case is not in exactly one `<testsuite>` is not nextest's: the collector's error.
fn collect(input: &CollectInput) -> Result<CollectOutput, String> {
    let [report] = &input.reports[..] else {
        return Err(format!(
            "rust-test reads one report, {REPORT}; it was given {}",
            input.reports.len()
        ));
    };
    let found = cases(&report.content).map_err(|why| format!("{}: {why}", report.path))?;
    let items = items(&input.facts);
    let mut reported = BTreeSet::new();
    let mut tests = Vec::new();
    for case in &found {
        let [binary] = &case.suites[..] else {
            return Err(format!(
                "{}: the case '{}' sits in {} <testsuite>s; nextest writes each case in exactly one, named by its \
                 binary id, so this is not a nextest JUnit report",
                report.path,
                case.name,
                case.suites.len()
            ));
        };
        let id = TestId::rust(binary, &case.name);
        let key = id.to_string();
        let test = TestCase::of(id, case, None);
        tests.push(match items.get(key.as_str()) {
            Some(item) => item.place(test),
            None => test,
        });
        reported.insert(key);
    }
    for (test_id, item) in &items {
        if item.ignored && !reported.contains(*test_id) {
            let id = TestId::from_str(test_id).map_err(|e| format!("{}: {e}", item.fact.id))?;
            tests.push(item.place(TestCase::unrun(id, TestStatus::Skipped)));
        }
    }
    Ok(CollectOutput {
        facts: test_facts(tests),
        ..CollectOutput::default()
    })
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);

#[cfg(test)]
mod tests;
