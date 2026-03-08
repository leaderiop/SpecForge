// Core domain types — fundamental data shapes

type SpecFile {
  path       string    @readonly
  imports    ImportDeclaration[]
  entities   Entity[]
  errors     ParseError[]  @optional
}

type Entity {
  id         EntityId  @readonly  @unique
  kind       EntityKind
  // title is a grammar-level positional element (the string after keyword + id),
  // NOT a FieldRegistry entry. It is parsed by the generic_entity_block rule
  // and stored here for display, but does not participate in field validation.
  title      string
  fields     FieldMap
  source_span SourceSpan  @readonly
}

// EntityId uniqueness is on the raw string alone — two entities with
// different kinds but the same raw ID are forbidden. See entity_id_uniqueness
// invariant in invariants/core.spec.
type EntityId {
  kind       EntityKind @readonly
  raw        string     @readonly  @unique
}

// EntityKind is an open string — not a closed enum.
// All domain keywords come from extensions via KindRegistry.
// Grammar-level constructs (use, define) are not entity kinds.
//
// Structural node types (parsed by dedicated grammar rules, not KindRegistry):
//   - spec: singleton project root, creates a graph node
//   - ref: external reference, creates a graph node for orphan detection (W012)
// These are NOT in the KindRegistry — the KindRegistry is exclusively for
// extension-defined domain entity kinds. Structural node types have hardcoded
// parsing and validation in core.
// See zero-entity-core architecture.
type EntityKind {
  raw        string     @readonly
}

type FieldMap {
  entries    FieldEntry[]
}

type FieldEntry {
  key        string
  value      FieldValue
}

// Note: integer, bool, and enum field values are parsed as StringValue
// by the generic tree-sitter grammar. Type coercion and validation happen
// in the semantic phase using FieldRegistry type declarations.
type FieldValue = StringValue | ReferenceList | StringList | Block | VerifyList

type StringValue {
  _tag       "StringValue"    @literal
  content    string
}

type ReferenceList {
  _tag       "ReferenceList"  @literal
  ids        EntityId[]
}

type StringList {
  _tag       "StringList"     @literal
  items      string[]
}

type Block {
  _tag       "Block"          @literal
  entries    FieldEntry[]
}

type VerifyList {
  _tag       "VerifyList"     @literal
  items      VerifyStatement[]
}

type VerifyStatement {
  kind       VerifyKind
  description string
}

// VerifyKind is an open string — not a closed enum.
// Extensions declare allowedVerifyKinds per entity kind in their manifest.
// The core compiler does not define any built-in verify kinds.
type VerifyKind {
  raw        string     @readonly
}

// Opaque JSON value for Wasm host function I/O.
// Serialized as serde_json::Value in Rust.
type JsonValue {
  _tag       "JsonValue"     @literal
  raw        string
}

type SourceSpan {
  file       string    @readonly
  start_line integer   @readonly
  start_col  integer   @readonly
  end_line   integer   @readonly
  end_col    integer   @readonly
}

type TextEdit {
  file_path  string    @readonly
  range      SourceSpan
  new_text   string
}

type ImportDeclaration {
  path           string
  selected_ids   EntityId[]      @optional
}

type timestamp = string

type JsonObject = string // Serialized JSON object; used for MCP arguments
