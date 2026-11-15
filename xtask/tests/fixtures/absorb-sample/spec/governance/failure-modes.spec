// One failure mode of a seed invariant (joins); one of an outside invariant (stays out).

use "invariants/w"
use "behaviors/other"

failure_mode w_escape "Sandbox Escape" {
  invariant  w_isolated
  severity   critical
  occurrence rare
  detection  likely
  cause      "A host function trusts a guest pointer."
  effect     "A guest reads host memory."
}

failure_mode other_torn "Other Torn" {
  invariant  other_whole
  severity   low
  occurrence rare
  detection  likely
  cause      "Something else tears; a Chain breaks."
  effect     "Something else is half read."
}
