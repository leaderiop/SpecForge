// Logical Data Model types — intermediate representation for schema visualization

type ModelFormat = "markdown" | "mermaid" | "dot" | "json" | "dbml"

type GroupBy = "extension" | "none"

type FieldLevel = "none" | "keys" | "all"

type Cardinality = "1:1" | "1:N" | "N:1" | "N:M"

type ModelOptions {
  format          ModelFormat   @readonly
  group_by        GroupBy       @readonly
  fields          FieldLevel    @readonly
  extension_filter string       @readonly @optional
  kind_filter     string[]      @readonly @optional
  root            string        @readonly @optional
  depth           integer       @readonly @optional
  verify unit "ModelOptions schema is valid"
}

type ModelFieldType = "string" | "integer" | "boolean" | "enum" | "string_list" | "reference" | "reference_list" | "block"

type ModelField {
  name            string         @readonly
  field_type      ModelFieldType @readonly
  required        boolean        @readonly
  description     string         @readonly @optional
  default_value   string         @readonly @optional
  enum_values     string[]       @readonly @optional
  is_primary_key  boolean        @readonly
  references      string         @readonly @optional // target entity name
  verify unit "ModelField schema is valid"
}

type ModelEntity {
  name            string         @readonly
  extension       string         @readonly
  description     string         @readonly @optional
  fields          ModelField[]   @readonly
  verify unit "ModelEntity schema is valid"
}

type ModelRelationship {
  name            string         @readonly
  source          string         @readonly
  target          string         @readonly
  cardinality     Cardinality    @readonly
  source_field    string         @readonly @optional
  description     string         @readonly @optional
  verify unit "ModelRelationship schema is valid"
}

type ModelExtension {
  name            string         @readonly
  version         string         @readonly
  entity_count    integer        @readonly
  edge_count      integer        @readonly
  verify unit "ModelExtension schema is valid"
}

type ModelIntermediate "ERD-oriented intermediate representation" {
  model_version   string              @readonly
  extensions      ModelExtension[]    @readonly
  entities        ModelEntity[]       @readonly
  relationships   ModelRelationship[] @readonly
  verify unit "ModelIntermediate schema is valid"
}
