// Error reporting behaviors — diagnostic formatting and UX

use invariants/core
use invariants/validation
use types/diagnostics
use types/core

behavior format_diagnostics_with_source_context "Format Diagnostics with Source Context" {
  invariants [multi_error_collection, diagnostic_determinism]
  types      [Diagnostic, SourceSpan, CodePrefix]

  contract """
    Every diagnostic MUST include the source file path, line number,
    column number, and a context snippet showing the offending line
    with a caret pointing to the exact position. Multi-line spans
    MUST show the full range.
  """

  verify unit "diagnostic shows file:line:col"
  verify unit "context snippet highlights offending token"
  verify unit "multi-line span shows full range"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior provide_did_you_mean_suggestions "Provide Did-You-Mean Suggestions" {
  invariants [reference_resolution_completeness]
  types      [Diagnostic]

  contract """
    When an entity ID reference is unresolvable, the compiler MUST
    compute edit distance against all known entity IDs and SHOULD
    suggest the closest match if the distance is within a threshold
    (Levenshtein distance <= 3). Suggestions MUST appear in the
    diagnostic's help text.
  """

  verify unit "close match produces suggestion"
  verify unit "distant match produces no suggestion"
  verify unit "suggestion appears in help text"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior aggregate_diagnostic_summary "Aggregate Diagnostic Summary" {
  invariants [multi_error_collection]
  types      [DiagnosticBag]

  contract """
    After all diagnostics are printed, the compiler MUST print a summary
    line: "N errors, M warnings, K info". If errors exist, the summary
    MUST be styled in red. The summary MUST match the actual counts.
  """

  verify unit "summary shows correct counts"
  verify unit "summary is red when errors exist"
  verify unit "summary matches actual diagnostics"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
