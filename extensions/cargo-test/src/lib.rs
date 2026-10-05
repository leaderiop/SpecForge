//! @specforge/cargo-test — collects `cargo test` results (ADR 0002).
//!
//! Tests link themselves to entities with the `#[specforge_test(...)]`
//! attribute from the `specforge-test` crate, which records every annotated
//! test and writes one JSON report per test binary. This extension declares
//! the command `specforge collect` runs (`cargo test --workspace --no-fail-fast`, so one failing test
//! binary doesn't stop the others from reporting) and maps
//! those per-binary reports to entity results. It runs nothing itself: the
//! host runs the command with the user's consent and passes the report
//! files to `collect__cargo_test`.
//!
//! Plain `#[test]` functions leave no report; they only appear in libtest's
//! output. The collector captures that output and returns every test the
//! attribute didn't record as *unlinked*, for the host to link by naming
//! convention (`add_item__rejects_a_duplicate_item`, `mod add_item`).

use specforge_extension_sdk::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

const COLLECTOR: &str = "cargo-test";

#[specforge_extension_sdk::extension(
    name = "@specforge/cargo-test",
    version = "1.0.0",
    description = "Collects cargo test results for tests linked to entities with #[specforge_test]"
)]
struct CargoTest;

impl Contributions for CargoTest {
    fn contribute(c: &mut ContributionsBuilder) {
        c.meta.peer_dependencies.push(PeerDependency {
            name: "@specforge/testing".to_string(),
            version: "^1.0".to_string(),
            optional: false,
        });
        c.collector(COLLECTOR, |k| {
            k.input_format("specforge-test-json")
                .input_format("libtest-stdout")
                .detect_files(&["Cargo.toml"])
                .run(&["cargo", "test", "--workspace", "--no-fail-fast"])
                .report("target/specforge")
                .capture_stdout();
        });
    }
}

/// One binary's report, as `specforge-test` writes it.
#[derive(serde::Deserialize)]
struct BinaryReport {
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    entity_id: String,
    test_name: String,
    #[serde(default)]
    module_path: Option<String>,
    #[serde(default)]
    verify: Option<String>,
    #[serde(default)]
    duration_ms: Option<f64>,
    status: String,
}

/// The tests the attribute recorded, by the name libtest prints for them.
/// Entries from a `specforge-test` too old to record the module path are
/// known by their function name alone.
#[derive(Default)]
struct Recorded {
    paths: BTreeSet<String>,
    bare_names: BTreeSet<String>,
}

impl Recorded {
    fn add(&mut self, entry: &Entry) {
        match entry.module_path.as_deref() {
            // libtest drops the crate (the first segment of the module path).
            Some(module) => {
                let path = match module.split_once("::") {
                    Some((_, inner)) => format!("{inner}::{}", entry.test_name),
                    None => entry.test_name.clone(),
                };
                self.paths.insert(path);
            }
            None => {
                self.bare_names.insert(entry.test_name.clone());
            }
        }
    }

    fn contains(&self, name: &str) -> bool {
        let last = name.rsplit("::").next().unwrap_or(name);
        self.paths.contains(name) || self.bare_names.contains(last)
    }
}

/// Map `specforge-test` reports to entity results, and return the tests in
/// libtest's output (the captured stdout, or a report file that isn't a
/// binary report) that no report recorded as unlinked. Other files in the
/// report directory, like the graph snapshot the integration keeps there,
/// yield nothing.
fn collect(input: &CollectInput) -> CollectOutput {
    let mut by_entity: BTreeMap<String, Vec<CollectTestResult>> = BTreeMap::new();
    let mut recorded = Recorded::default();
    let mut outputs: Vec<&str> = input.stdout.as_deref().into_iter().collect();
    for file in &input.reports {
        let Ok(report) = serde_json::from_str::<BinaryReport>(&file.content) else {
            outputs.push(&file.content);
            continue;
        };
        for entry in report.entries {
            recorded.add(&entry);
            let status = match entry.status.as_str() {
                "pass" => "passed",
                "fail" => "failed",
                _ => "skipped",
            };
            by_entity
                .entry(entry.entity_id)
                .or_default()
                .push(CollectTestResult {
                    name: entry.test_name,
                    status: status.to_string(),
                    verify: entry.verify,
                    duration_ms: entry.duration_ms,
                });
        }
    }
    let unlinked = outputs
        .into_iter()
        .flat_map(parse_libtest)
        .filter(|(name, _)| !recorded.contains(name))
        .map(|(name, status)| CollectUnlinkedTest {
            path: name.split("::").map(str::to_string).collect(),
            name,
            status: status.to_string(),
        })
        .collect();
    CollectOutput {
        entity_results: by_entity
            .into_iter()
            .map(|(entity_id, test_results)| CollectEntityResult {
                entity_id,
                test_results,
            })
            .collect(),
        unlinked,
    }
}

/// The test results in libtest's human-readable output: every
/// `test <name> ... <result>` line, as `(name, status)`. Doc tests
/// (`src/lib.rs - item (line 3)`) and benchmarks are skipped, as is the
/// `failures:` section, where a failing test's own output is replayed.
fn parse_libtest(output: &str) -> Vec<(String, &'static str)> {
    let mut results = Vec::new();
    let mut in_failures = false;
    for line in output.lines() {
        let line = strip_ansi(line.trim_end());
        if line == "failures:" {
            in_failures = true;
            continue;
        }
        if line.starts_with("test result:") || line.starts_with("running ") {
            in_failures = false;
            continue;
        }
        if in_failures {
            continue;
        }
        let Some((name, result)) = line
            .strip_prefix("test ")
            .and_then(|rest| rest.split_once(" ... "))
        else {
            continue;
        };
        let name = name.strip_suffix(" - should panic").unwrap_or(name);
        if name.contains(' ') {
            continue;
        }
        let status = match result {
            "ok" => "passed",
            "FAILED" => "failed",
            r if r == "ignored" || r.starts_with("ignored, ") => "skipped",
            _ => continue,
        };
        results.push((name.to_string(), status));
    }
    results
}

/// `text` without ANSI escape sequences (`ESC [ … letter`), which libtest
/// emits when it's asked for colour.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn collect_export(input: &[u8]) -> Result<Vec<u8>, String> {
    let input: CollectInput =
        serde_json::from_slice(input).map_err(|e| format!("malformed collect input: {e}"))?;
    serde_json::to_vec(&collect(&input)).map_err(|e| e.to_string())
}

fn dispatch(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    match export {
        "collect__cargo_test" => Some(collect_export(input)),
        _ => None,
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build, handler = dispatch);

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test --workspace --no-fail-fast` on a crate with a lib test
    /// module, a nested module, an integration test and a doc test.
    const LIBTEST: &str = include_str!("../tests/fixtures/libtest-stdout.txt");

    fn file(path: &str, content: &str) -> CollectReportFile {
        CollectReportFile {
            path: path.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn groups_entries_by_entity_across_binaries() {
        let input = CollectInput {
            reports: vec![
                file(
                    "target/specforge/a.json",
                    r#"{"schema_version":"1.0","binary_name":"a","entries":[
                        {"entity_kind":"behavior","entity_id":"login","test_name":"ok","file":"a.rs","verify":"accepts","duration_ms":3,"status":"pass"},
                        {"entity_kind":"behavior","entity_id":"login","test_name":"bad","file":"a.rs","duration_ms":1,"status":"fail"}]}"#,
                ),
                file(
                    "target/specforge/b.json",
                    r#"{"entries":[{"entity_kind":"type","entity_id":"user","test_name":"t","file":"b.rs","duration_ms":0,"status":"skipped"}]}"#,
                ),
                file("target/specforge/graph.json", r#"{"nodes":[]}"#),
                file("target/specforge/junk.json", "not json"),
            ],
            ..CollectInput::default()
        };
        let out = collect(&input);
        let ids: Vec<&str> = out
            .entity_results
            .iter()
            .map(|e| e.entity_id.as_str())
            .collect();
        assert_eq!(ids, vec!["login", "user"]);
        let login = &out.entity_results[0].test_results;
        assert_eq!(login.len(), 2);
        assert_eq!(login[0].status, "passed");
        assert_eq!(login[0].verify.as_deref(), Some("accepts"));
        assert_eq!(login[1].status, "failed");
        assert_eq!(out.entity_results[1].test_results[0].status, "skipped");
        assert!(out.unlinked.is_empty());
    }

    #[test]
    fn parses_libtest_output() {
        let parsed = parse_libtest(LIBTEST);
        assert_eq!(
            parsed,
            vec![
                ("tests::plain_ignored".to_string(), "skipped"),
                ("tests::slow_one".to_string(), "skipped"),
                (
                    "tests::add_item::rejects_a_duplicate_item".to_string(),
                    "passed"
                ),
                (
                    "tests::add_item__rejects_an_empty_name".to_string(),
                    "passed"
                ),
                ("tests::panics".to_string(), "passed"),
                ("tests::fails".to_string(), "failed"),
                ("cart__is_empty_at_first".to_string(), "passed"),
            ],
            "doc tests and a failing test's replayed output are skipped"
        );
        let coloured = "test \u{1b}[32ma\u{1b}[0m ... \u{1b}[32mok\u{1b}[0m\r\n";
        assert_eq!(parse_libtest(coloured), vec![("a".to_string(), "passed")]);
        assert!(parse_libtest("test b ... bench:  10 ns/iter (+/- 1)").is_empty());
    }

    #[test]
    fn reports_tests_the_attribute_did_not_record_as_unlinked() {
        let report = r#"{"entries":[
            {"entity_id":"cart","test_name":"rejects_a_duplicate_item","module_path":"shop_lib::tests::add_item","file":"src/lib.rs","status":"pass"},
            {"entity_id":"cart","test_name":"panics","file":"src/lib.rs","status":"pass"}]}"#;
        let out = collect(&CollectInput {
            reports: vec![file("target/specforge/shop_lib.json", report)],
            stdout: Some(LIBTEST.to_string()),
        });
        let names: Vec<&str> = out.unlinked.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "tests::plain_ignored",
                "tests::slow_one",
                "tests::add_item__rejects_an_empty_name",
                "tests::fails",
                "cart__is_empty_at_first",
            ],
            "recorded by module path, or by name for an old report"
        );
        assert_eq!(
            out.unlinked[2].path,
            vec!["tests", "add_item__rejects_an_empty_name"]
        );
        assert_eq!(out.unlinked[3].status, "failed");
    }

    #[test]
    fn reads_libtest_output_passed_as_a_report_file() {
        let out = collect(&CollectInput {
            reports: vec![file("out.txt", LIBTEST)],
            ..CollectInput::default()
        });
        assert_eq!(out.unlinked.len(), 7);
    }
}
