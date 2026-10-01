// specforge-test crate features

use "behaviors"

feature test_annotation "Test Annotation" {
  problem  """
    Rust developers need a way to link their tests to spec entity IDs
    without changing how tests run. The standard test harness has no
    plugin API, and writing both #[test] and a linkage attribute on every
    test is noise that invites the two to drift apart.
  """
  solution """
    A proc macro #[specforge_test(behavior = "id", verify = "...")] that
    registers the test itself and injects a Drop-based guard. The guard
    records pass/fail/skipped per entity without interfering with the test
    harness. Runner attributes such as #[tokio::test] and #[rstest] keep
    registering their tests; the macro defers to them.
    `specforge collect` (@specforge/cargo-test) runs the tests and reads
    the per-binary reports.
  """
}

feature result_collection "Result Collection" {
  problem  """
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
  problem  """
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
  problem  """
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
