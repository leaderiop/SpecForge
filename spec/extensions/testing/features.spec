// @specforge/testing features

use "extensions/testing/behaviors"

feature te_test_vocabulary "Runner-Agnostic Test Vocabulary" {
  behaviors [
    te_contribute_testability,
    te_validate_unverified_testable,
    te_validate_verify_kind_allowlist,
  ]
  problem   """
    Test concepts were spread across core and @specforge/software:
    testability flags on software's kinds, W004/W009 in its rules, and
    allowlists that disagreed with the kinds' own verify kinds. A project
    could not use software's vocabulary without also taking its opinion
    of what must be tested, and test runners had nowhere to plug in.
  """
  solution  """
    @specforge/testing owns the test vocabulary (ADR 0002): which kinds
    accept verify obligations and of which kinds, and the rules over
    them. It contributes testability to other extensions' kinds through
    enhancements, so the owners stay test-agnostic. Runner extensions
    (@specforge/cargo-test, @specforge/vitest) build on it.
  """
}

feature te_coverage_analysis "Coverage Analysis" {
  behaviors [te_coverage_pass, te_coverage_gate]
  problem   """
    Knowing that a behavior declares verify obligations is not enough:
    teams need to see which obligations are backed by passing tests,
    which by failing ones and which by nothing, and to gate CI on it.
  """
  solution  """
    @specforge/testing's coverage pass scores intent (obligations),
    enforcement (referenced invariants) and proof (the test results
    `specforge collect` recorded, plus entailed formal claims).
    `specforge analyze coverage` reports it, and `--min` gates on the
    share of testable entities proven.
  """
}
