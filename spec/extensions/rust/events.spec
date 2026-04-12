// @specforge/rust extension events

use "extensions/rust/types"
event rust_tests_collected "Rust Tests Collected" {
  channel   "coverage.rust_collected"

  payload   RustTestsCollectedPayload


  verify integration "emits rust_tests_collected with correct mapping counts and format"

}
