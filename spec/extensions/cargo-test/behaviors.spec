// @specforge/cargo-test behaviors

use "types/wasm"

behavior ct_declare_cargo_collector "Declare the cargo test Collector" {
  category query
  types [CollectorContribution]

  contract """
    @specforge/cargo-test MUST declare one collector, `cargo-test`, selected
    by a `Cargo.toml` at the project root. Its command is
    `cargo test --workspace --no-fail-fast` (one failing test binary must
    not stop the others from reporting) and its report is the `target/specforge`
    directory, where the `specforge-test` attribute writes one JSON report
    per test binary (or `$SPECFORGE_REPORT`, which the host sets when it
    runs the command). It requires @specforge/testing, which owns the
    `verify` vocabulary the results prove.
  """

  ensures {
    detected_by_cargo_toml "a project with Cargo.toml at its root selects cargo-test"
    declares_command       "the declared command is cargo test --workspace --no-fail-fast"
    requires_testing       "@specforge/testing is a required peer, enabled with it by specforge add"
  }

  verify unit "cargo-test declares its collector"
}

behavior ct_map_binary_reports "Map specforge-test Reports to Entities" {
  category query
  types [CollectorDispatchInput, CollectorReport]

  contract """
    `collect__cargo_test` MUST map every per-binary report entry
    (`entity_id`, `test_name`, `verify`, `duration_ms`, `status`) to a test
    result of that entity, in entity order: `pass` is passed, `fail` is
    failed and anything else is skipped. Files in the report directory
    that aren't binary reports are ignored. It runs nothing and reads no
    files: the host passes the report text in.
  """

  ensures {
    grouped_by_entity  "results are grouped by entity across binaries"
    statuses_mapped    "pass and fail map to passed and failed, anything else to skipped"
    non_reports_ignored "files that aren't binary reports are skipped"
  }

  verify unit "cargo-test maps per-binary reports to entity results"
}
