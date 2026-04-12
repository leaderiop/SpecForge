// @specforge/coverage extension invariants

use "behaviors/validation"
invariant traceability_chain_integrity "Traceability Chain Integrity" {
  guarantee """
    Every file-reference field (any extension-declared field with
    file_reference=true, e.g. gherkin) MUST reference an
    existing file. Every entity ID in a specforge-report.json MUST match
    a declared entity in the spec graph. No broken links MUST exist in
    the intent -> linkage -> proof traceability chain. The compiler MUST
    detect and report all gaps.
  """
  risk high

  verify property "every file-reference field references an existing file"
  verify unit "broken traceability links in the intent-linkage-proof chain are detected and reported"

}
