// The method's own vocabulary.

use "product/roadmap"

term phase "phase" {
  definition "One step of a milestone with a goal and success criteria, planned and verified on its own."
  module     links
  see_also   [verify_obligation]
}

term requirement "requirement" {
  definition "A testable statement of what the system must do, written as a behavior's contract."
  module     links
  see_also   [verify_obligation]
}

term verify_obligation "verify obligation" {
  definition "A `verify` line on a behavior or invariant: a check that a named test must pass before the work counts as done."
  module     links
}
