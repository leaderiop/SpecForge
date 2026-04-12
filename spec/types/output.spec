// Output types for rendering and emitting

type OutputFile {
  path       string   @readonly
  content    string   @readonly
  checksum   string   @readonly
  format     OutputFormat @readonly @optional
  verify unit "OutputFile schema is valid"
}

type OutputFormat = "json" | "dot" | "context" | "brief" | "graph"

type DiagnosticFormat = "text" | "json"

type AgentExportConfig {
  format     OutputFormat  @readonly
  scope      string        @readonly @optional
  depth      integer       @readonly @optional
  kind_filter string[]     @readonly @optional
  max_tokens integer       @readonly @optional
  strategy   string        @readonly @optional // default: "prioritize"
  centrality_metric string @readonly @optional // default: "degree"
  verify unit "AgentExportConfig schema is valid"
}

// ── Token Economics ───────────────────────────────────────

type TokenBudgetStrategy = "truncate" | "prioritize" | "error"

type TokenBudgetResult {
  estimated_tokens   integer
  within_budget      boolean
  /// Present when budget is applied and strategy=truncate; empty array when no truncation.
  truncated_entities string[]       @optional
  strategy           TokenBudgetStrategy
  verify unit "TokenBudgetResult schema is valid"
}

// Entity embedding types moved to spec/extensions/embeddings/types.spec

type PlanValidationResult "Agent Plan Validation Result" {
  valid_entries     integer
  invalid_entries   integer
  gap_count         integer
  ordering_violations integer
  details           PlanValidationEntry[]

  verify unit "Agent Plan Validation Result conforms to schema"
}

type PlanValidationEntry "Single Plan Entry Validation" {
  entity_id    string
  status       "valid" | "invalid" | "not_found"
  reason       string @optional

  verify unit "Single Plan Entry Validation conforms to schema"
}

type EntityKindCount "Entity count for a single kind" {
  kind   string   @readonly
  count  integer  @readonly

  verify unit "Entity count for a single kind conforms to schema"
}

type ProjectStatistics "Project-Level Statistics" {
  entity_count_by_kind  EntityKindCount[]
  coverage_percentage   float
  orphan_count          integer
  diagnostic_summary    DiagnosticSummary
  testable_entity_count integer
  verified_entity_count   integer

  verify unit "Project-Level Statistics conforms to schema"
}

type DiagnosticSummary "Diagnostic Count Summary" {
  error_count   integer
  warn_count    integer
  info_count    integer

  verify unit "Diagnostic Count Summary conforms to schema"
}

// ── Graph Annotations ──────────────────────────────────────
// Graph Protocol nodes MAY carry annotations prefixed with _ (underscore).
// Annotations are metadata computed by the compiler or extensions, not
// declared in .spec files. They are included in export output for agent
// consumption.

type GraphAnnotation {
  key        string   @readonly // Always starts with "_" (e.g., "_similarity")
  value_type string   @readonly // JSON type: "number", "string", "boolean", "object"
  source     string   @readonly // "core" or extension name that produced it
  verify unit "GraphAnnotation schema is valid"
}

// ── Schema Cache ───────────────────────────────────────────

type SchemaCacheEntry {
  schema_version SchemaVersion @readonly
  hash           string   @readonly // SHA256 of the serialized schema JSON
  cached_at      string   @readonly // ISO 8601 timestamp
  verify unit "SchemaCacheEntry schema is valid"
}

type ExportResult {
  format OutputFormat
  entity_count integer
  edge_count integer
  // semver format: "MAJOR.MINOR.PATCH"
  schema_version string @optional
  token_budget_applied boolean @optional
  truncated_entities string[] @optional
  verify unit "ExportResult schema is valid"
}
