use super::*;

use std::path::PathBuf;

/// The gsd fixture whose two partition reports merge into the report its proof test collects.
fn fixture() -> PathBuf {
    specforge_repo_root::repo_root().join("xtask/tests/fixtures/requirement-links")
}

fn read(path: &str) -> String {
    let path = fixture().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_partitions_merge_into_one_report() {
    let parts = [read("junit/part-1.xml"), read("junit/part-2.xml")];
    assert_eq!(merge(&parts), Ok(read("junit.xml")));
}

#[test]
fn a_report_with_no_testsuites_root_is_refused() {
    let parts = [
        read("junit/part-1.xml"),
        "<?xml version=\"1.0\"?>\n<testsuite name=\"x\"/>\n".to_string(),
    ];
    let refused = merge(&parts).expect_err("no <testsuites> root");
    assert!(refused.contains("report 2"), "{refused}");
    let none = merge(&[]).expect_err("nothing to merge");
    assert!(
        none.contains("cargo gate full --only tests,guests,gsd,dogfood"),
        "{none}"
    );
}
