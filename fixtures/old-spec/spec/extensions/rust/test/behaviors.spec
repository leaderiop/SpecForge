// @rust/test behaviors

use "extensions/code/core/features"
use "types/wasm"

behavior rt_declare_test_collector "Declare the rust-test Collector" {
  features [rt_nextest_facts, cc_test_facts]
  category query
  types    [CollectorDescriptor]
  contract """
    @rust/test MUST declare one collector, `rust-test`, selected by a
    Cargo.toml at the project root. Its command is `cargo-nextest nextest
    run --workspace --no-tests=warn --tool-config-file specforge:{config}
    --profile specforge`, the program cargo-nextest so a missing nextest is
    not found on PATH and its install line `cargo install cargo-nextest
    --locked` is said. Its config `nextest.toml` defines the profile
    `specforge`, which runs every test and writes JUnit to the report path.
    Its report is `.specforge/reports/rust-test.xml`; it is served the item
    facts and requires @code/core ^1.1 and @rust/source ^1.0 (ADR 0348 D8).
  """
  ensures {
    nextest_program "the program run is cargo-nextest"
    tool_config     "the config is layered under the project's own"
    install_line    "a missing nextest names its install line"
  }
  verify unit "rust-test declares nextest, its config, its install line and its peers"
  verify unit "the config's specforge profile writes JUnit to the report path"
  verify integration "a missing nextest names its install line and keeps the packages and items"
}

behavior rt_map_nextest_report "Map a nextest Report to Test Facts" {
  features [rt_nextest_facts, cc_test_facts]
  category query
  types    [CollectInput, CollectOutput, Fact]
  contract """
    `collect__rust_test` MUST answer a test and a test_run per case of the
    report, read by specforge-code-junit and minted by specforge-code-facts:
    rust:<binary id>#<name>, the binary id the one <testsuite> holding the
    case. A test whose id an item's test_id holds MUST take the item's
    places, file and names. A case outside exactly one <testsuite>, or a
    report that is not JUnit, is the collector's error.
  """
  ensures {
    named_by_binary "a test is named by its binary id and path"
    placed          "a test is where its item is written"
    nextest_only    "a report that is not nextest's is the collector's error"
  }
  verify unit "cases become tests named by their binary id and path"
  verify unit "a test takes its places, file and names from its item"
  verify unit "a case outside exactly one testsuite is the collector's error"
  verify unit "an empty report has no test"
  verify integration "rust-test writes its nextest config and runs cargo-nextest with it"
}

behavior rt_fill_ignored_tests "Fill the Ignored Tests nextest Omits" {
  features [rt_nextest_facts, cc_test_facts]
  category computation
  types    [Fact]
  contract """
    Every served item whose attrs hold test and ignore and whose test_id the
    report lacks MUST become a skipped test without a test_run, placed at
    the item: a cited ignored test is held, not code/E101. An ignored test
    the report holds keeps the status it ran with (ADR 0348 D8).
  """
  ensures {
    held_not_missing "an ignored test is a skipped test"
    no_run           "a test that did not run has no test_run"
  }
  verify unit "an ignored test the report omits is a skipped test without a run"
  verify unit "an ignored test the report holds keeps the status it ran with"
}
