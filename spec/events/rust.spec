// Rust-specific events

use types/rust-codegen
use behaviors/rust-codegen
use behaviors/rust-collection

event rust_code_generated "Rust Code Generated" {
  trigger   generate_rust_test_stubs
  channel   "gen.rust_generated"

  payload {
    fileCount     integer
    typeCount     integer
    portCount     integer
    testCount     integer
    benchCount    integer
    timestamp     timestamp
  }

  consumers [detect_rust_code_drift]

  verify integration "emits rust_code_generated with correct file and entity counts"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

event rust_tests_collected "Rust Tests Collected" {
  trigger   collect_rust_test_results
  channel   "coverage.rust_collected"

  payload {
    totalTests      integer
    mappedTests     integer
    unmappedTests   integer
    format          CollectFormat
    timestamp       timestamp
  }

  consumers [emit_specforge_report_from_rust]

  verify integration "emits rust_tests_collected with correct mapping counts and format"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
