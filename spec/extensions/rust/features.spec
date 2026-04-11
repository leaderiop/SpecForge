// @specforge/rust extension features — Rust test collection

use "extensions/rust/behaviors"
feature rust_test_collection "Rust Test Collection" {
  behaviors [collect_rust_test_results, parse_junit_xml, parse_libtest_json, resolve_entity_mapping, validate_rust_entity_ids, merge_workspace_reports, emit_specforge_report_from_rust]

  problem """
    Rust test frameworks (cargo test, nextest) produce results in various
    formats but none natively link tests to spec entities. The specforge
    coverage pipeline needs a standardized specforge-report.json.
  """

  solution """
    specforge collect rust reads test output (JUnit XML, libtest JSON, or
    stdin), maps test names to entity IDs via three-level precedence
    (tests field > proc macro > naming convention), and emits
    specforge-report.json for the traceability pipeline.
  """
}

feature rust_proc_macro_annotation "Rust Proc Macro Annotation" {
  behaviors [record_test_via_drop_guard]

  problem """
    Naming conventions alone are fragile and can break when test functions
    are renamed. Developers need explicit, compiler-checked linkage from
    test functions to spec entity IDs.
  """

  solution """
    #[specforge::test("entity_id")] proc macro attribute wraps test body
    with a Drop-based guard that records pass/fail. Results are written
    to target/specforge/ for collection by specforge collect rust.
    Composable with #[tokio::test], #[rstest], etc.
  """
}
