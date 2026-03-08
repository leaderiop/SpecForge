// specforge-test crate features

use behaviors

feature test_annotation "Test Annotation" {
  behaviors [expand_test_attribute, record_test_result_on_drop, register_atexit_handler]

  problem """
    Rust developers need a way to link their #[test] functions to spec
    entity IDs without changing how tests run. The standard test harness
    has no plugin API, so any solution must work alongside #[test], not
    replace it.
  """

  solution """
    A proc macro #[specforge::test(behavior = "id")] that stacks alongside
    #[test] and injects a Drop-based guard. The guard records pass/fail
    per entity without interfering with the test harness. Works with
    #[tokio::test], #[rstest], proptest, and other frameworks.
  """
}

feature result_collection "Result Collection" {
  behaviors [emit_binary_report, resolve_convention_mapping]

  problem """
    Test results must be captured and written to disk so that
    `specforge collect rust` can transform them into specforge-report.json.
    Multiple test binaries in a workspace each need their own report file.
  """

  solution """
    An atexit handler writes target/specforge/<binary-name>.json containing
    all collected TestRecordEntries. Convention-based mapping (module names,
    double-underscore naming) provides zero-config fallback for projects
    that don't use the proc macro.
  """
}

feature build_integration "Build Integration" {
  behaviors [invoke_specforge_export, generate_entity_constants]

  problem """
    The graph export can become stale if developers edit .spec files
    without re-exporting. Typos in entity ID strings are only caught
    at collection time, not at compile time.
  """

  solution """
    build.rs calls `specforge export` on every build, keeping the graph
    fresh. It optionally generates entity ID constants so that typos
    become compile errors. Graceful degradation when specforge is not
    installed means no hard dependency on the tool.
  """
}

feature coverage_summary "Coverage Summary" {
  behaviors [load_graph_at_exit, compute_coverage_diff, print_coverage_summary]

  problem """
    Developers must run `specforge trace` as a separate step to see
    which behaviors lack test coverage. This breaks the feedback loop
    and delays awareness of gaps.
  """

  solution """
    The atexit handler reads the graph export and prints a coverage
    summary to stderr immediately after cargo test completes. Developers
    see which entities are fully covered, partially covered, or missing
    tests without leaving their terminal. The graph export timestamp
    makes staleness visible.
  """
}
