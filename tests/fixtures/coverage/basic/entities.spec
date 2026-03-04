behavior validate_data "Validate Data" {
  contract """The system MUST validate incoming data before processing."""
  verify unit "rejects empty input"
  verify unit "accepts valid input"
  tests "tests/validate_data.test.ts"
}

behavior transform_data "Transform Data" {
  contract """The system MUST transform data according to rules."""
  verify unit "applies transform correctly"
}

invariant data_integrity "Data Integrity" {
  guarantee """Data MUST remain consistent across operations."""
  enforced_by [validate_data]
}

feature data_pipeline "Data Pipeline" {
  problem """Need reliable data processing."""
  solution """Build a validation and transformation pipeline."""
  behaviors [validate_data, transform_data]
}

type data_record "Data Record" {
  fields {
    id string @readonly
    name string
    value integer
  }
}
