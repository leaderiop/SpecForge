// Graph types — in-memory graph data shapes
//
// Node is the graph-level representation of an Entity. It has the same
// fields as Entity (from types/core) because the graph stores the full
// entity data at each node. Entity is the parser output; Node is the
// graph node. They are structurally identical but semantically distinct:
// Entity belongs to a SpecFile, Node belongs to a Graph.

use "types/core"
use "types/diagnostics"
type Graph {
  schema_version string    @readonly
  nodes      Node[]
  edges      Edge[]
  file_index FileIndex
}

type Node {
  id         EntityId    @readonly  @unique
  kind       EntityKind
  title      string
  fields     FieldMap
  source_span SourceSpan  @readonly
}

type Edge {
  source     EntityId    @readonly
  target     EntityId    @readonly
  edge_type  EdgeType
}

// EdgeType is an open string — not a closed enum.
// Extensions declare edge types via edgeTypes and field-to-edge mappings
// in their manifests. The core only knows edge labels are strings.
type EdgeType {
  label      string     @readonly
}

type FileIndex {
  files      FileEntry[]
}

type FileEntry {
  path       string     @readonly
  entities   EntityId[]
  imports    string[]
}

type Subgraph {
  node_ids   EntityId[]
  edges      Edge[]
}

type TraceChain {
  root       EntityId
  links      TraceLink[]
}

type TraceLinkStatus = "resolved" | "missing"

type TraceLink {
  from       EntityId
  to         EntityId
  edge_type  EdgeType
  depth      integer
  status     TraceLinkStatus
}

// ── Graph Protocol Schema Types ──────────────────────────────

type GraphProtocolSchema {
  schema_version  SchemaVersion   @readonly
  extensions      SchemaExtensionInfo[]
  entity_kinds    SchemaEntityKind[]
  edge_types      SchemaEdgeType[]
}

type SchemaExtensionInfo {
  name            string          @readonly
  version         string          @readonly
}

type SchemaEntityKind {
  name            string          @readonly
  extension       string          @readonly
  testable        boolean
  singleton       boolean
  fields          SchemaField[]
  has_body_parser boolean         @optional
  grammar_extension string        @optional
}

type SchemaField {
  name            string          @readonly
  field_type      string          @readonly
  required        boolean
  target_kind     string          @optional
  enum_values     string[]        @optional
}

type SchemaEdgeType {
  label           string          @readonly
  source_kinds    string[]        @optional
  target_kinds    string[]        @optional
  extension       string          @readonly
}

// ── Graph Protocol Versioning ──────────────────────────────

type SchemaVersion {
  major          integer         @readonly
  minor          integer         @readonly
  patch          integer         @readonly
  label          string          @optional @readonly
}

type SchemaMigration {
  from_version   SchemaVersion   @readonly
  to_version     SchemaVersion   @readonly
  breaking       boolean
  changes        SchemaMigrationChange[] @optional
}

type SchemaMigrationChange {
  change_type   string          @readonly // "kind_added" | "kind_removed" | "edge_added" | "edge_removed" | "field_added" | "field_removed"
  entity_kind   string          @optional
  edge_type     string          @optional
  field_name    string          @optional
  breaking      boolean
}

type SchemaCompatibility {
  requested      SchemaVersion   @readonly
  supported_min  SchemaVersion   @readonly
  supported_max  SchemaVersion   @readonly
  compatible     boolean
  // Sourced from CompilerConfig.supported_schema_min / .supported_schema_max (see ADR graph_protocol_version_management)
  source         string          @readonly
}

// ── Graph Delta Types ────────────────────────────────────────

// GraphDelta is used for incremental compilation and MCP delta notifications.
// It is NOT part of the stable Graph Protocol schema (not included in
// embed_schema_in_export output). Its structure may evolve between compiler
// versions without requiring a Graph Protocol major version bump.
// Arrays MUST be sorted by EntityId.raw (lexicographic) for deterministic output
type GraphDelta {
  timestamp       timestamp       @readonly
  added_nodes     NodeChange[]
  removed_nodes   NodeChange[]
  modified_nodes  ModifiedNodeChange[]
  added_edges     Edge[]
  removed_edges   Edge[]
  affected_files  string[]
}

type NodeChange {
  id              string          @readonly
  kind            string          @readonly
  file            string          @optional
  line            integer         @optional
}

type ModifiedNodeChange {
  id              string          @readonly
  changed_fields  string[]
  // old_value and new_value are populated when delta_include_values is true
  // in CompilerConfig (default false for token efficiency per P3), or always
  // in debug mode (debug build configuration / --verify-incremental).
  old_value       JsonValue       @optional
  new_value       JsonValue       @optional
  file            string          @optional
  line            integer         @optional
}

type DiagnosticsDelta {
  added           Diagnostic[]
  removed         Diagnostic[]
}

// ── Agent Plan Types ──────────────────────────────────────

type AgentPlan {
  plan_id        string          @readonly @unique
  entries        AgentPlanEntry[]
}

type AgentPlanEntry {
  entity_id      string          @readonly
  action         string
  dependencies   string[]        @optional
}
