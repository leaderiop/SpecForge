// Extension behaviors — extensions, providers, renderers

use invariants/core
use invariants/extensions
use invariants/validation
use invariants/wasm
use invariants/zero-entity-core
use types/config
use types/zero-entity-core
use types/wasm
use types/errors
use types/diagnostics
use ports/outbound
use events/extensions
use events/compilation

// load_extension_manifests is the top-level orchestrator for extension discovery.
// It delegates to load_extension_manifest (behaviors/wasm.spec) for per-extension
// loading and to validate_extension_manifest (behaviors/wasm.spec) for schema
// validation. After all manifests are loaded, it triggers registry population
// via the behaviors in behaviors/zero-entity-core.spec.
behavior load_extension_manifests "Load Extension Manifests" {
  invariants [registry_population_before_validation, zero_domain_knowledge_core, extension_load_order_determinism, offline_first_extension_resolution]
  ports      [FileSystem]
  types      [ManifestV2, CompilerConfig, ExtensionError]
  consumes   [all_files_parsed]
  produces   [extension_manifests_loaded]

  contract """
    At startup, the compiler MUST read the extensions list from specforge.json
    and locate each extension's Wasm manifest. The manifest MUST declare
    entity types, edge types, validation rules, wasmPath to the .wasm
    binary, and optional peer dependencies. Missing extensions or unloadable
    .wasm binaries MUST produce a diagnostic, not a crash. This behavior
    orchestrates: for each extension, it calls load_extension_manifest to
    locate and parse the manifest, then validate_extension_manifest for
    schema validation. Once all manifests are loaded and the
    extension_manifests_loaded event is produced,
    register_extension_entity_types consumes it to populate the
    KindRegistry, FieldRegistry, and EdgeRegistry.
  """

  verify unit "installed extension manifest is loaded"
  verify unit "missing extension produces diagnostic"
  verify unit "manifest declares entity types and validations"
  verify unit "manifest includes wasmPath to .wasm binary"
  verify integration "two extensions loaded and registries populated without collision"

}

// register_extension_entity_types is a thin delegation wrapper that calls
// register_entity_kinds_from_manifest (behaviors/zero-entity-core.spec)
// for each loaded extension. The detailed registration semantics — including
// KindRegistry population, field registry setup, and edge type registration —
// are defined in the zero-entity-core behaviors.
behavior register_extension_entity_types "Register Extension Entity Types" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [ManifestV2, KindRegistryEntry]
  consumes   [extension_manifests_loaded]
  produces   [extension_entity_types_registered]

  contract """
    After loading manifests, the compiler MUST register each extension's
    entity types by delegating to register_entity_kinds_from_manifest
    for kind registration, populate_field_registry_from_extensions for
    field registration, and populate_edge_registry_from_extensions for
    edge types. When resolving references, the KindRegistry MUST be
    consulted to determine which extension owns each entity type and
    whether soft resolution applies for cross-extension references.
    When a reference targets an entity kind from an uninstalled extension,
    the compiler MUST emit I004 with the format:
    "Unknown entity kind '{kind}' — install an extension that provides
    it (e.g., `specforge add @specforge/{extension}`)." The message
    MUST include the unresolved kind name and a suggested extension
    when one can be inferred from the kind prefix.
  """

  verify unit "delegates to register_entity_kinds_from_manifest per extension"
  verify unit "unregistered type triggers soft resolution"
  verify unit "KindRegistry records source extension for each kind"
  verify unit "I004 message includes unresolved kind name and suggested extension"

}

behavior load_provider_configurations "Load Provider Configurations" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [ProviderConfig, CompilerConfig]
  ports      [FileSystem]
  consumes   [extension_manifests_loaded]
  produces   [provider_configured]

  contract """
    The compiler MUST parse provider blocks from specforge.json and
    create provider instances with their configured settings. The core
    MUST NOT hardcode any provider schemes or kinds — all provider
    configuration comes exclusively from specforge.json and extension
    manifests. Multiple instances of the same provider with different
    aliases MUST be supported.
  """

  verify unit "single provider instance is created"
  verify unit "multiple aliased instances are created"
  verify unit "provider config settings are passed through"
  verify unit "no hardcoded provider schemes exist in core"

}

behavior register_provider_schemes "Register Provider Schemes" {
  invariants [reference_resolution_completeness, diagnostic_determinism, zero_domain_knowledge_core]
  types      [ProviderConfig, ManifestV2, SchemeRegistryEntry, Diagnostic]
  ports      [WasmRuntime]
  consumes   [provider_configured]
  produces   [provider_schemes_registered]

  contract """
    After loading provider configurations from specforge.json, the compiler
    MUST query each provider extension's manifest for the ref schemes and
    kinds it supports. Each scheme MUST be registered as a SchemeRegistryEntry
    so that validate_provider_refs can route refs to the correct provider.
    The core MUST NOT contain any built-in scheme registrations — all
    schemes come exclusively from provider extensions. Schemes already
    registered by another provider MUST produce a W025 warning listing
    both providers; the provider declared first in the specforge.json
    providers array MUST win the scheme registration as a deterministic
    tiebreaker. Unresolvable provider extensions MUST produce an
    ExtensionError diagnostic.
  """

  verify unit "provider schemes registered from manifest"
  verify unit "duplicate scheme from two providers produces W025"
  verify unit "duplicate scheme resolved by specforge.json declaration order"
  verify unit "unresolvable provider extension produces ExtensionError"
  verify unit "no built-in schemes exist before provider loading"
  verify integration "Wasm-based provider scheme registered and validates ref"

}

behavior validate_provider_refs "Validate Provider Refs" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [SchemeRegistryEntry, Diagnostic]
  ports      [RefValidator]
  consumes   [provider_schemes_registered]
  produces   [provider_ref_validated]

  contract """
    When a ref entity uses a registered scheme, the compiler MUST
    delegate validation to the corresponding provider. The provider
    MUST validate the kind and identifier format. Unknown schemes
    MUST emit I005.
  """

  verify unit "known scheme delegates to provider"
  verify unit "unknown scheme emits I005"
  verify unit "provider validates identifier format"
  verify unit "no built-in ref validation logic exists in core"

}

// remove_extension is the user-facing CLI entry point for extension removal.
// It delegates to uninstall_wasm_extension (behaviors/wasm.spec) for the full
// Wasm lifecycle cleanup. This behavior owns the CLI interaction and post-removal
// diagnostic messaging; uninstall_wasm_extension owns the implementation.
behavior remove_extension "Remove Extension" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [CompilerConfig, ExtensionError, Diagnostic, UnknownKindError]
  ports      [FileSystem]
  produces   [extension_removed]

  contract """
    When specforge remove <extension-specifier> is invoked, the system MUST
    delegate to uninstall_wasm_extension (behaviors/wasm.spec) for the full
    Wasm lifecycle cleanup: removing the extension entry from specforge.json,
    deleting the .wasm binary, invalidating the AOT cache, updating
    specforge.lock, and checking peer dependencies. This behavior is the
    user-facing CLI entry point; uninstall_wasm_extension handles the
    implementation. Existing .spec files using the extension's entities
    MUST NOT be modified. On the next compilation, entity blocks using the
    removed extension's keywords MUST produce E024 (unknown entity kind)
    since the keyword is no longer in the KindRegistry. Reference list
    entries pointing to those entities MUST produce E001 (dangling
    reference). The user MUST either reinstall the extension or remove
    the affected entity blocks.
  """

  verify unit "delegates to uninstall_wasm_extension for lifecycle cleanup"
  verify unit "extension is removed from extensions list"
  verify unit "removed extension keywords produce E024 on next compile"
  verify unit ".spec files are not modified by removal"

}

// Read-only query — no event produced.
behavior list_installed_extensions "List Installed Extensions" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  types      [ManifestV2, KindRegistryEntry]

  contract """
    When specforge extensions is invoked, the system MUST list all installed
    extensions with their name, version, entity count, and registered entity types.
    The listing MUST query the KindRegistry to enumerate entity kinds per
    extension. Output order MUST be deterministic (alphabetical by extension name).
  """

  verify unit "list shows all installed extensions"
  verify unit "list includes entity counts and entity types"
  verify unit "output order is deterministic"

}

// Read-only query — no event produced.
behavior list_configured_providers "List Configured Providers" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  types      [ProviderConfig, SchemeRegistryEntry]

  contract """
    When specforge providers is invoked, the system MUST list all configured
    providers with their alias, extension, registered schemes, and supported
    kinds. The listing MUST query the SchemeRegistryEntry set to show which
    schemes each provider handles. Providers with multiple instances MUST
    show each alias separately. Output order MUST be deterministic.
  """

  verify unit "list shows all configured providers"
  verify unit "list includes scheme and kind registrations"
  verify unit "multiple aliases shown separately"
  verify unit "output order is deterministic"

}

// Read-only query — no event produced.
behavior validate_ref_target_format "Validate Ref Target Format" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [Diagnostic, SchemeRegistryEntry]
  ports      [RefValidator]

  contract """
    When a provider is installed, the validator MUST check that ref
    identifiers match the provider's expected pattern by delegating to the
    RefValidator port's validateIdentifier method. Malformed identifiers
    MUST produce an E011 diagnostic.
  """

  verify unit "valid ref identifier passes"
  verify unit "malformed ref identifier produces E011"
  verify unit "no built-in format patterns exist in core"

}

// Read-only query — no event produced.
behavior validate_provider_kinds "Validate Provider Kinds" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [Diagnostic, SchemeRegistryEntry]
  ports      [RefValidator]

  contract """
    When a ref uses a known scheme but an unregistered kind, the validator
    MUST delegate to the RefValidator port's validateKind method and
    MUST produce an E013 diagnostic listing the valid kinds for that scheme.
  """

  verify unit "valid scheme and kind passes"
  verify unit "valid scheme with unknown kind produces E013"
  verify unit "no built-in kind registrations exist in core"

}

// -- Registry Behaviors -----

behavior resolve_registry_source "Resolve Registry Source" {
  invariants [registry_integrity, multi_error_collection, extension_operation_atomicity, offline_first_extension_resolution]
  types      [RegistryConfig, RegistryResponse, CompilerConfig, ExtensionError]
  ports      [RegistryClient]
  consumes   [registries_configured]
  produces   [registry_resolved]

  contract """
    When resolving an extension specifier with @scope/name format, the
    system MUST query the configured registry for that scope. Scope routing
    MUST use the scope_filter field from RegistryConfig entries in
    specforge.json. If no scope-specific registry matches, the system MUST
    fall back to the default registry. Network errors MUST produce an
    ExtensionError diagnostic with retry guidance.
  """

  verify unit "scope-specific registry queried for matching scope"
  verify unit "default registry used when no scope filter matches"
  verify unit "network error produces ExtensionError with retry guidance"
  verify unit "successful query returns RegistryResponse"
  verify integration "unreachable scope-specific registry falls back to next scope"

}

behavior search_registry "Search Registry" {
  invariants [diagnostic_determinism, multi_error_collection]
  types      [RegistryConfig, RegistrySearchResult, RegistryResponse, CompilerConfig]
  ports      [RegistryClient]
  produces   [registry_search_completed]

  contract """
    When specforge search is invoked, the system MUST query all configured
    registries with the search term. Results MUST be filterable by
    contribution type (entities, validators, renderers, providers,
    collectors). Results from multiple registries MUST be merged and
    deduplicated using a composite key of name + version — when the
    same name + version appears from multiple registries, the first
    registry in specforge.json declaration order wins. Output MUST be
    deterministic — sorted by relevance score then extension name.
  """

  verify unit "queries all configured registries"
  verify unit "filters by contribution type"
  verify unit "deduplicates results across registries"
  verify unit "output is deterministic"
  verify unit "error from one registry does not abort search of others"

}

// CLI entry point: `specforge publish`. Delegates Wasm binary packaging
// to publish_wasm_extension in behaviors/wasm.spec.
behavior publish_to_registry "Publish to Registry" {
  invariants [registry_integrity, multi_error_collection]
  types      [ManifestV2, RegistryConfig, ExtensionError]
  ports      [RegistryClient, FileSystem]
  produces   [extension_published_to_registry]

  contract """
    When specforge publish is invoked with a registry target,
    the system MUST validate the manifest, compute the SHA256 hash of the
    .wasm binary, upload both to the registry, and authenticate the
    request. Duplicate version numbers MUST be rejected unless --force is
    provided. Successful publish MUST return the registry URL for the
    published version.
  """

  verify unit "manifest validated before publish"
  verify unit "SHA256 computed and included in upload"
  verify unit "duplicate version rejected without --force"
  verify unit "successful publish returns registry URL"
  verify unit "unauthenticated publish produces ExtensionError"

}

behavior verify_registry_integrity "Verify Registry Integrity" {
  invariants [registry_integrity, aot_cache_integrity, offline_first_extension_resolution]
  types      [RegistryResponse, LockFileEntry, TrustLevel, ExtensionError]
  ports      [FileSystem]
  produces   [registry_integrity_verified]

  contract """
    After downloading a .wasm binary from a registry, the system MUST
    verify its SHA256 hash against the hash declared in the RegistryResponse.
    Mismatches MUST produce a hard error and abort installation. The
    trust level MUST be assigned deterministically from the source:
    local filesystem paths MUST receive "local", git URLs MUST receive
    "git", community registries without publisher verification MUST
    receive "community", and registries with publisher signature
    verification MUST receive "verified". The assigned trust level
    MUST be recorded in specforge.lock alongside the SHA256 hash.
  """

  verify unit "matching SHA256 passes verification"
  verify unit "mismatched SHA256 produces hard error"
  verify unit "trust level recorded in specforge.lock"
  verify unit "local source assigned local trust level"
  verify unit "git source assigned git trust level"
  verify unit "community registry source assigned community trust level"
  verify unit "verified registry source assigned verified trust level"

}

behavior configure_registries "Configure Registries" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core, registry_integrity, registry_api_openness, offline_first_extension_resolution]
  types      [RegistryConfig, CompilerConfig]
  ports      [FileSystem]
  produces   [registries_configured]

  contract """
    At startup, the compiler MUST parse the registries array from
    specforge.json into RegistryConfig entries. Each entry MUST have an
    alias (unique identifier), url, and optional scope_filter array.
    When resolving extension specifiers, scope_filter MUST route @scope/
    prefixed specifiers to the matching registry. If no registries are
    configured, registry operations (search, fetch) MUST produce an I003
    info diagnostic indicating no registries are available. First-use
    MUST NOT require network access — registries are opt-in configuration.
    Setting `registries: []` with `default_registry: false` in specforge.json
    explicitly disables the default public registry, forcing fully offline
    operation.
  """

  verify unit "registries parsed from specforge.json"
  verify unit "scope_filter routes to correct registry"
  verify unit "default registry used when none configured"
  verify unit "duplicate alias produces warning"
  verify property "registry API schema is published as open specification"
  verify integration "all registries disabled produces fully offline mode"

}

// ── Registry Authentication ──────────────────────────────────

// Called imperatively during registry operations (resolve, search, publish) —
// not event-driven. Auth is request-time, triggered by 401 responses or
// pre-configured credentials.
behavior authenticate_registry_request "Authenticate Registry Request" {
  invariants [registry_integrity, multi_error_collection, credential_secrecy, offline_first_extension_resolution]
  types      [RegistryConfig, RegistryCredential, ExtensionError, RegistryError]
  ports      [RegistryClient]
  produces   [registry_authenticated]

  contract """
    When making a request to a registry that has a configured credential,
    the system MUST resolve the authentication token from the specified
    source: environment variable (token_env_var) or token file (token_file).
    The resolved token MUST be attached as an Authorization header. If the
    token source is unavailable (env var unset, file missing), the system
    MUST produce an ExtensionError diagnostic with guidance. On receiving
    a 401 response, the compiler MUST re-resolve the credential from its
    source. If the re-resolved credential also fails, the compiler MUST
    emit an E-level diagnostic with resolution guidance (e.g., "run
    `specforge registry login`"). On receiving a 403 response, the
    compiler MUST emit an E-level diagnostic with permission guidance.
    Retry logic for transient failures (429, timeout) is handled by
    retry_registry_request. When the token source is available but the
    registry is unreachable (network timeout, DNS failure), the system
    MUST fall back to the cached manifest in specforge.lock and the
    locally stored .wasm binary if available, emitting an I-level
    diagnostic. Authentication failure (401/403) MUST NOT trigger cache
    fallback — only network-level failures. Raw tokens MUST never be
    logged or stored in specforge.json.
  """

  verify unit "token resolved from environment variable"
  verify unit "token resolved from token file"
  verify unit "missing token source produces ExtensionError"
  verify unit "401 response triggers credential re-resolution"
  verify unit "double 401 after re-resolution emits E-level diagnostic with login guidance"
  verify unit "raw tokens never logged or stored in config"
  verify unit "403 response produces E-level diagnostic with permission guidance"
  verify unit "unreachable registry with cached extension falls back to cache with I-level diagnostic"
  verify unit "authentication failure (401/403) does not trigger cache fallback"

}

behavior retry_registry_request "Retry Registry Request" {
  invariants [registry_integrity, multi_error_collection]
  types      [RegistryConfig, RegistryError, ExtensionError]
  ports      [RegistryClient]

  contract """
    When a registry request receives a 429 (rate limited) response, the
    system MUST retry with exponential backoff (base 1s, max 30s, max
    retries 3). When a request times out (network timeout), the system
    MUST produce an ExtensionError diagnostic with retry guidance. Retry
    logic applies to all registry operations (authentication, download,
    search, publish) uniformly.
  """

  verify unit "429 response retries with exponential backoff"
  verify unit "network timeout produces ExtensionError with retry guidance"
  verify unit "max retries exceeded produces final error"

}

behavior validate_registry_credentials "Validate Registry Credentials" {
  invariants [registry_integrity, diagnostic_determinism, credential_secrecy]
  types      [RegistryCredential, RegistryConfig, RegistryError]
  ports      [RegistryClient]
  produces   [registry_credentials_validated]

  contract """
    When specforge registry login is invoked, the system MUST validate the
    provided credentials against the target registry by making an authenticated
    test request. Valid credentials MUST be stored as a RegistryCredential
    entry referencing only the environment variable name or token file path —
    never the raw token value. The system MUST confirm successful authentication
    with an info message including the registry alias and authenticated scope.
  """

  verify unit "valid credentials stored as RegistryCredential reference"
  verify unit "invalid credentials produce error with guidance"
  verify unit "raw token never stored in specforge.json"
  verify unit "success message includes registry alias and scope"

}

behavior logout_registry "Logout Registry" {
  invariants [registry_integrity, diagnostic_determinism, credential_secrecy]
  types      [RegistryConfig, RegistryCredential, RegistryError]
  ports      [FileSystem]
  produces   [registry_logged_out]

  contract """
    When specforge registry logout --alias <alias> is invoked, the system
    MUST remove the stored credential reference for the given registry alias.
    The alias MUST match a RegistryConfig entry's alias field. The removal
    MUST delete only the RegistryCredential entry whose alias matches —
    credentials for other aliases (and their scopes) MUST remain untouched.
    If no credential exists for the specified alias, the command MUST succeed
    silently. The system MUST NOT attempt any network requests during logout.
  """

  verify unit "credential reference removed for matching alias"
  verify unit "credentials for other aliases and scopes remain untouched"
  verify unit "no credential for alias succeeds silently"
  verify unit "no network requests made during logout"

}

behavior generate_keyword_extension_index "Generate Keyword Extension Index" {
  invariants [registry_integrity]
  types      [RegistryConfig, KeywordExtensionIndex]
  ports      [RegistryClient, FileSystem]
  produces   [keyword_extension_index_generated]

  contract """
    At build time or release time, the system MUST read the extension
    registry to enumerate all published extensions and their declared
    entity keywords. The result MUST be serialized as a KeywordExtensionIndex
    JSON file mapping each known keyword to the extension name that provides
    it. This index is shipped as a bundled data file for use by
    suggest_missing_extensions at runtime. The generation MUST be
    deterministic — the same registry state MUST produce the same index
    file. Keywords claimed by multiple extensions MUST include all
    providing extensions in the mapping.
  """

  verify unit "index maps each keyword to its providing extension"
  verify unit "index generation is deterministic"
  verify unit "keyword claimed by multiple extensions lists all providers"
  verify unit "empty registry produces empty index"

}

behavior support_private_registries "Support Private Registries" {
  invariants [registry_integrity, wasm_sandbox_integrity, credential_secrecy]
  types      [RegistryConfig, RegistryCredential, TrustLevel, RegistryResponse]
  ports      [RegistryClient]

  contract """
    When fetching extensions from a registry with configured credentials,
    the system MUST authenticate before fetching. The scope_filter on the
    RegistryConfig MUST be respected — only extensions matching the scope
    filter SHOULD be fetched from authenticated registries. Extensions from
    private registries MUST be assigned the appropriate trust level based on
    the registry's trust configuration. Private registry errors MUST NOT
    leak authentication details in diagnostic messages.
  """

  verify unit "authentication occurs before fetch from private registry"
  verify unit "scope_filter restricts which extensions are fetched"
  verify unit "trust level assigned based on registry configuration"
  verify unit "error messages do not leak authentication details"

}
