// @specforge/compliance extension behaviors

use "extensions/compliance/manifest"
use "extensions/compliance/types"
behavior ce_validate_compliance_graph "Validate Compliance Graph" {
  types [ComplianceRegulation, ComplianceControl, ComplianceEvidence, ComplianceAudit]

  category   validation
  contract """
    The @specforge/compliance extension MUST validate the regulation -> control
    -> evidence traceability chain. Every regulation MUST have at least one
    control (W102). Every control MUST be governed by at least one regulation
    (W101). Controls with maturity 'implemented' or higher MUST have supporting
    evidence (W103). Expired evidence MUST produce a warning (W104). Validation
    MUST be implemented as declarative patterns in the extension manifest,
    executed by the core's execute_validation_pattern engine.
  """

  verify unit "regulation without controls produces W102"
  verify unit "orphan control produces W101"
  verify unit "implemented control without evidence produces W103"
  verify unit "expired evidence produces W104"
  verify unit "valid compliance chain passes without warnings"

}

behavior ce_render_compliance_matrix "Render Compliance Matrix" {
  types [ComplianceRegulation, ComplianceControl, ComplianceEvidence]

  category   query
  contract """
    The @specforge/compliance extension MUST provide a renderer contribution
    that produces a regulation-to-control traceability matrix. The matrix
    MUST list each regulation as a row, each control as a column, and indicate
    the maturity level at each intersection. Controls without evidence MUST
    be highlighted. The renderer MUST use the specforge.emit_file host function
    to write the matrix as a non-code output (HTML or Markdown table).
  """

  verify unit "matrix lists all regulations as rows"
  verify unit "matrix lists all controls as columns"
  verify unit "maturity level shown at intersections"
  verify unit "controls without evidence highlighted"
  verify unit "output written via emit_file host function"

}
