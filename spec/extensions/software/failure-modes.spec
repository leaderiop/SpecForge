// @specforge/software extension failure modes — FMEA risk analysis
//
// Failure modes specific to the software engineering entity model:
// formatting, traceability, and library dependency concerns.

use "invariants/core"
use "invariants/formatting"
use "extensions/software/invariants"
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

// ── Formal Analysis Failure Modes ─────────────────────────────

failure_mode false_positive_liskov_violation "False Positive Liskov Violation" {
  invariant  se_formal_contract_consistency
  severity   high
  occurrence occasional
  detection  unlikely
  rpn        72

  cause      "Contract check pass incorrectly flags a valid refinement as Liskov-violating because the postcondition comparison uses string equality instead of semantic implication — e.g., 'result is non-empty list' vs 'result contains at least one element'"
  effect     "Developer receives spurious E031 errors on valid refinements, loses trust in formal analysis, and disables the contract_check pass"
  mitigation "Postcondition comparison uses structured condition names (not prose descriptions) for implication checks; integration tests with semantically equivalent but textually different conditions; escape hatch via @suppress annotation"

  post_mitigation {
    severity   high
    occurrence rare
    detection  likely
    rpn        12
  }
  verify unit "False Positive Liskov Violation failure mode is handled"
}

failure_mode proof_obligation_leak "Proof Obligation Leak" {
  invariant  se_formal_contract_consistency
  severity   high
  occurrence unlikely
  detection  undetectable
  rpn        70

  cause      "Proof obligation pass fails to generate an obligation for a contract condition because the condition is in a maintains block that the pass does not traverse — maintains conditions are only checked for invariant_preservation but not for contract_preservation"
  effect     "A contract condition exists without a corresponding proof obligation — it appears verified when it has never been checked, creating a false sense of completeness"
  mitigation "Exhaustive obligation generation: every condition in requires, ensures, and maintains produces exactly one obligation; count assertion: total obligations >= total conditions across all contract blocks; integration test with maintains-only entities"

  post_mitigation {
    severity   high
    occurrence rare
    detection  likely
    rpn        14
  }
  verify unit "Proof Obligation Leak failure mode is handled"
}

failure_mode concurrency_analysis_timeout "Concurrency Analysis Timeout" {
  invariant  se_event_trigger_validity
  severity   medium
  occurrence occasional
  detection  likely
  rpn        30

  cause      "Large event graph with many sync barriers causes the process_analyze pass to exceed the 30-second timeout, producing an incomplete ConcurrencyAnalysisReport with timed_out=true"
  effect     "Some concurrency sub-analyses (deadlock, livelock, starvation) are omitted from the report — user sees partial results without clear indication of what was skipped"
  mitigation "Timeout warning includes explicit list of incomplete sub-analyses; timed_out field in ConcurrencyAnalysisReport enables programmatic detection; --timeout CLI flag allows increasing the limit for large projects; incremental analysis caches previous results"

  post_mitigation {
    severity   medium
    occurrence rare
    detection  certain
    rpn        5
  }
  verify unit "Concurrency Analysis Timeout failure mode is handled"
}
