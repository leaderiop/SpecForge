//! A harness value is recorded by its line in `specforge-harness`, read as a check input; one with no line is
//! E083 (ADR 0287, behavior `record_harness_values`).

use std::sync::Arc;

use serde_json::json;
use specforge_package::Sha256;
use specforge_project::testing::gsd_history_runtime;
use specforge_test::prelude::*;

use crate::computed::{BASE, project, with_code};

const UAT: &str = "uat_test U1 \"U\" {\n  phase 03\n  result pass\n}\n";

/// The ledger line that records `value` (JSON text) for `field` of `path`.
fn line(path: &str, field: &str, value: &str) -> String {
    let bytes = format!("{path}\0{field}\0{value}");
    format!("{path} {field} {}\n", Sha256::of(bytes.as_bytes()))
}

fn diagnostics_with(ledger: Option<&str>, spec: &str) -> Vec<specforge_common::Diagnostic> {
    let dir = project(spec);
    if let Some(ledger) = ledger {
        dir.write("specforge-harness", ledger);
    }
    dir.compile_in(Arc::new(gsd_history_runtime(json!([]))))
        .diagnostics()
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "a harness value with no matching line in specforge-harness is E083"
)]
#[ignore = "K4 T6"]
fn a_harness_value_without_its_line_is_e083() {
    let header = "# specforge-harness 1\n";
    let _ = BASE;
    let none = diagnostics_with(None, UAT);
    assert_eq!(with_code(&none, "E083").len(), 1, "{none:?}");
    let recorded = format!("{header}{}", line("v1.0/03/U1", "result", "\"pass\""));
    let ok = diagnostics_with(Some(&recorded), UAT);
    assert!(with_code(&ok, "E083").is_empty(), "{ok:?}");
    let other = format!("{header}{}", line("v1.0/03/U1", "result", "\"fail\""));
    assert_eq!(
        with_code(&diagnostics_with(Some(&other), UAT), "E083").len(),
        1
    );
    // An orphan line under another path that matches the value is named.
    let orphan = format!("{header}{}", line("v1.0/02/U1", "result", "\"pass\""));
    let named = diagnostics_with(Some(&orphan), UAT);
    let e083 = with_code(&named, "E083");
    assert_eq!(e083.len(), 1, "{named:?}");
    assert!(e083[0].message.contains("v1.0/02/U1"), "{e083:?}");
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "a malformed ledger line is W163 and ignored, and a bad header makes every harness value E083"
)]
#[ignore = "K4 T6"]
fn a_malformed_ledger_line_is_w163_and_ignored() {
    let recorded = line("v1.0/03/U1", "result", "\"pass\"");
    let bad = format!(
        "# specforge-harness 1\nshort line\nv1.0/03/U1 result nothex\n<<<<<<< HEAD\n{recorded}"
    );
    let diagnostics = diagnostics_with(Some(&bad), UAT);
    let w163 = with_code(&diagnostics, "W163");
    assert_eq!(w163.len(), 3, "{diagnostics:?}");
    assert!(
        with_code(&diagnostics, "E083").is_empty(),
        "{diagnostics:?}"
    );
    let header = format!("# specforge-harness 2\n{recorded}");
    let diagnostics = diagnostics_with(Some(&header), UAT);
    assert_eq!(with_code(&diagnostics, "W163").len(), 1, "{diagnostics:?}");
    assert_eq!(with_code(&diagnostics, "E083").len(), 1, "{diagnostics:?}");
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "a union-merged ledger reads, and is I203 once"
)]
#[ignore = "K4 T6"]
fn a_union_merged_ledger_reads_and_is_i203() {
    let current = line("v1.0/03/U1", "result", "\"pass\"");
    let stale = line("v1.0/03/U1", "result", "\"fail\"");
    let union = format!("# specforge-harness 1\n{stale}# specforge-harness 1\n{current}");
    let diagnostics = diagnostics_with(Some(&union), UAT);
    assert!(
        with_code(&diagnostics, "E083").is_empty(),
        "{diagnostics:?}"
    );
    assert_eq!(with_code(&diagnostics, "I203").len(), 1, "{diagnostics:?}");
}
