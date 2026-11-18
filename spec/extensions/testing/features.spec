requirement FEATURE-127 "Runner-Agnostic Test Vocabulary" {
  category "Feature"
  tier     v2
  text     """
    Test concepts were spread across core and @specforge/software:
    testability flags on software's kinds, W004/W009 in its rules, and
    allowlists that disagreed with the kinds' own verify kinds. A project
    could not use software's vocabulary without also taking its opinion
    of what must be tested, and test runners had nowhere to plug in.

    @specforge/testing owns the test vocabulary (ADR 0002): which kinds
    accept verify obligations and of which kinds, and the rules over
    them. It contributes testability to other extensions' kinds through
    enhancements, so the owners stay test-agnostic. Runner extensions
    (@specforge/cargo-test, @specforge/vitest) build on it.
  """
}

requirement FEATURE-126 "Coverage Analysis" {
  category "Feature"
  tier     v2
  text     """
    Knowing that a behavior declares verify obligations is not enough:
    teams need to see which obligations are backed by passing tests,
    which by failing ones and which by nothing, and to gate CI on it.

    @specforge/testing's coverage pass scores intent (obligations),
    enforcement (referenced invariants) and proof (the test results
    `specforge collect` recorded, plus entailed formal claims).
    `specforge analyze coverage` reports it, and `--min` gates on the
    share of testable entities proven.
  """
}
