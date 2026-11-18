//! Pure functions the checks call; each is tested on literal input.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde_json::Value;

use crate::baseline::{PATH as BASELINE, WarningCounts};

pub struct SpecVerdict {
    pub lines: Vec<String>,
    pub failed: bool,
    /// With `write`: the `specs` section lowered to today's counts (never raised, never with an error left).
    pub new_baseline: Option<WarningCounts>,
}

/// The spec checks' verdict (was `gate.sh:94-137`): errors per project, and every warning code whose count differs
/// from `baseline` (the `specs` section). `write` lowers the section and refuses when a count rose or an error remains.
pub fn spec_verdict(
    found: &BTreeMap<String, Vec<Value>>,
    baseline: &WarningCounts,
    write: bool,
) -> SpecVerdict {
    let mut lines = Vec::new();
    let mut failed = false;
    let mut counts = WarningCounts::new();
    let nothing = BTreeMap::new();
    for (project, diagnostics) in found {
        fn severity(d: &Value) -> Option<&str> {
            d["severity"].as_str()
        }
        let errors: Vec<&Value> = diagnostics
            .iter()
            .filter(|d| severity(d) == Some("Error"))
            .collect();
        // A machine warning is this machine's network, approvals or journal, not the project's: it is never counted
        // against the baseline, only named (ADR 0337 D7).
        let (mut warnings, mut machine): (BTreeMap<String, u64>, BTreeMap<String, u64>) =
            Default::default();
        for d in diagnostics
            .iter()
            .filter(|d| severity(d) == Some("Warning"))
        {
            let counted = if d["machine"] == true {
                &mut machine
            } else {
                &mut warnings
            };
            *counted
                .entry(d["code"].as_str().unwrap_or("?").to_string())
                .or_default() += 1;
        }
        lines.push(format!(
            "{project}: {} errors, {} warnings",
            errors.len(),
            warnings.values().sum::<u64>()
        ));
        if !machine.is_empty() {
            let named: Vec<String> = machine
                .iter()
                .map(|(code, n)| format!("{code} {n}"))
                .collect();
            lines.push(format!(
                "  {} machine warnings not judged: {}",
                machine.values().sum::<u64>(),
                named.join(", ")
            ));
        }
        for d in &errors {
            let span = &d["span"];
            lines.push(format!(
                "  {} {}:{}: {}",
                d["code"].as_str().unwrap_or("?"),
                span["file"].as_str().unwrap_or("?"),
                span["start_line"],
                d["message"].as_str().unwrap_or("")
            ));
        }
        failed |= !errors.is_empty();
        let allowed = baseline.get(project).unwrap_or(&nothing);
        let codes: BTreeSet<&str> = warnings
            .keys()
            .chain(allowed.keys())
            .map(String::as_str)
            .collect();
        for code in codes {
            let now = warnings.get(code).copied().unwrap_or(0);
            let was = allowed.get(code).copied().unwrap_or(0);
            if now > was {
                lines.push(format!(
                    "  {code} rose {was} -> {now}: fix the new warnings"
                ));
                failed = true;
            } else if now < was && !write {
                lines.push(format!(
                    "  {code} fell {was} -> {now}: lower it in {BASELINE} in the same commit (cargo gate baseline --only specs)"
                ));
                failed = true;
            }
        }
        if !warnings.is_empty() {
            counts.insert(project.clone(), warnings);
        }
    }
    let mut new_baseline = None;
    if write {
        if failed {
            lines.push(format!(
                "{BASELINE} not written: fix the errors and risen counts first"
            ));
        } else {
            new_baseline = Some(counts);
        }
    }
    SpecVerdict {
        lines,
        failed,
        new_baseline,
    }
}

/// How many failed requirements [`requirement_verdict`] names one by one before it counts the rest.
const FAILED_NAMED: usize = 10;

/// The requirement verdict (ADR 0374 D5): each `requirement` node of `export` (`specforge export --format graph
/// --kinds requirement`) and its computed `proof` (`@gsd/core`), a node with none counted under `none`. It fails
/// when there is no requirement, when none is `proven` (a test run with an empty result shows red) or when any is
/// `failed`, naming the first failed ones. It always prints `requirements proven: N of M` with the count of each
/// other state, and how many `no_test` requirements only guests' tests name (`guest_named`): guests run under
/// `cargo test` and write no JUnit, so what only they prove reads `no_test`.
pub fn requirement_verdict(export: &Value, guest_named: &BTreeSet<String>) -> (Vec<String>, bool) {
    let mut states: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in export["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| n["kind"] == "requirement")
    {
        let state = node["fields"]["proof"].as_str().unwrap_or("none");
        states
            .entry(state)
            .or_default()
            .push(node["id"].as_str().unwrap_or(""));
    }
    let count = |state: &str| states.get(state).map_or(0, Vec::len);
    let total: usize = states.values().map(Vec::len).sum();
    let guests_only = states.get("no_test").map_or(0, |ids| {
        ids.iter().filter(|id| guest_named.contains(**id)).count()
    });
    let others: String = states
        .iter()
        .filter(|(state, _)| !matches!(**state, "proven" | "failed" | "not_run" | "no_test"))
        .map(|(state, ids)| format!(", {state} {}", ids.len()))
        .collect();
    let mut lines = vec![format!(
        "requirements proven: {} of {total} (failed {}, not_run {}, no_test {}, of them {guests_only} named only by \
         guests' tests{others})",
        count("proven"),
        count("failed"),
        count("not_run"),
        count("no_test"),
    )];
    let mut failed = false;
    if total == 0 {
        lines.push("no requirement in the export: the spec holds nothing to prove".to_string());
        failed = true;
    } else if count("proven") == 0 {
        lines.push(
            "no requirement is proven: no passing test names one (run `cargo gate full --only tests` first)"
                .to_string(),
        );
        failed = true;
    }
    let failures = states.get("failed").map(Vec::as_slice).unwrap_or_default();
    for id in failures.iter().take(FAILED_NAMED) {
        lines.push(format!("{id} failed: a test that names it failed"));
    }
    if failures.len() > FAILED_NAMED {
        lines.push(format!("and {} more failed", failures.len() - FAILED_NAMED));
    }
    failed |= !failures.is_empty();
    (lines, failed)
}

/// The `executable` of `bin`'s compiler-artifact message in cargo's JSON output (the last one, when it was built
/// more than once).
pub fn executable_of(messages: &str, bin: &str) -> Option<PathBuf> {
    messages
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|m| m["reason"] == "compiler-artifact" && m["target"]["name"] == bin)
        .filter(|m| {
            m["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|k| k == "bin"))
        })
        .filter_map(|m| m["executable"].as_str().map(PathBuf::from))
        .next_back()
}

/// Whether a crate's sources hold a runnable doctest (was `gate.sh:54-66`'s awk): a `///` or `//!` fence opening with
/// no language, `rust`, `compile_fail`, `should_panic` or `no_run`. A closing fence never reopens.
pub fn has_runnable_doctest(sources: &[&str]) -> bool {
    sources.iter().any(|source| {
        let mut open = false;
        source.lines().any(|line| {
            let Some(after) = fence_language(line) else {
                return false;
            };
            if open {
                open = false;
                return false;
            }
            open = true;
            matches!(
                after.trim_end_matches([' ', '\t']),
                "" | "rust" | "compile_fail" | "should_panic" | "no_run"
            )
        })
    })
}

/// The text after the backticks of a doc-comment fence line (`/// ```rust` gives `rust`).
fn fence_language(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches([' ', '\t']).strip_prefix("//")?;
    let rest = rest.strip_prefix('/').or_else(|| rest.strip_prefix('!'))?;
    rest.trim_start_matches([' ', '\t']).strip_prefix("```")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_runnable_fence_marks_a_doctest_crate() {
        for lang in ["", "rust", "no_run", "compile_fail", "should_panic"] {
            let source = format!("/// Doc.\n/// ```{lang}\n/// let a = 1;\n/// ```\nfn f() {{}}\n");
            assert!(has_runnable_doctest(&[&source]), "```{lang}");
        }
        for lang in ["text", "ignore", "json", "rust,ignore"] {
            let source = format!("//! ```{lang}\n//! x\n//! ```\n");
            assert!(!has_runnable_doctest(&[&source]), "```{lang}");
        }
        // The closing fence of a text block does not open a runnable one.
        assert!(!has_runnable_doctest(&["/// ```text\n/// x\n/// ```\n"]));
        // A plain comment is not a doc comment.
        assert!(!has_runnable_doctest(&["// ```\n// x\n// ```\n"]));
        assert!(!has_runnable_doctest(&[]));
    }

    use serde_json::json;

    fn allowed(entries: &[(&str, &str, u64)]) -> WarningCounts {
        let mut counts = WarningCounts::new();
        for &(project, code, n) in entries {
            counts
                .entry(project.to_string())
                .or_default()
                .insert(code.to_string(), n);
        }
        counts
    }

    fn found(project: &str, diagnostics: Value) -> BTreeMap<String, Vec<Value>> {
        BTreeMap::from([(project.to_string(), diagnostics.as_array().unwrap().clone())])
    }

    fn warnings(code: &str, count: usize) -> Value {
        Value::Array(vec![
            json!({"severity": "Warning", "code": code, "message": "m"});
            count
        ])
    }

    #[test]
    fn a_warning_that_rose_fails_and_one_that_fell_asks_for_the_baseline() {
        // §3.3: W003 1 -> 2 failed in gate.sh and passed in CI.
        let baseline = allowed(&[(".", "W003", 1)]);
        let rose = spec_verdict(&found(".", warnings("W003", 2)), &baseline, false);
        assert!(rose.failed);
        assert!(
            rose.lines
                .contains(&"  W003 rose 1 -> 2: fix the new warnings".to_string()),
            "{:?}",
            rose.lines
        );

        let fell = spec_verdict(&found(".", warnings("W003", 0)), &baseline, false);
        assert!(fell.failed);
        assert!(
            fell.lines
                .iter()
                .any(|l| l.contains("W003 fell 1 -> 0: lower it in gate/baseline.json in the same commit (cargo gate baseline --only specs)"))
        );

        let same = spec_verdict(&found(".", warnings("W003", 1)), &baseline, false);
        assert!(!same.failed && same.new_baseline.is_none());

        // `write` lowers a fallen count, drops a project with none, and refuses after a rise.
        let lowered = spec_verdict(&found(".", warnings("W003", 0)), &baseline, true);
        assert!(!lowered.failed);
        assert_eq!(lowered.new_baseline, Some(WarningCounts::new()));
        let refused = spec_verdict(&found(".", warnings("W003", 2)), &baseline, true);
        assert!(refused.failed && refused.new_baseline.is_none());
        assert!(refused.lines.last().unwrap().contains("not written"));
    }

    #[test]
    fn machine_warnings_stay_out_of_the_baseline() {
        let baseline = allowed(&[(".", "W003", 1)]);
        let diagnostics = json!([
            {"severity": "Warning", "code": "W003", "message": "m"},
            {"severity": "Warning", "code": "checks/W172", "message": "m", "machine": true},
            {"severity": "Warning", "code": "checks/W172", "message": "m", "machine": true},
        ]);
        let verdict = spec_verdict(&found(".", diagnostics), &baseline, true);
        assert!(!verdict.failed, "{:?}", verdict.lines);
        assert_eq!(
            verdict.new_baseline,
            Some(baseline),
            "the project's counts are as they were"
        );
        assert_eq!(
            verdict.lines,
            [
                ".: 0 errors, 1 warnings",
                "  2 machine warnings not judged: checks/W172 2"
            ]
        );
    }

    #[test]
    fn an_error_fails_the_spec_checks_whatever_the_baseline() {
        let error = json!([{"severity": "Error", "code": "E001", "message": "boom",
                            "span": {"file": "a.spec", "start_line": 7}}]);
        let verdict = spec_verdict(&found("p", error), &WarningCounts::new(), false);
        assert!(verdict.failed);
        assert!(
            verdict.lines.contains(&"  E001 a.spec:7: boom".to_string()),
            "{:?}",
            verdict.lines
        );
        let written = spec_verdict(
            &found("p", json!([{"severity": "Error", "code": "E001"}])),
            &WarningCounts::new(),
            true,
        );
        assert!(written.failed && written.new_baseline.is_none());
    }

    /// The requirement nodes of the fixture's real export (`xtask/tests/fixtures/requirement-links` collected through
    /// `rust-cargo`, `rust-source` and `rust-test`), cut to `id`, `kind` and `fields.proof`.
    fn tracer_export() -> Value {
        json!({"nodes": [
            {"id": "TRACER-01", "kind": "requirement", "fields": {"proof": "proven"}},
            {"id": "TRACER-02", "kind": "requirement", "fields": {"proof": "proven"}},
            {"id": "TRACER-03", "kind": "requirement", "fields": {"proof": "no_test"}},
        ]})
    }

    #[test]
    fn requirement_verdict_fails_an_export_with_no_requirement() {
        let decision = json!({"nodes": [{"id": "D-001", "kind": "decision", "fields": {}}]});
        for export in [json!({"nodes": []}), json!({}), decision] {
            let (lines, failed) = requirement_verdict(&export, &BTreeSet::new());
            assert!(failed, "{lines:?}");
            assert!(
                lines[1].starts_with("no requirement in the export"),
                "{lines:?}"
            );
        }
    }

    #[test]
    fn requirement_verdict_fails_when_none_is_proven() {
        let export = json!({"nodes": [
            {"id": "A-01", "kind": "requirement", "fields": {}},
            {"id": "A-02", "kind": "requirement", "fields": {"proof": "no_test"}},
        ]});
        let (lines, failed) = requirement_verdict(&export, &BTreeSet::new());
        assert!(failed);
        assert_eq!(
            lines[0],
            "requirements proven: 0 of 2 (failed 0, not_run 0, no_test 1, of them 0 named only by guests' tests, \
             none 1)"
        );
        assert!(
            lines[1].starts_with("no requirement is proven"),
            "{lines:?}"
        );
    }

    #[test]
    fn requirement_verdict_fails_a_failed_requirement_and_names_it() {
        let mut export = tracer_export();
        export["nodes"][1]["fields"]["proof"] = json!("failed");
        let (lines, failed) = requirement_verdict(&export, &BTreeSet::new());
        assert!(failed);
        assert_eq!(
            lines,
            [
                "requirements proven: 1 of 3 (failed 1, not_run 0, no_test 1, of them 0 named only by guests' tests)",
                "TRACER-02 failed: a test that names it failed",
            ]
        );
    }

    #[test]
    fn requirement_verdict_names_ten_failed_and_counts_the_rest() {
        let mut nodes =
            vec![json!({"id": "OK-01", "kind": "requirement", "fields": {"proof": "proven"}})];
        nodes.extend((1..=12).map(|n| {
            json!({"id": format!("BAD-{n:02}"), "kind": "requirement", "fields": {"proof": "failed"}})
        }));
        let (lines, failed) = requirement_verdict(&json!({ "nodes": nodes }), &BTreeSet::new());
        assert!(failed);
        assert_eq!(lines.len(), 1 + 10 + 1, "{lines:?}");
        assert_eq!(lines.last().unwrap(), "and 2 more failed");
    }

    #[test]
    fn requirement_verdict_passes_the_fixtures_export() {
        let (lines, failed) = requirement_verdict(&tracer_export(), &BTreeSet::new());
        assert!(!failed, "{lines:?}");
        assert_eq!(
            lines,
            [
                "requirements proven: 2 of 3 (failed 0, not_run 0, no_test 1, of them 0 named only by guests' tests)"
            ]
        );
    }

    #[test]
    fn requirement_verdict_counts_the_no_test_requirements_only_guests_name() {
        let guests = BTreeSet::from(["TRACER-03".to_string(), "TRACER-01".to_string()]);
        let (lines, failed) = requirement_verdict(&tracer_export(), &guests);
        assert!(!failed, "{lines:?}");
        assert!(
            lines[0].contains("no_test 1, of them 1 named only by guests' tests"),
            "{lines:?}"
        );
    }

    #[test]
    fn the_specforge_binary_is_where_cargo_says() {
        // §3.4: gate.sh assumed target/debug/specforge; cargo builds into CARGO_TARGET_DIR.
        let messages = [
            r#"{"reason":"compiler-artifact","target":{"name":"specforge","kind":["bin"]},"executable":"/elsewhere/targets/p04/debug/specforge"}"#,
            r#"{"reason":"compiler-artifact","target":{"name":"specforge","kind":["lib"]},"executable":null}"#,
            r#"{"reason":"compiler-artifact","target":{"name":"other","kind":["bin"]},"executable":"/x/other"}"#,
            r#"{"reason":"build-finished","success":true}"#,
            "not json",
        ]
        .join("\n");
        assert_eq!(
            executable_of(&messages, "specforge"),
            Some(PathBuf::from("/elsewhere/targets/p04/debug/specforge"))
        );
        assert_eq!(executable_of(&messages, "missing"), None);
    }
}
