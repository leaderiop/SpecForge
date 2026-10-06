// Wasm component extension runtime types
//
// Extension declarations use ExtensionDeclaration from types/zero-entity-core.spec.
// This file contains supporting types for the Wasm runtime: dependencies,
// host function bindings, sandbox policies, caching, enhancements, and queries.

use "types/core"

type PeerDependency {
  name     string
  // A semver range the installed version must satisfy
  version  string
  // An optional peer that is not installed is fine
  optional boolean @optional
  verify unit "PeerDependency schema is valid"
}

type HostFunctionBinding {
  name          string @readonly
  input_schema  string
  output_schema string
  verify unit "HostFunctionBinding schema is valid"
}

type SandboxPolicy {
  max_memory_mb             integer  @optional
  max_execution_ms          integer  @optional
  allowed_domains           string[] @optional
  allowed_paths             string[] @optional
  file_system_access        string   @optional
  network_access            string   @optional
  // Default: 5000. Per-request HTTP timeout in milliseconds.
  http_timeout_ms           integer  @optional
  // Default: 15000. Total HTTP time budget per compilation in milliseconds.
  http_total_budget_ms      integer  @optional
  // Default: [".json", ".html", ".csv", ".svg", ".dot", ".xml", ".txt", ".pdf"]
  // .md is NOT in the default list because SpecForge is not a documentation
  // generator (vision/README.md). Extensions that produce structured reports
  // (traceability matrices, coverage dashboards — not prose) MAY add .md to
  // their own sandbox policy via allowed_output_extensions override.
  allowed_output_extensions string[] @optional
  // Default: 1MB. Maximum file size readable via read_file host function.
  max_read_file_size        u64      @optional
  verify unit "SandboxPolicy schema is valid"
}

// ── Entity Enhancement Types ─────────────────────────────────

type DynamicEdgeType {
  label            string  @readonly
  source_extension string  @readonly
  soft             boolean @optional
  verify unit "DynamicEdgeType schema is valid"
}

// ── Extension Lifecycle Types ─────────────────────────────────

type ExtensionInstallResult {
  extension_name string @readonly
  version        string @readonly
  source         ExtensionSource
  wasm_size      integer
  installed_path string
  verify unit "ExtensionInstallResult schema is valid"
}

type ExtensionSource = registry | local | git

type WasmTrapInfo {
  kind           string @readonly
  message        string @readonly
  export_name    string @optional
  memory_address string @optional
  extension_name string
  verify unit "WasmTrapInfo schema is valid"
}

// ── Lock File Types ──────────────────────────────────────────

// TrustLevel is defined canonically in types/config.spec.

type LockFileEntry {
  extension_name string     @readonly
  version        string     @readonly
  source         ExtensionSource
  wasm_hash      string     @readonly
  resolved_at    string
  trust_level    TrustLevel @optional
  verify unit "LockFileEntry schema is valid"
}

// Serialized as JSON (specforge.lock). See P6: standard is the moat.
type LockFile {
  path             string  @readonly
  lockfile_version integer @readonly
  entries          LockFileEntry[]
  verify unit "LockFile schema is valid"
}

// ── Collector Contribution Types ────────────────────────────

type CollectorDescriptor {
  name          string              @readonly
  input_formats string[]
  auto_detect   CollectorAutoDetect @optional
  export        string
  run           string[]            @optional
  report        string              @optional
  capture       string              @optional
  verify unit "CollectorDescriptor schema is valid"
}

type CollectorAutoDetect {
  file_patterns string[]
  env_vars      string[] @optional
  verify unit "CollectorAutoDetect schema is valid"
}

type CollectorTestStatus = passed | failed | skipped

type ExtensionSpecifier "Parsed Extension Specifier" {
  raw     string
  format  ExtensionSource
  scope   string @optional
  name    string
  version string @optional
  path    string @optional
  git_ref string @optional
  verify unit "Parsed Extension Specifier conforms to schema"
}

// ── Extension Call Payloads ─────────────────────────────────
// What the host sends each export it calls on a loaded extension, and
// what the export answers: one specforge_protocol_types type each, shared
// by the host and the SDK (ADR 0013). An optional field the host leaves
// unset is absent from the wire, never null.

// What a collect__<name> export receives: the runner's report files and,
// when the collector captures it, the command's standard output.
type CollectInput {
  reports CollectReportFile[]
  stdout  string @optional
  verify unit "CollectInput schema is valid"
}

type CollectReportFile {
  path    string
  content string
  verify unit "CollectReportFile schema is valid"
}

// What a collect__<name> export answers: tests grouped by the entity each
// proves, and the tests the report links to none (the host links them by
// naming convention).
type CollectOutput {
  entity_results CollectEntityResult[]
  unlinked       CollectUnlinkedTest[] @optional
  verify unit "CollectOutput schema is valid"
}

type CollectEntityResult {
  entity_id    string
  test_results CollectTestResult[]
  verify unit "CollectEntityResult schema is valid"
}

type CollectTestResult {
  name        string
  status      CollectorTestStatus
  verify      string @optional
  duration_ms float  @optional
  verify unit "CollectTestResult schema is valid"
}

type CollectUnlinkedTest {
  name   string
  // The name's path segments, the test's own name last.
  path   string[]
  status CollectorTestStatus
  verify unit "CollectUnlinkedTest schema is valid"
}

// What a __pass_<name> export receives: the entity snapshot, the resolved
// references, and the recorded test results, the claims the prove pass
// entailed and (check-phase passes) the previous build's statuses when the
// host has them.
type PassInput {
  entities      PassEntity[]
  edges         PassEdge[]
  // The normalized specforge-report.json: per entity id, its tests.
  test_results  object   @optional
  proved_claims string[] @optional
  // The build cache's statuses: per entity id, its kind and status.
  previous      object   @optional
  verify unit "PassInput schema is valid"
}

// One entity of the snapshot. testable: its kind's flag; exempt: it owes
// no obligations of its own (ADR 0004, D2-b), decided by the host from the
// registries.
type PassEntity {
  id                  string
  kind                string
  // Its fields, stringified, by name.
  fields              object
  incoming_edge_count integer
  outgoing_edge_count integer
  span                SourceSpan @optional
  testable            bool
  exempt              bool
  verify_kinds        string[]
  verify_texts        string[]
  verify unit "PassEntity schema is valid"
}

type PassEdge {
  source string
  target string
  label  string
  verify unit "PassEdge schema is valid"
}

// A diagnostic a pass reports. Naming an entity and carrying no span, it
// gets that entity's span from the host.
type PassDiagnostic {
  code       string
  severity   Severity
  message    string
  span       SourceSpan @optional
  suggestion string     @optional
  entity     string     @optional
  verify unit "PassDiagnostic schema is valid"
}

// What a pass answers: its diagnostics bare (an array), or with a summary
// whose keys join the host's report of the pass (an object).
type PassOutput {
  diagnostics PassDiagnostic[]
  summary     object @optional
  verify unit "PassOutput schema is valid"
}

// What a custom rule's wasm_function receives for one entity: the entity,
// the resolution of its references, the declared types and the host's
// primitive types.
type ValidatorContext {
  entity         object
  referenced     object[]
  declared_types string[]
  primitives     string[]
  verify unit "ValidatorContext schema is valid"
}

// {"verdict": "pass"} or {"verdict": "fail", "field"?, "value"?}.
type ValidatorVerdict {
  verdict ValidatorVerdictKind
  field   string @optional
  value   string @optional
  verify unit "ValidatorVerdict schema is valid"
}

type ValidatorVerdictKind = pass | fail

// What an analyzer's scan export receives: one source file.
type ScanRequest {
  file_path string
  content   string
  verify unit "ScanRequest schema is valid"
}

// The public items the scan found.
type ScanResponse {
  items    ScannedItem[]
  language string @optional
  verify unit "ScanResponse schema is valid"
}

type ScannedItem {
  name       string
  item_kind  string
  line       integer
  visibility string @optional
  signature  string @optional
  verify unit "ScannedItem schema is valid"
}

// What an MCP resource's export receives, and answers.
type McpResourceRequest {
  uri string
  verify unit "McpResourceRequest schema is valid"
}

type McpResourceContent {
  content   string
  mime_type string
  verify unit "McpResourceContent schema is valid"
}

// What the migration hook receives after specforge migrate rewrote the
// project's files. Its answer is not read.
type MigrationInput {
  from  string
  to    string
  files string[]
  verify unit "MigrationInput schema is valid"
}
