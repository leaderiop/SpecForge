// @rust/test features

use "extensions/rust/test/behaviors"

feature rt_nextest_facts "nextest Test Facts" {
  status   in_progress
  problem  """
    A Rust project's tests became facts only through @specforge/cargo-test's
    per-binary reports, written by a macro the tests had to use: a project
    testing with plain #[test] and nextest had no runner pack, and an
    ignored test, which nextest does not report, vanished.
  """
  solution """
    @rust/test runs cargo-nextest under a profile the config file the host
    writes for it defines, reads nextest's JUnit report with the JUnit
    library and answers a test and a test_run per case, named by its binary
    id and path. It joins @rust/source's items by their runner id, so a test
    is placed where it is written and names what its attribute names, and
    an ignored test item the report lacks is a skipped test (ADR 0348 D8).
  """
}
