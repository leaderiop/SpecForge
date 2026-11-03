// @javascript/node-test behaviors

use "types/wasm"

behavior nt_declare_node_test_collector "Declare the node-test Collector" {
  features [nt_node_test_collection, cc_test_facts]
  category query
  types    [CollectorDescriptor]
  contract """
    @javascript/node-test MUST declare one collector, `node-test`, selected
    by a `package.json` at the project root. Its command is `node --test
    --test-reporter=spec --test-reporter-destination=stdout
    --test-reporter=junit --test-reporter-destination={report}`: node itself,
    not `npm test`, whose script may set reporters of its own; the spec
    reporter keeps the user's usual output and the JUnit reporter (node 20.10
    and later) writes the report the pack reads. Its report is the file
    `.specforge/reports/node-test.xml`, never a directory, which the host
    reads for its JSON files only. It reads every `package.json` of the
    project with its text, and requires @code/core, whose kinds it writes.
  """
  ensures {
    detected_by_package_json "a package.json at the project root selects node-test"
    both_reporters           "node's spec output goes to stdout and its JUnit report to the report path"
    report_path_expanded     "the JUnit report is written to the path the host chose"
    requires_code_core       "@code/core is a required peer"
  }
  verify unit "node-test declares its collector, its reads and its peer"
  verify integration "collect runs node --test with the spec and JUnit reporters and reads the report it wrote"
  verify integration "a package.json selects node-test among several collectors"
}

behavior nt_map_node_test_report "Map a node --test Report to Test Facts" {
  features [nt_node_test_collection, cc_test_facts]
  category query
  types    [CollectInput, CollectOutput, Fact]
  contract """
    `collect__node_test` MUST answer facts only, a test and a test_run per
    test of the JUnit report, minted by specforge-code-junit with the status
    node gave it. A test is named js:<package>/<file>#<suite > … > name>:
    its file is made relative to the project root (a Windows verbatim root
    included), and <package> is the name of the nearest read package.json at
    or above it, <file> the path under that package's directory. A report
    that is not JUnit, a test with no file, a file outside the project and a
    test with no named package at or above it are the collector's error,
    never a test dropped silently.
  """
  ensures {
    facts_only        "the answer is test and test_run facts, no entity result"
    named_by_package  "a test is named after the nearest package.json with a name"
    errors_never_drop "a test the pack cannot name is the collector's error"
  }
  verify unit "the twin's tests are named after its package"
  verify unit "a workspace test is named after its nearest package"
  verify unit "an absolute file is made relative to the project root"
  verify unit "a Windows file is made relative to a verbatim cwd"
  verify unit "a failing and a skipped test keep their status"
  verify unit "a test with no named package is the collector's error"
  verify unit "a case without a file is the collector's error"
  verify unit "a file outside the project, absolute or through .., is the collector's error"
  verify unit "a report that is not JUnit is the collector's error"
  verify integration "a failing node test exits 1 and its report is still read"
  verify integration "node-test's tests become test and test_run facts named js:<package>/<file>#<name>"
}
