// Error reporting behaviors — diagnostic formatting and UX

use invariants/core
use invariants/validation
use types/diagnostics
use types/core
use events/compilation
use invariants/zero-entity-core
use types/zero-entity-core

behavior format_diagnostics_with_source_context "Format Diagnostics with Source Context" {
  // Runs inline during resolution pass, not event-driven
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  types      [Diagnostic, SourceSpan, CodePrefix]

  contract """
    Every diagnostic MUST include the source file path, line number,
    column number, and a context snippet showing the offending line
    with a caret pointing to the exact position. Multi-line spans
    MUST show the full range.
  """

  requires {
    valid_source_span "Diagnostic carries a valid SourceSpan referencing an accessible source file"
  }

  ensures {
    header_present "Output includes file:line:col header"
    context_snippet_present "Output includes source context snippet with the offending line"
    caret_marker_present "Output includes caret marker pointing to the exact position"
  }

  verify unit "diagnostic shows file:line:col"
  verify unit "context snippet highlights offending token"
  verify unit "multi-line span shows full range"
  verify contract "requires/ensures consistency for diagnostic source context formatting"

}

behavior provide_did_you_mean_suggestions "Provide Did-You-Mean Suggestions" {
  // Runs inline during resolution pass, not event-driven
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [Diagnostic, EntityId]

  contract """
    When an entity ID reference is unresolvable, the compiler MUST
    compute edit distance against all known entity IDs and SHOULD
    suggest the closest match if the distance is within a threshold
    (Levenshtein distance <= 3). Suggestions MUST appear in the
    diagnostic's help text.
  """

  requires {
    unresolved_reference_available "Unresolved reference with entity ID is available"
    kind_registry_populated "KindRegistry is populated with at least one entity kind"
  }

  ensures {
    distance_threshold "Suggestions have Levenshtein distance <= 3 from the unresolved ID"
    sorted_by_distance "Suggestions are sorted by ascending edit distance"
  }

  verify unit "close match produces suggestion"
  verify unit "distant match produces no suggestion"
  verify unit "suggestion appears in help text"
  verify contract "requires/ensures consistency for did-you-mean suggestions"

}

behavior aggregate_diagnostic_summary "Aggregate Diagnostic Summary" {
  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]
  types      [DiagnosticBag]
  consumes  [declarative_validation_executed]
  produces   [validation_complete]

  requires {
    validation_executed "declarative_validation_executed event has fired, confirming all structural and declarative validators have completed"
  }

  ensures {
    counts_match "Summary counts (errors, warnings, info) exactly match the number of diagnostics in the DiagnosticBag"
  }

  contract """
    After all diagnostics are printed, the compiler MUST print a summary
    line: "N errors, M warnings, K info". If errors exist, the summary
    MUST be visually distinguished as an error. The summary MUST match the actual counts.
  """

  verify unit "summary shows correct counts"
  verify unit "summary is red when errors exist"
  verify unit "summary matches actual diagnostics"
  verify contract "requires/ensures consistency for diagnostic summary aggregation"

}
