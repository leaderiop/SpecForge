// Wasm/Extism extension runtime types
//
// Extension manifests use ManifestV2 from types/zero-entity-core.spec.
// This file contains supporting types for the Wasm runtime: dependencies,
// host function bindings, sandbox policies, caching, enhancements, and queries.

use types/config
use types/core

type PeerDependency {
  extension         string
  version           string
}

type HostFunctionBinding {
  name              string          @readonly
  input_schema      string
  output_schema     string
}

type SandboxPolicy {
  max_memory_mb     integer         @optional
  max_execution_ms  integer         @optional
  allowed_domains   string[]        @optional
  allowed_paths     string[]        @optional
  file_system_access string         @optional
  network_access    string          @optional
  // Default: 5000. Per-request HTTP timeout in milliseconds.
  http_timeout_ms   integer         @optional
  // Default: 15000. Total HTTP time budget per compilation in milliseconds.
  http_total_budget_ms integer      @optional
  // Default: [".json", ".html", ".csv", ".svg", ".dot", ".xml", ".txt", ".pdf"]
  // .md is NOT in the default list because SpecForge is not a documentation
  // generator (vision/README.md). Extensions that produce structured reports
  // (traceability matrices, coverage dashboards — not prose) MAY add .md to
  // their own sandbox policy via allowed_output_extensions override.
  allowed_output_extensions string[] @optional
  // Default: 1MB. Maximum file size readable via read_file host function.
  max_read_file_size u64           @optional
}

type WasmModuleCache {
  wasm_hash         string          @readonly
  aot_path          string
  platform          string
  created_at        string
}

type WarmEngineConfig {
  max_instances     u32             @doc "Default: 16"
  max_memory_mb     u32             @doc "Default: 512"
}

// trapped state removed — extensions that trap are immediately unloaded
type ExtensionLifecycleState = discovered | loading | initialized | validating | exporting | unloaded | failed

// ── Entity Enhancement Types ─────────────────────────────────

type FieldEnhancement {
  target_entity     string          @readonly
  field_name        string          @readonly
  field_type        EnhancedFieldType
  required          boolean         @optional
  description       string          @optional
}

type EnhancedFieldType = string_type | integer_type | bool_type | enum_type | string_list_type | reference_type | reference_list_type

type EnumFieldType {
  values            string[]
}

type ReferenceFieldType {
  // Maps to EdgeType.label when building graph edges
  edge_label        string
  target_kind       string          @optional
}

type DynamicEdgeType {
  label             string          @readonly
  source_extension    string          @readonly
  soft              boolean         @optional
}

type EnhancementConflict {
  entity_kind       string          @readonly
  field_name        string          @readonly
  first_extension     string          @readonly
  second_extension    string          @readonly
  resolution        ConflictResolution
}

type ConflictResolution = unresolved | explicit_override | load_order | namespaced

// v1: error only. priority and namespace policies are deferred to a future phase.
type EnhancementPolicy = error

// ── Query Extension Types ───────────────────────────────────

type QueryExtension {
  kind              QueryFileKind   @readonly
  patterns          string
}

type QueryFileKind = highlights | folds | indents | injections

// ── Extension Lifecycle Types ─────────────────────────────────

type ExtensionInstallResult {
  extension_name      string          @readonly
  version           string          @readonly
  source            ExtensionSource
  wasm_size         integer
  aot_compiled      boolean
  installed_path    string
}

type ExtensionSource = registry | local | git

type WasmTrapInfo {
  kind              string          @readonly
  message           string          @readonly
  export_name       string          @optional
  memory_address    string          @optional
  extension_name      string
}

// ── Lock File Types ──────────────────────────────────────────

// TrustLevel is defined canonically in types/config.spec.

type LockFileEntry {
  extension_name      string          @readonly
  version           string          @readonly
  source            ExtensionSource
  wasm_hash         string          @readonly
  resolved_at       string
  trust_level       TrustLevel      @optional
}

// Serialized as JSON (specforge.lock). See P6: standard is the moat.
type LockFile {
  path              string          @readonly
  lockfile_version  integer         @readonly
  entries           LockFileEntry[]
}

// ── Collector Contribution Types ────────────────────────────

type CollectorContribution {
  name              string          @readonly
  description       string          @optional
  input_formats     string[]
  auto_detect       CollectorAutoDetect @optional
  entity_mapping    CollectorEntityMapping
  export            string
  output_schema     string
}

type CollectorAutoDetect {
  file_patterns     string[]
  env_vars          string[]        @optional
}

type CollectorEntityMapping {
  strategies        EntityMappingStrategy[]
}

type EntityMappingStrategy {
  priority          integer
  strategy_type     string          @readonly
  description       string          @optional
}

type CollectorReport {
  schema            string          @readonly
  entries           CollectorReportEntry[]
  unmapped_tests    string[]        @optional
  stats             CollectorStats
}

type CollectorReportEntry {
  entity_id         string
  test_id           string
  status            CollectorTestStatus
  duration_ms       integer         @optional
  source            string          @optional
}

type CollectorTestStatus = pass | fail | skip | error

type CollectorStats {
  total             integer
  mapped            integer
  unmapped          integer
}

type ExtensionSpecifier "Parsed Extension Specifier" {
  raw        string
  format     ExtensionSource
  scope      string @optional
  name       string
  version    string @optional
  path       string @optional
  git_ref    string @optional
}


type CollectorDispatchInput {
  collector_id    string
  test_report_path string
  entity_ids      EntityId[]
  options         JsonObject      @optional
}

// --- Extension-Defined Grammar Types ---

type GrammarContribution {
  entity_kinds      string[]
  grammar_wasm_path string
  language_name     string
  version           string          @optional
  checksum          string          @optional
}

type BodyParserContribution {
  entity_kinds      string[]
  export_name       string
  output_schema     string          @optional
  timeout_ms        integer         @optional
}

type GrammarConflictPolicy = error | priority | namespace

type GrammarCacheEntry {
  grammar_hash      string
  compiled_path     string
  language_name     string
  source_extension  string
  created_at        string
}
