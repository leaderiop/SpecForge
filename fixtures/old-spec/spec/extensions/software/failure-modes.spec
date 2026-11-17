// @specforge/software extension failure modes — FMEA risk analysis
//
// Failure modes specific to the software engineering entity model:
// formatting, traceability, and library dependency concerns.

use "extensions/software/invariants"
use "invariants/core"
use "invariants/formatting"

failure_mode formatting_idempotency_violation "Formatting Idempotency Violation" {
  invariant  formatting_idempotency
  severity   high
  occurrence occasional
  detection  moderate
  rpn        63
  cause      "Bug in alignment or wrapping rules causes the formatter to oscillate between two states — e.g., a reference list that alternates between inline and multi-line on successive runs"
  effect     "specforge format --check flakes in CI — developers cannot achieve clean formatting, losing trust in the tool"
  mitigation "Property-based tests with random valid .spec files verify format(format(x)) == format(x); regression test for every reported violation; alignment rules use stable column computation"
  post_mitigation {
    severity   high
    occurrence rare
    detection  certain
    rpn        7
  }
  verify unit "Formatting Idempotency Violation failure mode is handled"
}

failure_mode comment_loss_during_formatting "Comment Loss During Formatting" {
  invariant  comment_preservation
  severity   critical
  occurrence unlikely
  detection  unlikely
  rpn        64
  cause      "Comment attachment algorithm fails on edge cases — e.g., comment between closing brace and next block, or comment inside an empty block body"
  effect     "User loses documentation comments after running the formatter — silent data loss that may go unnoticed until much later"
  mitigation "Comment count assertion: formatted output must contain the same number of comment tokens as input; fuzzing with comment-heavy .spec files; diff review mode shows comment changes"
  post_mitigation {
    severity   critical
    occurrence rare
    detection  likely
    rpn        16
  }
  verify unit "Comment Loss During Formatting failure mode is handled"
}

failure_mode traceability_gap_undetected "Traceability Gap Undetected" {
  invariant  traceability_chain_integrity
  severity   high
  occurrence occasional
  detection  unlikely
  rpn        84
  cause      "A testable entity lacks test linkage or test execution proof but specforge trace fails to flag it — e.g., a missing branch in the coverage level computation"
  effect     "Team believes spec is fully covered when gaps exist — untested behaviors ship to production without detection"
  mitigation "Four-level coverage model (declared/linked/executed/passing) catches gaps at each layer; integration tests with deliberately incomplete traceability chains"
  post_mitigation {
    severity   high
    occurrence rare
    detection  likely
    rpn        14
  }
  verify unit "Traceability Gap Undetected failure mode is handled"
}
