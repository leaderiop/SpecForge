// @specforge/compliance extension field type definitions
//
// These types describe the fields declared in the extension manifest's
// entityKinds entries. They are not standalone AST types — they document
// the field schemas that the manifest registers into the FieldRegistry.

type ComplianceRegulation {
  jurisdiction    string
  effective_date  string          @optional
  controls        string[]
}

type ComplianceControl {
  category        string
  maturity        string
  evidence        string[]
}

type ComplianceEvidence {
  evidence_type   string
  collected_date  string          @optional
  expires         string          @optional
}

type ComplianceAudit {
  audit_type      string
  scope           string[]
  findings        string          @optional
}
