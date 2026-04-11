// MCP Server types — Model Context Protocol descriptors and response shapes

use "types/core"
use "types/diagnostics"
use "types/formatting"
type JsonRpcErrorCode = -32700 | -32600 | -32601 | -32602 | -32603

type McpErrorCode = "invalid_input" | "compilation_failed" | "entity_not_found" | "file_not_found" | "extension_not_found" | "permission_denied" | "timeout" | "not_initialized" | "schema_mismatch" | "internal_error"

type JsonSchema "JSON Schema Object" {
  type           string
  properties     object          @optional
  required       string[]        @optional
  description    string          @optional
}

type McpError "MCP Structured Error Response" {
  code       McpErrorCode
  message    string
  entity_id  string @optional
  tool       string @optional
  timestamp  string
}

type McpCapabilities "MCP Server Capabilities" {
  tools             McpToolDescriptor[]
  resources         McpResourceDescriptor[]
  prompts           McpPromptDescriptor[]
  subscriptions     boolean
  server_name       string
  server_version    string
  graph_protocol_version string
}

type McpResourceDescriptor {
  uri            string          @readonly
  name           string
  description    string          @optional
  mime_type      string          @optional
}

type McpToolDescriptor {
  name           string          @readonly
  description    string
  input_schema   JsonSchema
  output_schema  JsonSchema      @optional
  category       McpToolCategory @optional
  /// "core" for built-in tools, extension name for contributed tools
  source         string          @optional
}

type McpSubscription {
  client_id      string          @readonly
  channel        string          @readonly
  subscribed_at  timestamp
}

// Tool categories: "management" corresponds to the "Project Management Tools"
// feature group (specforge.extensions, specforge.providers, specforge.doctor,
// specforge.collect, specforge.render). The short name is used in the protocol
// for brevity; UIs and documentation may display "project management".
type McpToolCategory = "core" | "navigation" | "mutation" | "management"

type McpPromptDescriptor {
  name           string          @readonly
  description    string
  arguments      McpPromptArgument[]
}

type McpPromptArgument {
  name           string          @readonly
  description    string
  required       boolean
}

type McpInspectResult {
  entity_id      string          @readonly
  kind           string          @readonly
  title          string
  source_extension string        @optional
  testable       boolean
  reference_count integer
  summary        string          @optional
  source_span    SourceSpan      @readonly
  contract        string          @optional
  fields          FieldMap        @optional
  references      string[]        @optional
  verify_declarations string[]    @optional
  coverage_status string          @optional
  diagnostics     Diagnostic[]    @optional
}

type McpDefinitionResult {
  entity_id      string          @readonly
  file_path      string          @readonly
  line           integer         @readonly
  column         integer         @readonly
}

type McpReferenceLocation {
  referencing_entity_id string   @readonly
  source_span    SourceSpan      @readonly
}

type McpReferenceResult {
  entity_id      string          @readonly
  locations      McpReferenceLocation[]
}

type McpOutlineEntry {
  entity_id      string          @readonly
  kind           string          @readonly
  title          string
  range          SourceSpan      @readonly
  children       McpOutlineEntry[] @optional
}

type McpFixSuggestion {
  title          string
  kind           string
  diagnostic_code string          @optional
  edits          TextEdit[]
}

type McpStatsResult {
  entity_counts  McpEntityCount[]
  coverage_pct   float           @optional
  edge_count     integer
  orphan_count   integer
  diagnostic_summary McpDiagnosticSummary
}

type McpEntityCount {
  kind           string          @readonly
  count          integer
}

type McpDiagnosticSummary {
  errors         integer
  warnings       integer
  infos          integer
}

type McpExtensionInfo {
  name           string          @readonly
  version        string          @readonly
  entity_kinds   string[]
  contribution_types string[]
  status         string          @optional
}

type McpProviderInfo {
  scheme         string          @readonly
  alias          string          @optional
  extension      string          @readonly
  status         string
}

type McpDoctorFinding {
  check          string
  status         "ok" | "warn" | "error"
  code           string
  remediation    string          @optional
}

type McpDoctorReport {
  extensions_ok  boolean
  conflicts      string[]
  cache_status   string
  findings       McpDoctorFinding[]
}

type McpInitResult {
  project_path   string          @readonly
  config_file    string          @readonly
  starter_file   string          @readonly
  extensions_installed string[]  @optional
}

type McpFormatResult {
  changed_files  string[]
  total_checked  integer
  all_clean      boolean
  diffs          FormatDiff[]    @optional
}

type McpSearchResult {
  entity_id      string          @readonly
  kind           string          @readonly
  title          string
  file_path      string          @readonly
  line           integer         @readonly
  match_field    string          @optional
  match_snippet  string          @optional
}

type CoverageStatus = "covered" | "uncovered" | "partial"

// P2 compliance: CoverageStatus is a structural computation, not domain vocabulary.
// The three states map to a boolean triple computable without extension input:
//   covered   = has_verify_declarations AND has_evidence_collected
//   partial   = has_verify_declarations AND NOT has_evidence_collected
//   uncovered = NOT has_verify_declarations
// Extensions may overlay domain-specific labels (pass/fail, conformant/non-conformant)
// via extension-contributed metadata fields on their entity kinds.
type McpCoverageResult {
  entity_id      string          @readonly
  kind           string          @readonly
  status         CoverageStatus
  declared       boolean
  linked         boolean
  /// Whether evidence has been collected from an external report (test results, audit findings, review logs).
  evidence_collected boolean
}

type McpRenameResult {
  old_name       string          @readonly
  new_name       string          @readonly
  affected_files integer
  edits          TextEdit[]
}

type McpTracePlanResult "Trace tool response when plan parameter is provided" {
  /// Entities in the graph that are affected by the plan.
  affected_entities  string[]
  /// Traceability gaps between plan items and the graph.
  gaps               McpTraceGap[]
  /// Coverage status per affected entity.
  coverage_summary   McpCoverageResult[] @optional
}

type McpCollectResult {
  report_path    string          @readonly
  items_found    integer
  entities_mapped integer
}

type McpRenderResult {
  format         string          @readonly
  output_files   string[]
}

type McpRemoveExtensionResult {
  /// Result of removing an extension via MCP.
  removed_extension string
  orphan_warnings   string[]
  success           boolean
}

// The implement prompt provides structured context for agents, NOT generated
// code or instructions. Per vision: "SpecForge provides context, agents
// produce output." The structural_constraints field contains graph-derived structural
// hints (invariants to satisfy, edge constraints, field expectations) — never
// implementation directives or code suggestions.
type McpContextPromptResult "Context Prompt Result" {
  entity_id          string          @readonly
  kind               string          @readonly
  contract_text      string
  upstream_entities  string[]
  downstream_entities string[]
  verify_expectations string[]
  // Graph-derived structural constraints: invariants, edge constraints, field
  // expectations — NOT implementation directives or code suggestions.
  structural_constraints string[]    @optional
  affected_entities  string[]        @optional
}

type McpReviewPromptResult "Review Prompt Result" {
  entity_id          string          @readonly
  findings           McpReviewFinding[]
  coverage_summary   McpCoverageResult[] @optional
}

type McpReviewFinding {
  entity_id          string          @readonly
  severity           string
  message            string
  gap_context        string          @optional
}

type McpTracePromptResult "Trace Prompt Result" {
  /// Gaps in traceability between plan items and graph entities.
  coverage_gaps      McpTraceGap[]
  /// Entity IDs that have no verify declarations, no linked evidence, and no collected evidence.
  unverified_entities  string[]
  /// Entity IDs that the plan touches directly or transitively via graph edges.
  affected_entities  string[]
}

type McpTraceGap {
  source_entity      string          @readonly
  target_entity      string          @readonly
  missing_link_type  string
  gap_context        string          @optional
}

type McpExplorePromptResult "Explore Prompt Result" {
  /// Entity IDs matching the explore query filters (entity_id starting point
  /// and/or kind filter). When no filters are provided, contains all entities.
  matching_entities  string[]
  relationship_paths McpRelationshipPath[]
  /// Suggested entity IDs for agents to begin exploring — typically
  /// root-level entities (high out-degree, low in-degree).
  starting_points    string[]
  /// Entity IDs with the highest edge counts (in-degree + out-degree), useful
  /// for understanding the most interconnected parts of the graph.
  high_connectivity  string[]
  /// Entity IDs with no incoming or outgoing edges — candidates for cleanup or
  /// missing references.
  orphan_nodes       string[]
}

type McpRelationshipPath {
  from_entity        string          @readonly
  to_entity          string          @readonly
  edge_types         string[]
  path_length        integer
}
