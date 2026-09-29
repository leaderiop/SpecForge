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

use specforge_extension_sdk::prelude::*;
use std::collections::BTreeMap;

const COLLECTOR: &str = "cargo-test";

#[specforge_extension_sdk::extension(name = "@specforge/cargo-test", version = "1.0.0")]
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
                .detect_files(&["Cargo.toml"])
                .run(&["cargo", "test", "--workspace", "--no-fail-fast"])
                .report("target/specforge");
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
    verify: Option<String>,
    #[serde(default)]
    duration_ms: Option<f64>,
    status: String,
}

/// Map `specforge-test` reports to entity results. Files that aren't
/// binary reports (the graph snapshot the integration also keeps in the
/// report directory) are skipped.
fn collect(input: &CollectInput) -> CollectOutput {
    let mut by_entity: BTreeMap<String, Vec<CollectTestResult>> = BTreeMap::new();
    for file in &input.reports {
        let Ok(report) = serde_json::from_str::<BinaryReport>(&file.content) else {
            continue;
        };
        for entry in report.entries {
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
    CollectOutput {
        entity_results: by_entity
            .into_iter()
            .map(|(entity_id, test_results)| CollectEntityResult {
                entity_id,
                test_results,
            })
            .collect(),
    }
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
    }
}
