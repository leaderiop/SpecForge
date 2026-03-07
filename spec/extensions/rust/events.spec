// @specforge/rust extension events

use extensions/rust/types
use extensions/rust/behaviors

event rust_tests_collected "Rust Tests Collected" {
  trigger   collect_rust_test_results
  channel   "coverage.rust_collected"

  payload   RustTestsCollectedPayload

  consumers [emit_specforge_report_from_rust]

  verify integration "emits rust_tests_collected with correct mapping counts and format"

}
