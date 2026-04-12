use "types/wasm"
use "types/zero-entity-core"
use "types/graph"
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
  // Coverage configuration is owned by @specforge/coverage extension.
  // At runtime this is a FieldMap deserialized into the extension's CoverageConfig type.
  coverage     FieldMap          @optional
  enhancement_policy   EnhancementPolicy @optional
  enhancement_overrides FieldMap @optional
  registries   RegistryConfig[] @optional
  // When false, disables the well-known public registry. Default: true.
  default_registry boolean @optional
  // federation config is extension-provided (see @specforge/federation extension)
  watch_debounce_ms  integer @optional
  // When true, GraphDelta includes old+new values for modified fields (default: false)
  delta_include_values boolean @optional
  // Wasm warm engine pool configuration for LSP/MCP long-running modes
  warm_engine WarmEngineConfig @optional
  grammar_policy GrammarConflictPolicy @optional
  // Graph Protocol schema version compatibility range for agent negotiation.
  // Defaults to current major range (e.g., 1.0.0..1.x.x). See ADR graph_protocol_version_management.
  supported_schema_min SchemaVersion @optional
  supported_schema_max SchemaVersion @optional
  verify unit "CompilerConfig schema is valid"
}

type ProviderConfig {
  scheme     string    @readonly
  alias      string    @unique
  extension  string
  settings   FieldMap  @optional
  verify unit "ProviderConfig schema is valid"
}

// CoverageConfig removed — canonical definition lives in
// extensions/coverage/types.spec per Principle 5 (extensions over built-ins).
// CompilerConfig.coverage uses FieldMap; the coverage extension deserializes it.

// ── Registry Types ──────────────────────────────────────────

type RegistryConfig {
  alias             string          @readonly  @unique
  url               string
  scope_filter      string[]        @optional
  credential        RegistryCredential @optional
  verify unit "RegistryConfig schema is valid"
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
  verify unit "RegistryResponse schema is valid"
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
  prompts           integer         @optional
  parsers           integer         @optional
  grammars          integer         @optional
  body_parsers      integer         @optional
  verify unit "ContributesSummary schema is valid"
}

type RegistrySearchResult {
  results           RegistryResponse[]
  total_count       integer
  query             string
  verify unit "RegistrySearchResult schema is valid"
}

type TrustLevel = verified | community | local | git

// ── Registry Authentication ───────────────────────────────

// At least one of token_env_var or token_file MUST be present.
// Validation rule: authenticate_registry_request MUST emit E-level diagnostic if both are absent.
type AuthMethod = bearer | basic | custom

type RegistryCredential {
  alias          string          @readonly @unique
  scope          string
  token_env_var  string          @optional
  token_file     string          @optional
  auth_method    AuthMethod
  verify unit "RegistryCredential schema is valid"
}

type InitConfig {
  name string
  spec_root string @optional
  extensions string[] @optional
  interactive boolean @optional
  version string @optional
  verify unit "InitConfig schema is valid"
}

type InitOutput {
  project_root string @readonly
  config_path string @readonly
  spec_file_path string @readonly
  extensions_installed string[] @readonly
  verify unit "InitOutput schema is valid"
}

// ProjectConfig is the serialization shape of specforge.json — the subset
// of CompilerConfig that users edit directly. CompilerConfig extends this
// with computed fields (schema, test_dirs, watch settings, etc.) derived
// at compile time. ProjectConfig -> CompilerConfig is a one-way transform.
type ProjectConfig {
  name string
  version string @optional
  spec_root string
  extensions string[]
  providers ProviderConfig[] @optional
  verify unit "ProjectConfig schema is valid"
}

type InitError {
  kind "already_exists" | "unresolvable_extension" | "invalid_name" | "io_error"
  message string
  path string @optional
  extension string @optional
  verify unit "InitError schema is valid"
}

type BundledExtensionCatalog {
  extensions BundledExtensionEntry[]
  verify unit "BundledExtensionCatalog schema is valid"
}

type BundledExtensionEntry {
  name        string  @readonly
  description string  @optional
  tags        string[] @optional
  // Display ordering hint (lower = more prominent). Ordering MUST NOT
  // favor any single domain — software, compliance, design, data, etc.
  // must receive equal prominence. See P2: zero domain knowledge in core.
  priority    u32     @optional
  verify unit "BundledExtensionEntry schema is valid"
}

// ExtensionManifest is an alias for ManifestV2 (defined in types/zero-entity-core).
// Used in the CompilerApi.add() return type to represent a resolved extension.
type ExtensionManifest = ManifestV2
