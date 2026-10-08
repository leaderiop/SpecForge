// MCP Server types — Model Context Protocol descriptors and response shapes

use "types/core"
use "types/diagnostics"
use "types/formatting"

/// -32002: resource not found, in the handshake revisions (2025-03-26 to
/// 2025-11-25); the 2026-07-28 revision answers -32602.
type JsonRpcErrorCode = -32700 | -32600 | -32601 | -32602 | -32603 | -32002

type McpErrorCode = "invalid_input"
  | "compilation_failed"
  | "entity_not_found"
  | "file_not_found"
  | "extension_not_found"
  | "permission_denied"
  | "timeout"
  | "not_initialized"
  | "schema_mismatch"
  | "internal_error"
  | "conflict"
  | "precondition_failed"

type JsonSchema "JSON Schema Object" {
  type        string
  properties  object   @optional
  required    string[] @optional
  description string   @optional
  verify unit "JSON Schema Object conforms to schema"
}

// A failed tool call is an isError result whose content is an McpError
// (MCP 2025-11-25, SEP-1303). It carries no timestamp: the same failing
// call returns the same error (mcp_tool_idempotency).
type McpError "MCP Structured Error Response" {
  code       McpErrorCode
  message    string
  entity_id  string     @optional
  /// The project file the failure is about (file_not_found).
  file       string     @optional
  tool       string     @optional
  /// The prompt that refused, for a prompts/get answered with an error.
  prompt     string     @optional
  /// The URI of the resource whose read failed (the URI read).
  uri        string     @optional
  /// The argument the tool could not use, for invalid_input.
  argument   string     @optional
  /// The diagnostic behind the failure: its code (E003, E059, ...) is
  /// diagnostic.code, never only text in the message.
  diagnostic Diagnostic @optional
  /// More about the failure: the renderers available, a failed run's report.
  data       object     @optional
  verify unit "MCP Structured Error Response conforms to schema"
}

type McpCapabilities "MCP Server Capabilities" {
  tools                  McpToolDescriptor[]
  resources              McpResourceDescriptor[]
  prompts                McpPromptDescriptor[]
  subscriptions          boolean
  server_name            string
  server_version         string
  graph_protocol_version string
  verify unit "MCP Server Capabilities conforms to schema"
}

type McpResourceDescriptor {
  uri         string @readonly
  name        string
  description string @optional
  mime_type   string @optional
  verify unit "McpResourceDescriptor schema is valid"
}

type McpToolDescriptor {
  name          string             @readonly
  description   string
  input_schema  JsonSchema
  output_schema JsonSchema         @optional
  /// The tool's role, never where it comes from.
  category      McpToolCategory    @optional
  /// "core" for built-in tools, extension name for contributed tools
  source        string             @optional
  /// MCP ToolAnnotations: what the tool does to its environment.
  annotations   McpToolAnnotations @optional
  verify unit "McpToolDescriptor schema is valid"
}

// MCP's ToolAnnotations (wire names readOnlyHint, destructiveHint,
// idempotentHint, openWorldHint). A core tool's derive from the same
// definition its mutation events do.
type McpToolAnnotations {
  read_only_hint   boolean @optional
  destructive_hint boolean @optional
  idempotent_hint  boolean @optional
  open_world_hint  boolean @optional
  verify unit "McpToolAnnotations schema is valid"
}

type McpSubscription {
  uri             string @readonly
  subscription_id string @readonly @optional
  verify unit "McpSubscription schema is valid"
}

// Tool categories: "management" corresponds to the "Project Management Tools"
// feature group (specforge.extensions, specforge.providers, specforge.doctor,
// specforge.collect, specforge.render). The short name is used in the protocol
// for brevity; UIs and documentation may display "project management".
type McpToolCategory = "core" | "navigation" | "mutation" | "management"

type McpPromptDescriptor {
  name        string @readonly
  description string
  arguments   McpPromptArgument[]
  verify unit "McpPromptDescriptor schema is valid"
}

type McpPromptArgument {
  name        string @readonly
  description string
  required    boolean
  verify unit "McpPromptArgument schema is valid"
}

type McpInspectResult {
  entity_id           string       @readonly
  kind                string       @readonly
  title               string
  /// The extension that declares the entity's kind; null when none does.
  source_extension    string       @optional
  /// The entity's kind is testable, the standing the LSP hover shows.
  testable            boolean
  /// The entity declares at least one verify obligation.
  declared            boolean
  /// Testable, but it owes no obligations and declares none: it does not
  /// count toward coverage (specforge.coverage's row says the same).
  exempt              boolean
  /// Its kind must declare obligations (a no_verify_statements rule targets
  /// it): why an exempt entity is exempt.
  obligated           boolean
  /// Deprecated: the number of edges in both directions.
  reference_count     integer
  source_span         SourceSpan   @readonly
  /// The field the entity's extension declares headline and normative (a
  /// behavior's contract); absent when its kind declares none.
  contract            string       @optional
  fields              FieldMap     @optional
  /// The entities that reference this one (incoming), distinct, sorted.
  referenced_by       string[]
  /// The entities this one references (outgoing), distinct, sorted.
  refers_to           string[]
  /// Deprecated: both directions, unlabeled. Use referenced_by and refers_to.
  references          string[]     @optional
  verify_declarations string[]     @optional
  coverage_status     string       @optional
  diagnostics         Diagnostic[] @optional
  verify unit "McpInspectResult schema is valid"
}

type McpDefinitionResult {
  entity_id   string     @readonly
  file_path   string     @readonly
  /// The position of the entity's name.
  line        integer    @readonly
  column      integer    @readonly
  /// The entity's block.
  source_span SourceSpan @readonly
  /// The entity's name as written; its block when the name could not be read.
  name_span   SourceSpan @readonly
  /// "token" when name_span is the name as written, else "entity".
  precision   string     @readonly
  verify unit "McpDefinitionResult schema is valid"
}

type McpReferenceLocation {
  referencing_entity_id string     @readonly
  referenced_entity_id  string     @readonly
  /// The field naming the referenced entity; absent for its declaration.
  field                 string     @optional
  /// "declaration" or "reference".
  role                  string
  /// "token" when source_span is the identifier as written, else "entity"
  /// (the text was unreadable or stale: the referencing entity's block).
  precision             string
  source_span           SourceSpan @readonly
  verify unit "McpReferenceLocation schema is valid"
}

type McpReferenceResult {
  entity_id string @readonly
  /// "incoming", "outgoing" or "both".
  direction string @readonly
  locations McpReferenceLocation[]
  verify unit "McpReferenceResult schema is valid"
}

type McpOutlineEntry {
  entity_id  string            @readonly
  kind       string            @readonly
  title      string
  /// The entity's (or method's) block.
  range      SourceSpan        @readonly
  /// Its name as written: what an editor selects.
  name_range SourceSpan        @readonly
  children   McpOutlineEntry[] @optional
  verify unit "McpOutlineEntry schema is valid"
}

type McpFixSuggestion {
  /// The LSP code action's title, e.g. "Replace with 'session_limit'".
  title           string
  /// "quickfix" or "refactor".
  kind            string
  diagnostic_code string @optional
  /// The entity the fix is about.
  entity_id       string @optional
  /// At least one: what applying the fix changes.
  edits           TextEdit[]
  verify unit "McpFixSuggestion schema is valid"
}

type McpStatsResult {
  entity_counts      McpEntityCount[]
  /// Deprecated alias of declared_pct.
  coverage_pct       float @optional
  /// Testable entities with at least one verify statement, in percent.
  declared_pct       float @optional
  /// Testable entities proven, in percent; null without recorded test results.
  proof_pct          float @optional
  edge_count         integer
  orphan_count       integer
  diagnostic_summary McpDiagnosticSummary
  verify unit "McpStatsResult schema is valid"
}

type McpEntityCount {
  kind  string @readonly
  count integer
  verify unit "McpEntityCount schema is valid"
}

type McpDiagnosticSummary {
  errors   integer
  warnings integer
  infos    integer
  verify unit "McpDiagnosticSummary schema is valid"
}

type McpExtensionInfo {
  name             string @readonly
  /// The loaded version, else the locked one; absent when neither.
  version          string @optional
  /// "builtin", the lock entry's source ("registry", "local:<path>"), or
  /// "file:<path>" for a .wasm file entry of specforge.json.
  source           string @readonly
  /// The entity kinds the extension contributes.
  entity_kinds     string[]
  /// The project's entities of those kinds.
  entity_count     integer
  validation_rules integer
  /// "loaded"; "not_loaded" (enabled, but not installed or it failed to
  /// load); "not_configured" (installed, but not enabled).
  status           "loaded" | "not_loaded" | "not_configured"
  verify unit "McpExtensionInfo schema is valid"
}

type McpProviderInfo {
  scheme    string @readonly
  alias     string @optional
  extension string @readonly
  status    string
  verify unit "McpProviderInfo schema is valid"
}

type McpDoctorFinding {
  check       string
  status      "ok" | "warn" | "error"
  code        string
  remediation string @optional
  verify unit "McpDoctorFinding schema is valid"
}

type McpDoctorReport {
  extensions_ok boolean
  conflicts     string[]
  cache_status  string
  findings      McpDoctorFinding[]
  verify unit "McpDoctorReport schema is valid"
}

type McpInitResult {
  project_path         string   @readonly
  config_file          string   @readonly
  starter_file         string   @readonly
  extensions_installed string[] @optional
  verify unit "McpInitResult schema is valid"
}

type McpFormatResult {
  changed_files string[]
  total_checked integer
  /// The verdict specforge format exits by (0 when true): every file read and
  /// written, no region left unformatted, and under check no file that would change.
  ok            boolean
  /// True only when every file was read and is in canonical form: no change,
  /// no failure, no region left unformatted.
  all_clean     boolean
  diffs         FormatDiff[] @optional
  /// The files the call could not read or write (the call failed).
  failed_files  string[]     @optional
  /// W141 per configuration file used, W142 per region kept verbatim.
  diagnostics   Diagnostic[] @optional
  verify unit "McpFormatResult schema is valid"
}

type McpSearchResult {
  entity_id     string  @readonly
  kind          string  @readonly
  title         string
  file_path     string  @readonly
  line          integer @readonly
  /// The rank band: 1.0 exact, 0.9 prefix, 0.8 substring, 0.7 field text,
  /// 0.6 × similarity for a fuzzy match.
  score         float
  /// What matched: "id", "title" or a string field's name; absent for an
  /// empty query.
  match_field   string  @optional
  /// For a field-text match, the field's text around the match.
  match_snippet string  @optional
  verify unit "McpSearchResult schema is valid"
}

type CoverageStatus = "covered" | "uncovered" | "partial"

// P2 compliance: CoverageStatus is a structural computation, not domain vocabulary.
// An obligation is proven when a passing recorded test names its verify text,
// the same rule `analyze coverage` applies (A015), computable without extension input:
//   covered   = every obligation proven AND no recorded test fails
//   partial   = not covered, AND some obligation proven OR a recorded test fails
//   uncovered = no obligation proven and no recorded test fails (incl. no verify declarations)
// Extensions may overlay domain-specific labels (pass/fail, conformant/non-conformant)
// via extension-contributed metadata fields on their entity kinds.
type McpCoverageResult {
  entity_id          string @readonly
  kind               string @readonly
  status             CoverageStatus
  declared           boolean
  linked             boolean
  /// Whether evidence has been collected from an external report (test results, audit findings, review logs).
  evidence_collected boolean
  /// The entity's verify obligations.
  obligations        integer
  /// Obligations a passing recorded test names.
  proven             integer
  /// Verify texts no passing recorded test names, in declaration order.
  unproven           string[]
  /// A testable-kind entity that owes no obligations and declares none (W004 exempts it).
  exempt             boolean
  verify unit "McpCoverageResult schema is valid"
}

type McpRenameResult {
  old_name       string @readonly
  new_name       string @readonly
  /// The files the rename edits, relative to the spec root.
  affected_files string[]
  edits          McpRenameEdit[]
  verify unit "McpRenameResult schema is valid"
}

type McpRenameEdit "One occurrence of the old identifier a rename replaces" {
  /// The file, relative to the spec root.
  file      string @readonly
  /// 1-based line.
  line      integer
  /// Byte columns of the occurrence on its line.
  start_col integer
  end_col   integer
  new_text  string
  verify unit "McpRenameEdit schema is valid"
}

type McpTracePlanResult "Trace tool response when plan parameter is provided" {
  /// Entities in the graph that are affected by the plan.
  affected_entities string[]
  /// Traceability gaps between plan items and the graph.
  gaps              McpTraceGap[]
  /// Coverage status per affected entity.
  coverage_summary  McpCoverageResult[] @optional
  verify unit "Trace tool response when plan parameter is provided conforms to schema"
}

type McpCollectResult {
  /// "collected".
  status      string @readonly
  /// One entry per runner whose results were collected.
  runners     McpCollectRunner[]
  /// What collecting reported (W115: a test names an unknown entity).
  diagnostics Diagnostic[]
  /// The path of the written specforge-report.json.
  report      string @readonly
  verify unit "McpCollectResult schema is valid"
}

type McpCollectRunner "What one runner's collection found" {
  /// The collector's name, e.g. cargo-test.
  name          string  @readonly
  /// The extension contributing the collector.
  extension     string  @readonly
  /// Whether the runner's command ran (run=true), or its report was read.
  ran           boolean
  exit_code     integer @optional
  /// Report files read.
  files         integer
  /// Entities the results name.
  entities      integer
  passed        integer
  failed        integer
  skipped       integer
  /// Tests linked by naming convention rather than by the report.
  by_convention integer
  verify unit "McpCollectRunner schema is valid"
}

type McpRenderResult {
  format       string @readonly
  output_files string[]
  verify unit "McpRenderResult schema is valid"
}

type McpRemoveExtensionResult {
  /// Result of removing an extension via MCP.
  removed_extension string
  orphan_warnings   string[]
  success           boolean
  verify unit "McpRemoveExtensionResult schema is valid"
}

// The implement prompt provides structured context for agents, NOT generated
// code or instructions. Per vision: "SpecForge provides context, agents
// produce output." The structural_constraints field contains graph-derived structural
// hints (invariants to satisfy, edge constraints, field expectations) — never
// implementation directives or code suggestions.
type McpContextPromptResult "Context Prompt Result" {
  entity_id              string   @readonly
  kind                   string   @readonly
  // The field the extension declares headline and normative (a behavior's
  // contract); empty when the entity's kind declares none.
  contract_text          string
  upstream_entities      string[]
  downstream_entities    string[]
  verify_expectations    string[]
  // Graph-derived structural constraints: invariants, edge constraints, field
  // expectations — NOT implementation directives or code suggestions.
  structural_constraints string[] @optional
  affected_entities      string[] @optional
  verify unit "Context Prompt Result conforms to schema"
}

type McpReviewPromptResult "Review Prompt Result" {
  entity_id        string              @readonly
  findings         McpReviewFinding[]
  coverage_summary McpCoverageResult[] @optional
  verify unit "Review Prompt Result conforms to schema"
}

type McpReviewFinding {
  entity_id   string @readonly
  severity    string
  message     string
  gap_context string @optional
  verify unit "McpReviewFinding schema is valid"
}

type McpTracePromptResult "Trace Prompt Result" {
  /// Plan gaps for a plan; the traced entities' missing links for an entity.
  coverage_gaps       McpTraceGap[]
  /// Entity IDs the trace reaches that count toward coverage and are not proven.
  unverified_entities string[]
  /// Entity IDs that the plan touches directly or transitively via graph edges.
  affected_entities   string[]
  verify unit "Trace Prompt Result conforms to schema"
}

type McpTraceGap {
  source_entity     string @readonly
  target_entity     string @readonly
  missing_link_type string
  gap_context       string @optional
  verify unit "McpTraceGap schema is valid"
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
  verify unit "Explore Prompt Result conforms to schema"
}

type McpRelationshipPath {
  from_entity string @readonly
  to_entity   string @readonly
  edge_types  string[]
  path_length integer
  verify unit "McpRelationshipPath schema is valid"
}
