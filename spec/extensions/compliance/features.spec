// @specforge/compliance extension features

use "extensions/compliance/behaviors"
feature compliance_validation "Compliance Validation" {
  behaviors [ce_validate_compliance_graph]

  problem """
    Regulatory compliance specifications (SOC 2, HIPAA, GDPR, ISO 27001)
    require traceability from regulations through controls to evidence.
    Without automated validation, compliance gaps — orphan controls,
    unimplemented regulations, missing evidence — go undetected until
    audit time.
  """

  solution """
    Declarative validation rules in the @specforge/compliance extension
    manifest. The core engine's execute_validation_pattern behavior interprets
    these rules to detect orphan controls (W101), regulations without controls
    (W102), controls without evidence (W103), and expired evidence (W104).
    Validation runs automatically during specforge check.
  """
}

feature compliance_reporting "Compliance Reporting" {
  behaviors [ce_render_compliance_matrix]

  problem """
    Compliance auditors need a visual traceability matrix mapping regulations
    to controls with maturity status and evidence coverage. Producing this
    manually is error-prone and always stale.
  """

  solution """
    A renderer contribution in the @specforge/compliance extension produces
    a regulation-to-control traceability matrix via the emit_file host
    function. The matrix shows maturity levels and highlights gaps, providing
    auditors with an always-current compliance view generated directly from
    the spec graph.
  """
}
