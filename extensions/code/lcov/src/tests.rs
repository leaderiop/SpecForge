use specforge_code_facts::coverage_facts;
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// `cargo llvm-cov --lcov` form, hand-written to llvm-cov's `CoverageExporterLcov`: crate `demo` at `/project`.
const LLVM_COV: &str = include_str!("../fixtures/llvm-cov.info");
/// The served `item` and `package` facts a coverage collector receives, as JSON.
const ITEMS: &str = include_str!("../fixtures/items.json");

fn served() -> Vec<Fact> {
    serde_json::from_str(ITEMS).expect("the served facts parse")
}

#[test]
fn an_llvm_cov_report_mints_coverage_citing_its_items() {
    let files = read(LLVM_COV, "/project").expect("the report reads");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file, "src/lib.rs");
    insta::assert_json_snapshot!("llvm_cov", coverage_facts(&files, &served()));
}

#[test]
fn an_llvm_cov_report_reads_every_record() {
    let files = read(LLVM_COV, "/project").expect("the report reads");
    let file = &files[0];
    assert_eq!(file.functions.len(), 5);
    assert_eq!(
        file.functions[0],
        Function {
            line: 3,
            name: "_RNvCs4fqI2P2rA04_4demo3add".to_string(),
            hits: 4
        }
    );
    assert_eq!(file.lines.len(), 16);
    assert_eq!(
        file.branches,
        [
            Branch {
                line: 29,
                block: "0".to_string(),
                branch: "0".to_string(),
                taken: Some(0)
            },
            Branch {
                line: 29,
                block: "0".to_string(),
                branch: "1".to_string(),
                taken: Some(3)
            },
        ]
    );
}

#[test]
fn an_empty_text_has_no_file() {
    assert_eq!(read("", "/project"), Ok(Vec::new()));
    assert_eq!(read("\n\n", "/project"), Ok(Vec::new()));
}
