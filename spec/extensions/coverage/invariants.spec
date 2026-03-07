// @specforge/coverage extension invariants

use extensions/coverage/behaviors
use behaviors/validation

invariant traceability_chain_integrity "Traceability Chain Integrity" {
  guarantee """
    Every file-reference field (gherkin paths, file paths in any
    extension-declared field with file_reference=true) MUST reference an
    existing file. Every entity ID in a specforge-report.json MUST match
    a declared entity in the spec graph. No broken links MUST exist in
    the intent -> linkage -> proof traceability chain. The compiler MUST
    detect and report all gaps.
  """
  enforced_by [validate_file_reference_paths, compute_traceability_chain, serialize_traceability_data, validate_test_ids_against_spec, consume_specforge_report, merge_coverage_reports, compute_coverage_summary, gate_on_coverage_threshold]
  risk high

  verify property "every file-reference field references an existing file"
  verify unit "broken traceability links in the intent-linkage-proof chain are detected and reported"

}
