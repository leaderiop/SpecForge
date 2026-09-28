//! `specforge collect` — ingest test-runner reports into the project's
//! `specforge-report.json` so `specforge analyze coverage --test-results`
//! can score verify statements (RES-15 layer 3).
//!
//! v1 ingests the collector JSON shape `ingest_collector_report` understands:
//! `{"entity_results": [{"entity_id": "...", "test_results":
//! [{"name": "...", "status": "passed|failed"}]}]}`. junit/jest/pytest
//! conversion is planned. Extension-provided collector transforms
//! (`collect__*` wasm exports) will hook in here once an extension declares
//! the `collectors` contribution; none do today.

use crate::OutputFormat;
use specforge_common::find_project_root;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Default report locations: the rust integration's atexit handler writes
/// `target/specforge/<binary>.json`.
const DEFAULT_REPORT_GLOB: &str = "target/specforge";

pub fn run(path: &Path, collector: Option<&str>, reports: &[PathBuf], format: OutputFormat) -> i32 {
    let project_root = match find_project_root(path) {
        Some(root) => root,
        None => {
            let msg = "no specforge project found (missing specforge.json or specforge.spec)";
            return report_error(msg, "", format);
        }
    };

    // Compile the project: known entity ids gate which report entries map.
    let ctx = crate::pipeline::compile(&project_root);
    if !ctx.diagnostics.is_empty() && format != OutputFormat::Json {
        for d in &ctx.diagnostics {
            eprintln!("{}: {}", d.code, d.message);
        }
    }
    let known_ids: std::collections::HashSet<String> = ctx
        .graph
        .nodes()
        .iter()
        .map(|n| n.id.raw.to_string())
        .collect();

    // Resolve report inputs: explicit --report paths, else the default glob.
    let report_paths: Vec<PathBuf> = if reports.is_empty() {
        let dir = project_root.join(DEFAULT_REPORT_GLOB);
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        found.sort();
        found
    } else {
        reports.to_vec()
    };
    if report_paths.is_empty() {
        let msg = format!(
            "no report files found (passed --report or found *.json under {DEFAULT_REPORT_GLOB})"
        );
        return report_error(&msg, "E019", format);
    }

    let mut mapped = 0usize;
    let mut unmapped: Vec<serde_json::Value> = Vec::new();
    let mut updates: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    let mut ingested_files = 0usize;

    for report_path in &report_paths {
        let raw = match std::fs::read_to_string(report_path) {
            Ok(raw) => raw,
            Err(e) => {
                let msg = format!("failed to read report {}: {e}", report_path.display());
                return report_error(&msg, "E033", format);
            }
        };
        let mut report: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("report {} is not valid JSON: {e}", report_path.display());
                return report_error(&msg, "E045", format);
            }
        };

        // The specforge-test integration writes `{entries: [{entity_id,
        // test_name, verify, status: "pass"|"fail"}]}`; the ingest expects
        // `{entity_results: [{entity_id, test_results: [{name, status:
        // "passed"|"failed"}]}]}`. Normalize before ingesting so the two
        // halves of the protocol finally meet (C11-00).
        if report.get("entity_results").is_none()
            && let Some(entries) = report.get("entries").and_then(|v| v.as_array())
        {
            let mut by_entity: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
            for entry in entries {
                let Some(id) = entry.get("entity_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let status = entry
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("fail");
                let normalized = match status {
                    "pass" => "passed",
                    "fail" => "failed",
                    other => other,
                };
                by_entity
                    .entry(id.to_string())
                    .or_default()
                    .push(serde_json::json!({
                        "name": entry.get("test_name").and_then(|v| v.as_str()).unwrap_or(""),
                        "status": normalized,
                    }));
            }
            report = serde_json::json!({
                "entity_results": by_entity
                    .into_iter()
                    .map(|(entity_id, test_results)| serde_json::json!({
                        "entity_id": entity_id,
                        "test_results": test_results,
                    }))
                    .collect::<Vec<_>>(),
            });
        }

        let ingested = specforge_wasm::ingest_collector_report(&report, &known_ids);
        mapped += ingested.mapped_entries.len();
        unmapped.extend(ingested.unmapped_entries);
        for (entity_id, meta) in ingested.coverage_updates {
            let entry = updates.entry(entity_id).or_insert((0, 0, 0));
            entry.0 += meta.total;
            entry.1 += meta.passed;
            entry.2 += meta.failed;
        }
        ingested_files += 1;
    }

    // Merge into specforge-report.json (the TestReport shape consumed by
    // `specforge analyze coverage --test-results`).
    let out_path = project_root.join("specforge-report.json");
    let mut merged: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    if let Ok(existing) = std::fs::read_to_string(&out_path)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&existing)
        && let Some(results) = v.get("results").and_then(|r| r.as_object())
    {
        for (k, val) in results {
            merged.insert(k.clone(), val.clone());
        }
    }
    for (entity_id, (total, passed, failed)) in &updates {
        // The collector vocabulary is "passed"/"failed"; the analyze pass
        // matches "pass"/"fail". Normalize here so collect is the adapter.
        let tests: Vec<serde_json::Value> = (0..*passed)
            .map(|_| serde_json::json!({"status": "pass"}))
            .chain((0..*failed).map(|_| serde_json::json!({"status": "fail"})))
            .collect();
        merged.insert(
            entity_id.clone(),
            serde_json::json!({ "tests": tests, "total": total }),
        );
    }
    let out_doc = serde_json::json!({
        "runner": collector.unwrap_or("specforge-test"),
        "results": merged,
    });
    if let Err(e) = std::fs::write(
        &out_path,
        serde_json::to_string_pretty(&out_doc).expect("report serialization cannot fail"),
    ) {
        let msg = format!("failed to write {}: {e}", out_path.display());
        return report_error(&msg, "E033", format);
    }

    // Report
    if format == OutputFormat::Json {
        let output = serde_json::json!({
            "status": "collected",
            "files_ingested": ingested_files,
            "mapped_entries": mapped,
            "unmapped_entries": unmapped.len(),
            "entities_updated": updates.len(),
            "report": out_path.display().to_string(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("serialize JSON output")
        );
    } else {
        println!("collected {ingested_files} report file(s)");
        println!("mapped entries:   {mapped}");
        println!("entities updated: {}", updates.len());
        println!("report written:   {}", out_path.display());
        for entry in &unmapped {
            let id = entry
                .get("entity_id")
                .and_then(|v| v.as_str())
                .unwrap_or("<no id>");
            println!("W097: test record references unknown entity '{id}'");
            println!("  hint: check for renames or typos against the compiled graph");
        }
    }

    0
}

fn report_error(msg: &str, code: &str, format: OutputFormat) -> i32 {
    if format == OutputFormat::Json {
        let output = serde_json::json!({
            "error": msg,
            "code": code,
            "exit_code": 1,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("serialize JSON output")
        );
    } else {
        eprintln!("error[{code}]: {msg}");
    }
    1
}
