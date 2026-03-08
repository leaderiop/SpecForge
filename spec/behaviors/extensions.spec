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
// It delegates to load_extension_manifest (behaviors/wasm-lifecycle.spec) for per-extension
// loading and to validate_extension_manifest (behaviors/wasm-lifecycle.spec) for schema
// validation. After all manifests are loaded, it triggers registry population
// via the behaviors in behaviors/zero-entity-core.spec.
behavior load_extension_manifests "Load Extension Manifests" {
  invariants [registry_population_before_validation, zero_domain_knowledge_core, extension_load_order_determinism, offline_first_extension_resolution, compilation_pipeline_ordering]
  ports      [FileSystem]
  types      [ManifestV2, CompilerConfig, ExtensionError]
  consumes   [all_files_parsed]
  produces   [extension_manifests_loaded]

  requires {
    all_files_parsed "all_files_parsed event has fired, confirming all .spec files have been structurally parsed"
    extensions_config_available "Extensions list in specforge.json is available from the parsed project configuration"
  }

  ensures {
    all_extensions_attempted "All declared extensions have been attempted for loading"
    loaded_manifests_available "Successfully loaded manifests are available for registry population"
    failed_extensions_diagnosed "Failed extensions have produced diagnostics"
    loaded_event_fired "extension_manifests_loaded event fires exactly once after all extensions are processed"
  }

  maintains {
    extension_isolation "A failure loading one extension does not prevent loading of remaining extensions"
  }

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
    KindRegistry, FieldRegistry, and EdgeRegistry. After registry
    population, grammar and body parser contributions from manifests
    are registered via register_grammar_contributions and
    register_body_parser_contributions.
  """

  verify unit "installed extension manifest is loaded"
  verify unit "missing extension produces diagnostic"
  verify unit "manifest declares entity types and validations"
  verify unit "manifest includes wasmPath to .wasm binary"
  verify integration "two extensions loaded and registries populated without collision"
  verify contract "requires/ensures consistency for extension manifest loading"

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

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all extension manifests have been loaded and validated"
  }

  ensures {
    kind_registry_populated "KindRegistry contains entries for all entity kinds from loaded extensions"
    field_registry_populated "FieldRegistry contains field definitions from loaded extensions"
    edge_registry_populated "EdgeRegistry contains edge types from loaded extensions"
    registered_event_emitted "extension_entity_types_registered event fires exactly once after all registries are populated"
    soft_resolution_for_missing "Uninstalled extension kinds produce I004 info diagnostic with suggested extension"
  }

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
  verify contract "requires/ensures consistency for extension entity type registration"

}

behavior load_provider_configurations "Load Provider Configurations" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [ProviderConfig, CompilerConfig]
  ports      [FileSystem]
  consumes   [extension_manifests_loaded]
  produces   [provider_configured]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming extension manifests are available for provider lookup"
    specforge_json_available "specforge.json has been parsed and provider blocks are accessible"
  }

  ensures {
    provider_instances_created "Provider instances are created for all configured provider blocks"
    aliased_instances_distinct "Multiple instances of the same provider with different aliases are distinct"
    no_hardcoded_schemes "No provider schemes or kinds are hardcoded in core"
    provider_configured_emitted "provider_configured event fires exactly once after all providers are instantiated"
  }

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
  verify contract "requires/ensures consistency for provider configuration loading"

}

behavior register_provider_schemes "Register Provider Schemes" {
  invariants [reference_resolution_completeness, diagnostic_determinism, zero_domain_knowledge_core]
  types      [ProviderConfig, ManifestV2, SchemeRegistryEntry, Diagnostic]
  ports      [WasmRuntime]
  consumes   [provider_configured]
  produces   [provider_schemes_registered]

  requires {
    provider_configured_fired "provider_configured event has fired, confirming provider instances are created"
    wasm_runtime_available "WasmRuntime port is available for querying provider extension manifests"
  }

  ensures {
    schemes_registered "All provider schemes are registered as SchemeRegistryEntry entries"
    duplicate_scheme_warned "Duplicate schemes produce W025 warning listing both providers"
    declaration_order_tiebreak "Duplicate scheme conflicts resolved by specforge.json declaration order"
    schemes_registered_emitted "provider_schemes_registered event fires exactly once after all schemes are registered"
  }

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
  verify contract "requires/ensures consistency for provider scheme registration"

}

behavior validate_provider_refs "Validate Provider Refs" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [SchemeRegistryEntry, Diagnostic]
  ports      [RefValidator]
  consumes   [provider_schemes_registered]
  produces   [provider_ref_validated]

  requires {
    provider_schemes_registered_fired "provider_schemes_registered event has fired, confirming scheme-to-provider routing is established"
    ref_validator_available "RefValidator port is available for delegating validation"
  }

  ensures {
    known_scheme_delegated "Refs with known schemes are delegated to the corresponding provider"
    unknown_scheme_diagnosed "Refs with unknown schemes emit I005 diagnostic"
    ref_validated_emitted "provider_ref_validated event fires for each validated ref"
  }

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
  verify contract "requires/ensures consistency for provider ref validation"

}

// remove_extension is the user-facing CLI entry point for extension removal.
// It delegates to uninstall_wasm_extension (behaviors/wasm-lifecycle.spec) for the full
// Wasm lifecycle cleanup. This behavior owns the CLI interaction and post-removal
// diagnostic messaging; uninstall_wasm_extension owns the implementation.
behavior remove_extension "Remove Extension" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  types      [CompilerConfig, ExtensionError, Diagnostic, UnknownKindError]
  ports      [FileSystem]
  produces   [extension_removed]

  requires {
    extension_installed "Target extension is present in specforge.json extensions list"
    filesystem_available "FileSystem port is available for config and binary operations"
  }

  ensures {
    extension_entry_removed "Extension entry removed from specforge.json via uninstall_wasm_extension"
    spec_files_unchanged "Existing .spec files are not modified by the removal"
    extension_removed_emitted "extension_removed event fires exactly once after successful removal"
  }

  contract """
    When specforge remove <extension-specifier> is invoked, the system MUST
    delegate to uninstall_wasm_extension (behaviors/wasm-lifecycle.spec) for the full
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
  verify contract "requires/ensures consistency for extension removal"

}

// Read-only query — no event produced.
behavior list_installed_extensions "List Installed Extensions" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  types      [ManifestV2, KindRegistryEntry]

  requires {
    kind_registry_ready "KindRegistry is populated with entity kinds from loaded extensions"
  }

  ensures {
    all_extensions_listed "All installed extensions are included in the output"
    entity_counts_included "Each extension entry includes entity count and registered entity types"
    output_deterministic "Output order is alphabetical by extension name"
  }

  contract """
    When specforge extensions is invoked, the system MUST list all installed
    extensions with their name, version, entity count, and registered entity types.
    The listing MUST query the KindRegistry to enumerate entity kinds per
    extension. Output order MUST be deterministic (alphabetical by extension name).
  """

  verify unit "list shows all installed extensions"
  verify unit "list includes entity counts and entity types"
  verify unit "output order is deterministic"
  verify contract "requires/ensures consistency for extension listing"

}

// Read-only query — no event produced.
behavior list_configured_providers "List Configured Providers" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  types      [ProviderConfig, SchemeRegistryEntry]

  requires {
    scheme_registry_ready "SchemeRegistryEntry set is populated from provider registration"
  }

  ensures {
    all_providers_listed "All configured providers are included in the output"
    schemes_and_kinds_included "Each provider entry includes registered schemes and supported kinds"
    aliases_shown_separately "Providers with multiple instances show each alias separately"
    output_deterministic "Output order is deterministic"
  }

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
  verify contract "requires/ensures consistency for provider listing"

}

// Called imperatively by validate_provider_refs (which consumes provider_schemes_registered).
// Depends on SchemeRegistryEntry data populated during provider registration.
behavior validate_ref_target_format "Validate Ref Target Format" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core, diagnostic_determinism]
  types      [Diagnostic, SchemeRegistryEntry]
  ports      [RefValidator]

  requires {
    scheme_registry_populated "SchemeRegistryEntry data is populated from provider registration"
    ref_validator_available "RefValidator port is available for identifier validation"
  }

  ensures {
    valid_identifier_passes "Valid ref identifiers pass validation without diagnostics"
    malformed_identifier_diagnosed "Malformed ref identifiers produce E011 diagnostic"
  }

  contract """
    When a provider is installed, the validator MUST check that ref
    identifiers match the provider's expected pattern by delegating to the
    RefValidator port's validateIdentifier method. Malformed identifiers
    MUST produce an E011 diagnostic.
  """

  verify unit "valid ref identifier passes"
  verify unit "malformed ref identifier produces E011"
  verify unit "no built-in format patterns exist in core"
  verify contract "requires/ensures consistency for ref target format validation"

}

// Called imperatively by validate_provider_refs (which consumes provider_schemes_registered).
// Depends on SchemeRegistryEntry data populated during provider registration.
behavior validate_provider_kinds "Validate Provider Kinds" {
  invariants [reference_resolution_completeness, zero_domain_knowledge_core, diagnostic_determinism]
  types      [Diagnostic, SchemeRegistryEntry]
  ports      [RefValidator]

  requires {
    scheme_registry_populated "SchemeRegistryEntry data is populated from provider registration"
    ref_validator_available "RefValidator port is available for kind validation"
  }

  ensures {
    valid_kind_passes "Valid scheme and kind combinations pass validation"
    unknown_kind_diagnosed "Unknown kind for a known scheme produces E013 diagnostic listing valid kinds"
  }

  contract """
    When a ref uses a known scheme but an unregistered kind, the validator
    MUST delegate to the RefValidator port's validateKind method and
    MUST produce an E013 diagnostic listing the valid kinds for that scheme.
  """

  verify unit "valid scheme and kind passes"
  verify unit "valid scheme with unknown kind produces E013"
  verify unit "no built-in kind registrations exist in core"
  verify contract "requires/ensures consistency for provider kind validation"

}

// -- Registry Behaviors -----

behavior resolve_registry_source "Resolve Registry Source" {
  invariants [registry_integrity, multi_error_collection, extension_operation_atomicity, offline_first_extension_resolution]
  types      [RegistryConfig, RegistryResponse, CompilerConfig, ExtensionError]
  ports      [RegistryClient]
  consumes   [registries_configured]
  produces   [registry_resolved]

  requires {
    registries_configured_fired "registries_configured event has fired, confirming registry entries are parsed and available"
    registry_client_available "RegistryClient port is available for network queries"
  }

  ensures {
    scope_routed "Scope-prefixed specifiers are routed to the matching scope-specific registry"
    default_fallback_used "Specifiers with no matching scope fall back to the default registry"
    network_error_diagnosed "Network errors produce ExtensionError diagnostic with retry guidance"
    registry_resolved_emitted "registry_resolved event fires on successful resolution"
  }

  contract """
    When resolving an extension specifier with @scope/name format, the
    system MUST query the configured registry for that scope. Scope routing
    MUST use the scope_filter field from RegistryConfig entries in
    specforge.json. If no scope-specific registry matches, the system MUST
    fall back to the default registry. "Fall back to the default registry"
    means the default registry URL from user configuration, not a hardcoded
    URL in compiler source. The URL is published in documentation and can be
    overridden or disabled via `default_registry: false` in specforge.json.
    Network errors MUST produce an ExtensionError diagnostic with retry guidance.
  """

  verify unit "scope-specific registry queried for matching scope"
  verify unit "default registry used when no scope filter matches"
  verify unit "network error produces ExtensionError with retry guidance"
  verify unit "successful query returns RegistryResponse"
  verify integration "unreachable scope-specific registry falls back to next scope"
  verify contract "requires/ensures consistency for registry source resolution"

}

behavior search_registry "Search Registry" {
  invariants [diagnostic_determinism, multi_error_collection, offline_first_extension_resolution]
  types      [RegistryConfig, RegistrySearchResult, RegistryResponse, CompilerConfig]
  ports      [RegistryClient]
  produces   [registry_search_completed]

  requires {
    registries_available "At least one registry is configured or default registry is enabled"
    registry_client_available "RegistryClient port is available for network queries"
  }

  ensures {
    all_registries_queried "All configured registries are queried with the search term"
    results_deduplicated "Results from multiple registries are deduplicated by name + version"
    output_deterministic "Output is sorted by relevance score then extension name"
    search_completed_emitted "registry_search_completed event fires after results are collected"
  }

  maintains {
    partial_failure_resilience "Error from one registry does not abort search of remaining registries"
  }

  contract """
    When specforge search is invoked, the system MUST query all configured
    registries with the search term. Results MUST be filterable by
    contribution type (entities, validators, renderers, providers,
    collectors, prompts, parsers). Results from multiple registries MUST be merged and
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
  verify contract "requires/ensures consistency for registry search"

}

// CLI entry point: `specforge publish`. Delegates Wasm binary packaging
// to publish_wasm_extension in behaviors/wasm-lifecycle.spec.
behavior publish_to_registry "Publish to Registry" {
  invariants [registry_integrity, multi_error_collection, credential_secrecy]
  types      [ManifestV2, RegistryConfig, ExtensionError]
  ports      [RegistryClient, FileSystem]
  produces   [extension_published_to_registry]

  requires {
    manifest_valid "Extension ManifestV2 passes schema validation"
    wasm_binary_available "The .wasm binary referenced by wasmPath exists on the filesystem"
    registry_client_available "RegistryClient port is available for upload"
    credentials_available "Authentication credentials are available for the target registry"
  }

  ensures {
    sha256_computed "SHA256 hash of .wasm binary is computed and included in the upload"
    duplicate_version_rejected "Duplicate version numbers are rejected unless --force is provided"
    registry_url_returned "Successful publish returns the registry URL for the published version"
    published_event_emitted "extension_published_to_registry event fires on successful publish"
  }

  contract """
    When specforge publish is invoked with a registry target,
    the system MUST validate the extension's ManifestV2 (via validate_manifest_v2_schema), compute the SHA256 hash of the
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
  verify contract "requires/ensures consistency for registry publishing"

}

behavior verify_registry_integrity "Verify Registry Integrity" {
  invariants [registry_integrity, aot_cache_integrity, offline_first_extension_resolution]
  types      [RegistryResponse, LockFileEntry, TrustLevel, ExtensionError]
  ports      [FileSystem]
  produces   [registry_integrity_verified]

  requires {
    wasm_binary_downloaded "A .wasm binary has been downloaded from a registry"
    registry_response_available "RegistryResponse with declared SHA256 hash is available"
  }

  ensures {
    hash_verified "SHA256 hash of downloaded binary matches the declared hash"
    mismatch_aborts "Hash mismatch produces hard error and aborts installation"
    trust_level_assigned "Trust level is deterministically assigned based on source type"
    lock_file_updated "SHA256 hash and trust level are recorded in specforge.lock"
    integrity_verified_emitted "registry_integrity_verified event fires on successful verification"
  }

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
  verify contract "requires/ensures consistency for registry integrity verification"

}

behavior configure_registries "Configure Registries" {
  invariants [diagnostic_determinism, zero_domain_knowledge_core, registry_integrity, registry_api_openness, offline_first_extension_resolution]
  types      [RegistryConfig, CompilerConfig]
  ports      [FileSystem]
  produces   [registries_configured]

  requires {
    specforge_json_parsed "specforge.json has been parsed and registries array is accessible"
    filesystem_available "FileSystem port is available for reading configuration"
  }

  ensures {
    registry_entries_created "All registries array entries are parsed into RegistryConfig objects"
    scope_filters_set "Scope filters are configured for routing @scope/ specifiers"
    no_registries_diagnosed "Empty registries configuration produces I003 info diagnostic"
    no_hardcoded_urls "No registry URLs are compiled-in constants in compiler source"
    registries_configured_emitted "registries_configured event fires after all entries are parsed"
  }

  contract """
    At startup, the compiler MUST parse the registries array from
    specforge.json into RegistryConfig entries. Each entry MUST have an
    alias (unique identifier), url, and optional scope_filter array.
    When resolving extension specifiers, scope_filter MUST route @scope/
    prefixed specifiers to the matching registry. If no registries are
    configured, registry operations (search, fetch) MUST produce an I003
    info diagnostic indicating no registries are available. First-use
    MUST NOT require network access — registries are opt-in configuration.
    The default registry URL MUST be resolved from configuration
    (`specforge.json` registries array or `SPECFORGE_REGISTRY_URL`
    environment variable) — it MUST NOT be a compiled-in constant in
    compiler source. It is NEVER contacted until the user initiates a
    registry operation (search, fetch). Setting `default_registry: false`
    disables it entirely. First-use is always local/offline per P8.
    Setting `registries: []` with `default_registry: false` in specforge.json
    explicitly disables the default public registry, forcing fully offline
    operation.
  """

  verify unit "registries parsed from specforge.json"
  verify unit "scope_filter routes to correct registry"
  verify unit "no registries configured produces I003 info diagnostic"
  verify unit "scope mismatch falls back to default registry URL"
  verify unit "duplicate alias produces warning"
  verify property "registry API schema is published as open specification"
  verify integration "all registries disabled produces fully offline mode"
  verify integration "first specforge init succeeds without any registry authentication"
  verify unit "default public registry is accessible without credentials"
  verify contract "requires/ensures consistency for registry configuration"

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

  requires {
    credential_configured "A RegistryCredential entry exists for the target registry alias"
    registry_client_available "RegistryClient port is available for authentication requests"
  }

  ensures {
    token_resolved "Authentication token is resolved from environment variable or token file"
    auth_header_attached "Resolved token is attached as Authorization header"
    missing_source_diagnosed "Unavailable token source produces ExtensionError diagnostic with guidance"
    double_401_diagnosed "Failed re-resolution after 401 emits E-level diagnostic with login guidance"
    tokens_never_logged "Raw tokens are never logged or stored in specforge.json"
    cache_fallback_on_network_only "Cache fallback triggers only on network-level failures, not auth failures"
    authenticated_emitted "registry_authenticated event fires on successful authentication"
  }

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
  verify unit "both token_env_var and token_file absent produces E-level diagnostic"
  verify unit "403 response produces E-level diagnostic with permission guidance"
  verify unit "unreachable registry with cached extension falls back to cache with I-level diagnostic"
  verify unit "authentication failure (401/403) does not trigger cache fallback"
  verify contract "requires/ensures consistency for registry authentication"

}

behavior retry_registry_request "Retry Registry Request" {
  invariants [registry_integrity, multi_error_collection, credential_secrecy]
  types      [RegistryConfig, RegistryError, ExtensionError]
  ports      [RegistryClient]

  requires {
    registry_request_failed "A registry request has received a retryable response (429 or timeout)"
    registry_client_available "RegistryClient port is available for retry requests"
  }

  ensures {
    exponential_backoff_applied "429 responses trigger retry with exponential backoff (base 1s, max 30s, max 3 retries)"
    timeout_diagnosed "Network timeouts produce ExtensionError diagnostic with retry guidance"
    retries_exhausted_emitted "registry_request_retry_exhausted event fires when max retries exceeded"
  }

  contract """
    When a registry request receives a 429 (rate limited) response, the
    system MUST retry with exponential backoff (base 1s, max 30s, max
    retries 3). When a request times out (network timeout), the system
    MUST produce an ExtensionError diagnostic with retry guidance. Retry
    logic applies to all registry operations (authentication, download,
    search, publish) uniformly.
  """

  produces [registry_request_retry_exhausted]

  verify unit "429 response retries with exponential backoff"
  verify unit "network timeout produces ExtensionError with retry guidance"
  verify unit "max retries exceeded produces final error"
  verify contract "requires/ensures consistency for registry request retry"

}

behavior validate_registry_credentials "Validate Registry Credentials" {
  invariants [registry_integrity, diagnostic_determinism, credential_secrecy]
  types      [RegistryCredential, RegistryConfig, RegistryError]
  ports      [RegistryClient]
  produces   [registry_credentials_validated]

  requires {
    registry_configured "Target registry is configured in specforge.json"
    registry_client_available "RegistryClient port is available for test authentication request"
  }

  ensures {
    valid_credentials_stored "Valid credentials stored as RegistryCredential referencing env var or token file path"
    invalid_credentials_diagnosed "Invalid credentials produce error diagnostic with guidance"
    raw_token_never_stored "Raw token value is never stored in specforge.json"
    credentials_validated_emitted "registry_credentials_validated event fires on successful validation"
  }

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
  verify contract "requires/ensures consistency for registry credential validation"

}

behavior logout_registry "Logout Registry" {
  invariants [registry_integrity, diagnostic_determinism, credential_secrecy]
  types      [RegistryConfig, RegistryCredential, RegistryError]
  ports      [FileSystem]
  produces   [registry_logged_out]

  requires {
    alias_matches_config "The provided alias matches a RegistryConfig entry's alias field"
    filesystem_available "FileSystem port is available for credential removal"
  }

  ensures {
    credential_removed "RegistryCredential entry for the matching alias is removed"
    other_credentials_intact "Credentials for other aliases and scopes remain untouched"
    missing_credential_silent "No credential for the specified alias succeeds silently"
    no_network_requests "No network requests are made during logout"
    logged_out_emitted "registry_logged_out event fires after credential removal"
  }

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
  verify contract "requires/ensures consistency for registry logout"

}

behavior generate_keyword_extension_index "Generate Keyword Extension Index" {
  invariants [registry_integrity]
  types      [RegistryConfig, KeywordExtensionIndex]
  ports      [RegistryClient, FileSystem]
  produces   [keyword_extension_index_generated]

  requires {
    registry_accessible "Extension registry is accessible for enumerating published extensions"
    filesystem_available "FileSystem port is available for writing the index file"
  }

  ensures {
    keyword_mapping_complete "Every known keyword is mapped to its providing extension(s)"
    index_deterministic "Same registry state produces the same index file"
    multi_provider_included "Keywords claimed by multiple extensions list all providers"
    index_written "Index is written to data/keyword-index.json relative to installation directory"
    index_generated_emitted "keyword_extension_index_generated event fires after index is written"
  }

  contract """
    At build time or release time, the system MUST read the extension
    registry to enumerate all published extensions and their declared
    entity keywords. The result MUST be serialized as a KeywordExtensionIndex
    JSON file mapping each known keyword to the extension name that provides
    it. This index is shipped as a bundled data file for use by
    suggest_missing_extensions at runtime. The generation MUST be
    deterministic — the same registry state MUST produce the same index
    file. Keywords claimed by multiple extensions MUST include all
    providing extensions in the mapping. The bundled index is a bootstrap
    convenience for offline E024 diagnostics. It does not represent a
    privileged set of extensions. Users MAY regenerate the index from
    their configured registries via specforge registry refresh-index to
    include extensions from custom registries. The index MUST be written
    to data/keyword-index.json relative to the compiler binary's
    installation directory. The file is embedded at compile time via
    include_bytes! or loaded at runtime from binary_dir/data/keyword-index.json.
    Users MAY override via SPECFORGE_KEYWORD_INDEX environment variable.
  """

  verify unit "index maps each keyword to its providing extension"
  verify unit "index generation is deterministic"
  verify unit "keyword claimed by multiple extensions lists all providers"
  verify unit "empty registry produces empty index"
  verify property "index generation accepts configurable registry list"
  verify contract "requires/ensures consistency for keyword extension index generation"

}

behavior support_private_registries "Support Private Registries" {
  invariants [registry_integrity, wasm_sandbox_integrity, credential_secrecy]
  types      [RegistryConfig, RegistryCredential, TrustLevel, RegistryResponse]
  ports      [RegistryClient]

  requires {
    credentials_configured "Registry has configured credentials for authentication"
    registry_client_available "RegistryClient port is available for authenticated fetching"
  }

  ensures {
    authenticated_before_fetch "Authentication occurs before fetching from private registry"
    scope_filter_respected "Only extensions matching the scope_filter are fetched from authenticated registries"
    trust_level_assigned "Extensions receive appropriate trust level based on registry trust configuration"
    no_auth_leaks "Error messages do not leak authentication details"
  }

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
  verify contract "requires/ensures consistency for private registry support"
  // Observability: error diagnostics for private registry operations delegate
  // to authenticate_registry_request and retry_registry_request for auth and
  // retry details. This behavior owns the scope_filter and trust_level logic.

}
