// @javascript/node-test features

use "extensions/javascript/node-test/behaviors"

feature nt_node_test_collection "node --test Collection" {
  status   in_progress
  problem  """
    A JavaScript project tested with node's own test runner had no runner
    pack: its tests never became facts a spec could cite, so a requirement
    could not name the test that proves it.
  """
  solution """
    @javascript/node-test is the runner pack for `node --test`. `specforge
    collect` runs the project's own node with the spec and JUnit reporters,
    with the user's consent, and the pack answers a test and a test_run fact
    per test (ADR 0327), named js:<package>/<file>#<test> after the nearest
    named package.json (ADR 0329), for @code/core to check.
  """
}
