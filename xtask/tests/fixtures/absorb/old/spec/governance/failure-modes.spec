// Failure modes: one with every rating, a post_mitigation block, an invariant, a description, refs and an
// assessment; one without occurrence or detection.

use "behaviors/b"
use "governance/invariants"

failure_mode torn_write "Torn Write" {
  invariant          writes_are_durable
  severity           critical
  occurrence         occasional
  detection          unlikely
  rpn                350
  cause              "The process dies between two writes of one entry."
  effect             "Half an entry is read back."
  mitigation         "Write to a temporary file, then rename it."
  post_mitigation {
    severity   high
    occurrence rare
    detection  likely
    rpn        12
  }
  affected_behaviors [write_entry]
  description        "The classic crash between write and rename."
  refs               ["FMEA worksheet 3"]
  risk               "high"
  verify unit "a killed writer leaves the old entry"
}

failure_mode slow_disk "Slow Disk" {
  invariant entries_are_whole
  severity  low
  cause     "The disk is shared with a backup."
  effect    "Writes queue up."
}
