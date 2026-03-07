use types/wasm
use types/zero-entity-core

// Configuration types — compiler and extension configuration
//
// DSL-to-type mapping for specforge.json / spec block syntax:
//   providers { <scheme> "<alias>" { extension "..." repo "..." } }
//     → ProviderConfig { scheme, alias (from positional string), extension, settings }

type CompilerConfig {
  schema       string     @optional
  // Serialized as "$schema" in JSON ($ prefix is a JSON convention)
  name         string     @readonly
  version      string     @readonly
  spec_root    string     @optional
  strict       boolean    @optional
  namespace    string     @optional
  display_prefix string   @optional
  extensions   string[]
  providers    ProviderConfig[]  @optional
  test_dirs    string[]          @optional
  coverage     CoverageConfig    @optional
  entity_kind_policy   EntityKindPolicy  @optional
  enhancement_policy   EnhancementPolicy @optional
  enhancement_overrides FieldMap @optional
  registries   RegistryConfig[] @optional
  // federation config is extension-provided (see @specforge/federation extension)
  watch_debounce_ms  integer @optional
}

type ProviderConfig {
  scheme     string    @readonly
  alias      string    @unique
  extension  string
  settings   FieldMap  @optional
}

type CoverageConfig {
  threshold                integer
  reports                  string[]      @optional
  require_violation_tests  boolean       @optional
  fail_on_unknown_ids      boolean       @optional
}

// ── Registry Types ──────────────────────────────────────────

type RegistryConfig {
  alias             string          @readonly  @unique
  url               string
  scope_filter      string[]        @optional
  credential        RegistryCredential @optional
}

type RegistryResponse {
  extension_name    string          @readonly
  description       string          @optional
  latest_version    string
  versions          string[]
  contributes_summary ContributesSummary @optional
  peer_dependencies string[]        @optional
  downloads         integer         @optional
  published_at      string          @optional
  wasm_size_bytes   integer         @optional
  sha256            string          @optional
}

type ContributesSummary {
  entities          integer         @optional
  edges             integer         @optional
  ref_schemes       integer         @optional
  validators        integer         @optional
  renderers         integer         @optional
  providers         integer         @optional
  graph_views       integer         @optional
  collectors        integer         @optional
}

type RegistrySearchResult {
  results           RegistryResponse[]
  total_count       integer
  query             string
}

type TrustLevel = verified | community | local | git

// ── Registry Authentication ───────────────────────────────

// At least one of token_env_var or token_file MUST be present.
// Validation rule: authenticate_registry_request MUST emit E-level diagnostic if both are absent.
type RegistryCredential {
  alias          string          @readonly @unique
  scope          string
  token_env_var  string          @optional
  token_file     string          @optional
  auth_method    string
}

type InitConfig {
  name string
  spec_root string @optional
  extensions string[] @optional
  interactive boolean @optional
  version string @optional
}

type InitOutput {
  project_root string @readonly
  config_path string @readonly
  spec_file_path string @readonly
  extensions_installed string[] @readonly
}

type ProjectConfig {
  name string
  version string @optional
  spec_root string
  extensions string[]
  providers ProviderConfig[] @optional
}

type InitError {
  kind "already_exists" | "unresolvable_extension" | "invalid_name" | "io_error"
  message string
  path string @optional
  extension string @optional
}

type BundledExtensionCatalog {
  extensions BundledExtensionEntry[]
}

type BundledExtensionEntry {
  name        string  @readonly
  description string  @optional
  tags        string[] @optional
}

// ExtensionManifest is an alias for ManifestV2 (defined in types/zero-entity-core).
// Used in the CompilerApi.add() return type to represent a resolved extension.
type ExtensionManifest = ManifestV2
