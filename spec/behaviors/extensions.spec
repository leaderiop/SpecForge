// Extension behaviors — extensions, providers, renderers

use "events/compilation"
use "events/extensions"
use "invariants/core"
use "invariants/extensions"
use "invariants/validation"
use "invariants/wasm"
use "invariants/zero-entity-core"
use "ports/outbound"
use "types/config"
use "types/diagnostics"
use "types/errors"
use "types/wasm"
use "types/zero-entity-core"

// load_extension_manifests is the top-level orchestrator for extension loading.
// It reads each extension's declaration through load_extension_declaration and
// hands them all to build_registries_from_declarations.
behavior load_extension_manifests "Load Extension Manifests" {
  features   [extension_management]
  invariants [
    registry_population_before_validation,
    zero_domain_knowledge_core,
    extension_load_order_determinism,
    offline_first_extension_resolution,
    compilation_pipeline_ordering,
  ]
  category   command
  ports      [FileSystem]
  types      [ExtensionDeclaration, CompilerConfig, ExtensionError]
  consumes   [all_files_parsed]
  produces   [extension_manifests_loaded]
  requires {
    all_files_parsed            "all_files_parsed event has fired, confirming all .spec files have been structurally parsed"
    extensions_config_available "Extensions list in specforge.json is available from the parsed project configuration"
  }
  ensures {
    all_extensions_attempted    "All declared extensions have been attempted for loading"
    loaded_manifests_available  "Successfully loaded manifests are available for registry population"
    failed_extensions_diagnosed "Failed extensions have produced diagnostics"
    loaded_event_fired          "extension_manifests_loaded event fires exactly once after all extensions are processed"
  }
  maintains {
    extension_isolation "A failure loading one extension does not prevent loading of remaining extensions"
  }
  contract   """
    At startup, the compiler MUST read the extensions list from specforge.json
    and read each extension's declaration (load_extension_declaration): its
    entity types, edge types, validation rules and optional peer
    dependencies, from the binary itself. Missing extensions or unloadable
    .wasm binaries MUST produce a diagnostic, not a crash. A builtin loads
    from the binary. Any other entry names an installed extension by its
    bare name (a legacy name@version entry names the same extension): it
    MUST load from .specforge/extensions/<name>/extension.wasm under that
    name, on every surface, only when the binary's hash is the one its
    specforge.lock entry records; a mismatch MUST be refused with E033, and
    an extension enabled but not installed MUST produce E028 naming the
    command that installs it. An entry ending in .wasm names a component
    file instead (path.wasm, or name=path.wasm; a relative path is relative
    to the project root): it MUST load from that file, on every surface,
    under the name the component declares, the one rule the runtime and the
    environment both read an entry by; a name written before = MUST be the
    declared one. A file that does not exist, does not load as a component,
    declares another name than the one written, or declares an extension
    another entry already loads MUST produce E028 naming the entry. A
    specforge.json that is there and is not used as written MUST produce
    the error E069 naming why, one per problem, before any other
    diagnostic. When the file is not readable, not JSON or not a JSON
    object, or its extensions value is not an array, the compile loads no
    extension, and the I002 that follows MUST say that specforge.json
    could not be read, not that no extensions are configured. A key of the
    wrong type (name, version or spec_root not a string, exclude not an
    array) is replaced by its default, and an extensions or exclude item
    that is not a string is ignored; the rest of the file is used. A
    missing specforge.json is the default config and produces no E069.
    This behavior orchestrates: for each extension, it loads its
    binary and reads its declaration once. Once all declarations are loaded
    and the extension_manifests_loaded event is produced, the registry build
    (build_registries_from_declarations) validates them and populates the
    KindRegistry, FieldRegistry, and EdgeRegistry. The grammars and
    body_parsers contribution flags are reserved: nothing reads those
    contributions.
  """
  verify unit "installed extension manifest is loaded"
  verify integration "an extension installed from a registry loads through check"
  verify integration "an enabled extension with no installed binary produces E028 naming the command that installs it"
  verify integration "an entry naming a .wasm file loads that component from disk under the name it declares"
  verify integration "a .wasm file entry that is missing, is not a component, names another extension or repeats a loaded one produces E028 naming the entry"
  verify unit "missing extension produces diagnostic"
  verify unit "a declaration declares entity types and validations"
  verify integration "two extensions loaded and registries populated without collision"
  verify unit "unloadable extension binary produces diagnostic instead of crash"
  verify unit "E069 names why specforge.json can't be used"
  verify unit "E069 names a mistyped key or a non-string item, and the rest of specforge.json is used"
  verify integration "a specforge.json that is there and can't be used produces E069 first and an I002 that names it"
  verify contract "Load Extension Manifests: extension manifest loading holds — all_files_parsed, extensions_config_available, all_extensions_attempted, loaded_manifests_available, failed_extensions_diagnosed, loaded_event_fired, extension_isolation"
}

behavior load_extension_declaration "Load Extension Declaration" {
  features   [extension_management]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   command
  ports      [WasmRuntime]
  types      [ExtensionDeclaration, HandshakeResponse]
  contract   """
    The host MUST read an extension's declaration through one loader: its
    handshake, then every declared describe category (entities, edges,
    shared_fields, enhancements, validation_rules, surfaces, collectors,
    analyzers, passes, feature_flags), whatever its contribution flags say.
    A category that does not parse MUST fail the extension's load (E028)
    naming the category. A describe item key the protocol does not define
    MUST produce W138. The declaration is read once per environment load;
    nothing describes a category again outside it. A handshake whose
    protocol major version differs from the host's MUST fail the
    extension's load (E028), and none of its categories are read.
  """
  verify integration "every builtin's handshake and describe answers match their pinned snapshot byte for byte"
  verify unit "a declaration round-trips through its wire answers unchanged"
  verify unit "a describe category that does not parse fails the load naming the category"
  verify unit "a describe item key the protocol does not define produces W138"
  verify unit "the fields category is every kind's fields, concatenated"
  verify unit "an absent short is the name's last segment"
  verify unit "the SDK's short name reaches the handshake as ext_short"
  verify unit "a short name that is not lowercase kebab case is refused when the extension is built"
  verify unit "a raw category that does not parse panics when the extension is built"
  verify integration "the loader reads the handshake and every describe category once"
  verify integration "an extension that only declares commands registers its commands"
  verify integration "an extension that only declares passes has them in its declaration"
  verify integration "the declared short name reaches the registry build"
  verify integration "an unsupported protocol major version fails the load"
}

behavior build_registries_from_declarations "Build Registries From Declarations" {
  features   [extension_management]
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  category   validation
  types      [ExtensionDeclaration, RegistryBuild]
  produces   [registries_populated]
  contract   """
    The registry build MUST take the loaded declarations, in load order,
    and own everything derived from them: identity and shape (E030: an empty
    name or version, a malformed ext_short), self-consistency (W021), peer
    dependencies (E027), the order of declared passes (W145 when their
    constraints form a cycle, declaration order kept), the kind, field and
    edge registries, the rules and the surfaces. It MUST be pure.

    Its outcomes are stated by registry_build_kinds, registry_build_fields,
    registry_build_edges, registry_build_rules,
    registry_build_declaration_consistency and
    registry_build_peer_dependencies. The build produces
    registries_populated: nothing reads a registry before every loaded
    declaration is in it.
  """
  verify integration "the registry build of the builtins matches its pinned snapshot"
  verify unit "a declaration with an empty name or version produces E030"
  verify unit "a malformed ext_short produces E030"
  verify unit "peer dependencies are checked in the registry build"
  verify unit "declared passes are ordered in the registry build"
  verify unit "a pass constraint cycle produces W145 and keeps declaration order"
  verify integration "a passes description that does not parse fails the extension's load"
  verify unit "a build of no declarations has empty registries, no rules and no diagnostics"
  verify unit "the declarations' own diagnostics come in a fixed order: E030, W021, E027, W145"
  verify integration "every loaded declaration is registered before a compile checks anything"
}

// register_extension_entity_types is a thin delegation wrapper that calls
// registry_build_kinds (behaviors/zero-entity-registries.spec)
// for each loaded extension. The detailed registration semantics — including
// KindRegistry population, field registry setup, and edge type registration —
// are defined in the zero-entity-core behaviors.
behavior register_extension_entity_types "Register Extension Entity Types" {
  features   [extension_management]
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  category   command
  types      [ExtensionDeclaration, KindRegistryEntry]
  consumes   [extension_manifests_loaded]
  produces   [extension_entity_types_registered]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all extension manifests have been loaded and validated"
  }
  ensures {
    kind_registry_populated     "KindRegistry contains entries for all entity kinds from loaded extensions"
    field_registry_populated    "FieldRegistry contains field definitions from loaded extensions"
    edge_registry_populated     "EdgeRegistry contains edge types from loaded extensions"
    registered_event_emitted    "extension_entity_types_registered event fires exactly once after all registries are populated"
    soft_resolution_for_missing "Uninstalled extension kinds produce I004 info diagnostic with suggested extension"
  }
  contract   """
    After loading manifests, the compiler MUST register each extension's
    entity types by delegating to registry_build_kinds
    for kind registration, registry_build_fields for
    field registration, and registry_build_edges for
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
  verify unit "delegates to registry_build_kinds per extension"
  verify unit "unregistered type triggers soft resolution"
  verify unit "KindRegistry records source extension for each kind"
  verify unit "I004 message includes unresolved kind name and suggested extension"
  verify contract "Register Extension Entity Types: extension entity type registration holds — extension_manifests_loaded_fired, kind_registry_populated, field_registry_populated, edge_registry_populated, registered_event_emitted, soft_resolution_for_missing"
}

behavior load_provider_configurations "Load Provider Configurations" {
  features   [provider_based_ref_validation]
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  category   query
  types      [ProviderConfig, CompilerConfig]
  ports      [FileSystem]
  consumes   [extension_manifests_loaded]
  produces   [provider_configured]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming extension manifests are available for provider lookup"
    specforge_json_available         "specforge.json has been parsed and provider blocks are accessible"
  }
  ensures {
    provider_instances_created  "Provider instances are created for all configured provider blocks"
    aliased_instances_distinct  "Multiple instances of the same provider with different aliases are distinct"
    no_hardcoded_schemes        "No provider schemes or kinds are hardcoded in core"
    provider_configured_emitted "provider_configured event fires exactly once after all providers are instantiated"
  }
  contract   """
    The compiler MUST parse provider blocks from specforge.json and
    create provider instances with their configured settings. In
    specforge.json, providers is an array of {scheme, alias, extension,
    settings} entries, kept in declaration order; scheme, alias and
    extension are required, and an entry missing one, or a providers value
    that is not an array, is W118. The core
    MUST NOT hardcode any provider schemes or kinds — all provider
    configuration comes exclusively from specforge.json and extension
    manifests. Multiple instances of the same provider with different
    aliases MUST be supported, each instance with its own scheme.
  """
  verify unit "single provider instance is created"
  verify unit "multiple aliased instances are created"
  verify unit "provider config settings are passed through"
  verify unit "no hardcoded provider schemes exist in core"
  verify contract "Load Provider Configurations: provider configuration loading holds — extension_manifests_loaded_fired, specforge_json_available, provider_instances_created, aliased_instances_distinct, no_hardcoded_schemes, provider_configured_emitted"
}

behavior register_provider_schemes "Register Provider Schemes" {
  features   [provider_based_ref_validation]
  invariants [reference_resolution_completeness, diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [ProviderConfig, ExtensionDeclaration, SchemeRegistryEntry, Diagnostic]
  ports      [WasmRuntime]
  consumes   [provider_configured]
  produces   [provider_schemes_registered]
  requires {
    provider_configured_fired "provider_configured event has fired, confirming provider instances are created"
    wasm_runtime_available    "WasmRuntime port is available for querying provider extension manifests"
  }
  ensures {
    schemes_registered         "All provider schemes are registered as SchemeRegistryEntry entries"
    duplicate_scheme_warned    "Duplicate schemes produce E057 error listing both providers"
    declaration_order_tiebreak "Duplicate scheme conflicts resolved by specforge.json declaration order"
    schemes_registered_emitted "provider_schemes_registered event fires exactly once after all schemes are registered"
  }
  contract   """
    After loading provider configurations from specforge.json, the compiler
    MUST query each provider extension's manifest for the ref schemes and
    kinds it supports. Each scheme MUST be registered as a SchemeRegistryEntry
    so that validate_provider_refs can route refs to the correct provider.
    The core MUST NOT contain any built-in scheme registrations — all
    schemes come exclusively from provider extensions. Schemes already
    registered by another provider MUST produce an E057 error listing
    both providers; the provider declared first in the specforge.json
    providers array MUST win the scheme registration as a deterministic
    tiebreaker. Unresolvable provider extensions MUST produce an
    ExtensionError diagnostic.
  """
  verify unit "provider schemes registered from manifest"
  verify unit "duplicate scheme from two providers produces E057"
  verify unit "duplicate scheme resolved by specforge.json declaration order"
  verify unit "unresolvable provider extension produces ExtensionError"
  verify unit "no built-in schemes exist before provider loading"
  verify integration "Wasm-based provider scheme registered and validates ref"
  verify contract "Register Provider Schemes: provider scheme registration holds — provider_configured_fired, wasm_runtime_available, schemes_registered, duplicate_scheme_warned, declaration_order_tiebreak, schemes_registered_emitted"
}

behavior validate_provider_refs "Validate Provider Refs" {
  features   [provider_based_ref_validation]
  invariants [reference_resolution_completeness, zero_domain_knowledge_core]
  category   validation
  types      [SchemeRegistryEntry, Diagnostic]
  ports      [RefValidator]
  consumes   [provider_schemes_registered]
  produces   [provider_ref_validated]
  requires {
    provider_schemes_registered_fired "provider_schemes_registered event has fired, confirming scheme-to-provider routing is established"
    ref_validator_available           "RefValidator port is available for delegating validation"
  }
  ensures {
    known_scheme_delegated   "Refs with known schemes are delegated to the corresponding provider"
    unknown_scheme_diagnosed "Refs with unknown schemes emit I005 diagnostic"
    ref_validated_emitted    "provider_ref_validated event fires for each validated ref"
  }
  contract   """
    When a ref entity uses a registered scheme, the compiler MUST
    delegate validation to the corresponding provider. The provider
    MUST validate the kind and identifier format. Unknown schemes
    MUST emit I005.
  """
  verify unit "known scheme delegates to provider"
  verify unit "unknown scheme emits I005"
  verify unit "provider validates identifier format"
  verify unit "no built-in ref validation logic exists in core"
  verify contract "Validate Provider Refs: provider ref validation holds — provider_schemes_registered_fired, ref_validator_available, known_scheme_delegated, unknown_scheme_diagnosed, ref_validated_emitted"
}

// remove_extension is the user-facing CLI entry point for extension removal.
// It delegates to uninstall_wasm_extension (behaviors/wasm-lifecycle.spec) for the full
// Wasm lifecycle cleanup. This behavior owns the CLI interaction and post-removal
// diagnostic messaging; uninstall_wasm_extension owns the implementation.
behavior remove_extension "Remove Extension" {
  features   [extension_management]
  invariants [
    reference_resolution_completeness,
    zero_domain_knowledge_core,
    peer_dependency_satisfaction,
  ]
  category   command
  types      [CompilerConfig, ExtensionError, Diagnostic, UnknownKindError]
  ports      [FileSystem]
  produces   [extension_removed]
  requires {
    extension_installed  "Target extension is present in specforge.json extensions list"
    filesystem_available "FileSystem port is available for config and binary operations"
  }
  ensures {
    extension_entry_removed   "Extension entry removed from specforge.json via uninstall_wasm_extension"
    spec_files_unchanged      "Existing .spec files are not modified by the removal"
    extension_removed_emitted "extension_removed event fires exactly once after successful removal"
  }
  contract   """
    When specforge remove <extension-specifier> is invoked, the system MUST
    delegate to uninstall_wasm_extension (behaviors/wasm-lifecycle.spec) for the full
    Wasm lifecycle cleanup: removing the extension entry from specforge.json,
    deleting the .wasm binary, updating
    specforge.lock, and checking peer dependencies. A builtin extension has
    no binary or lock entry: removing it removes its specforge.json entry.
    A .wasm file entry (read by the rule load_extension_manifests loads it
    by) MUST be removable by the name its component declares, as the
    extensions listing names it, or by the entry as specforge.json writes
    it (or its path); removing it MUST only drop that entry from
    specforge.json, never delete the file nor touch specforge.lock, with
    the same dependents (E027) and orphan checks as any extension. A name
    more than one specforge.json entry enables MUST be refused as
    ambiguous (extension_conflict), naming the entries and changing
    nothing; a name no entry, lock entry or builtin matches is
    extension_not_found. Every refusal MUST be decided before anything is
    written, and a specforge.json the compile could not read refuses every
    removal (config_invalid), changing nothing; specforge.json is written
    before specforge.lock and the binary.
    Removing an extension that another loaded or installed extension
    requires as a non-optional peer MUST fail with E027 naming the
    dependents, unless --force is given. The CLI and the MCP
    remove_extension tool MUST run the same removal. This behavior is the
    user-facing CLI entry point; uninstall_wasm_extension handles the
    implementation. Existing .spec files using the extension's entities
    MUST NOT be modified. On the next compilation, entity blocks using the
    removed extension's keywords MUST produce E024 (unknown entity kind)
    since the keyword is no longer in the KindRegistry. Reference list
    entries pointing to those entities MUST produce E003 (dangling
    reference). The user MUST either reinstall the extension or remove
    the affected entity blocks. Its JSON output lists the files it wrote or
    deleted as files_written.
  """
  verify unit "delegates to uninstall_wasm_extension for lifecycle cleanup"
  verify unit "extension is removed from extensions list"
  verify unit "removed extension keywords produce E024 on next compile"
  verify unit ".spec files are not modified by removal"
  verify contract "Remove Extension: extension removal holds — extension_installed, filesystem_available, extension_entry_removed, spec_files_unchanged, extension_removed_emitted"
  verify unit "specforge remove for non-existent extension reports error"
  verify integration "remove --format json lists the files it wrote in files_written"
  verify unit "specforge remove with no lock file reports error"
  verify integration "removing an installed extension drops its specforge.json entry"
  verify integration "removing an extension another installed extension requires fails with E027 unless --force"
  verify integration "a .wasm file entry is removed by the name it declares or by its entry as written, leaving its file in place"
  verify integration "a name more than one specforge.json entry enables is refused as ambiguous, naming the entries"
  verify integration "removing a .wasm file entry another extension requires fails with E027 unless --force"
  verify integration "a removal with an unreadable specforge.json is config_invalid and changes nothing"
}

// Read-only query. (produces [] declared below; no event of its own.)
behavior list_installed_extensions "List Installed Extensions" {
  features   [extension_management]
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [ExtensionDeclaration, KindRegistryEntry]
  requires {
    kind_registry_ready "KindRegistry is populated with entity kinds from loaded extensions"
  }
  ensures {
    all_extensions_listed  "All installed extensions are included in the output"
    entity_counts_included "Each extension entry includes entity count and registered entity types"
    output_deterministic   "Output order is alphabetical by extension name"
  }
  contract   """
    When specforge extensions is invoked, the system MUST list all installed
    extensions with their name, version, entity count, and registered entity types.
    The listing MUST query the KindRegistry to enumerate entity kinds per
    extension. Each entry MUST carry its source (builtin, registry,
    local:<path>, or file:<path> for a .wasm file entry of specforge.json,
    listed under the name its component declares) and its status (loaded, not_loaded, not_configured). The
    CLI and the MCP extensions tool MUST list the same entries.
    Output order MUST be deterministic (alphabetical by extension name).
  """
  verify unit "list shows all installed extensions"
  verify unit "list includes entity counts and entity types"
  verify unit "output order is deterministic"
  verify integration "a .wasm file entry is listed under the name it declares, loaded, with source file:<path>"
  verify integration "the CLI and the MCP extensions tool list the same entries"
  verify contract "List Installed Extensions: extension listing holds — kind_registry_ready, all_extensions_listed, entity_counts_included, output_deterministic"
}

// Read-only query. (produces [] declared below; no event of its own.)
behavior list_configured_providers "List Configured Providers" {
  features   [provider_based_ref_validation]
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  category   query
  types      [ProviderConfig, SchemeRegistryEntry]
  requires {
    scheme_registry_ready "SchemeRegistryEntry set is populated from provider registration"
  }
  ensures {
    all_providers_listed       "All configured providers are included in the output"
    schemes_and_kinds_included "Each provider entry includes registered schemes and supported kinds"
    aliases_shown_separately   "Providers with multiple instances show each alias separately"
    output_deterministic       "Output order is deterministic"
  }
  contract   """
    When specforge providers is invoked, the system MUST list all configured
    providers with their alias, extension, registered schemes, and supported
    kinds. The listing MUST query the SchemeRegistryEntry set to show which
    schemes each provider handles, with each provider's status there:
    registered, extension_not_loaded, not_a_provider or scheme_taken.
    Providers with multiple instances MUST show each alias separately.
    Output order MUST be deterministic (declaration order). The CLI and the
    MCP providers tool MUST list the same entries.
  """
  verify unit "list shows all configured providers"
  verify unit "list includes scheme and kind registrations"
  verify unit "multiple aliases shown separately"
  verify unit "output order is deterministic"
  verify integration "the CLI and the MCP providers tool list the same entries"
  verify contract "List Configured Providers: provider listing holds — scheme_registry_ready, all_providers_listed, schemes_and_kinds_included, aliases_shown_separately, output_deterministic"
}

// The management operations are operations over the project view, as the
// read views are (ADR 0015, "Management operations").
behavior management_operations_over_the_project_view "Management Operations over the Project View" {
  features   [extension_management, mcp_project_management_tools]
  invariants [diagnostic_determinism, zero_domain_knowledge_core]
  category   command
  types      [ExtensionDeclaration, Diagnostic]
  ports      [CompilerApi, McpProtocol, FileSystem]
  requires {
    project_compiled "A compiled project or a project session supplies the project view"
  }
  ensures {
    one_project_read "Each operation reads the config, the enabled entries, the lock, the loaded declarations and the reported diagnostics of the compile behind its view, never specforge.json or specforge.lock again"
    root_for_disk    "Whatever an operation reads or writes on disk is at the view's root; without a root it refuses with no_project, except the listings and doctor"
  }
  contract   """
    The extensions listing, the providers listing, doctor, remove,
    collect, and inference progress and gaps MUST each be one operation
    over the project view and a request, shared by the CLI and MCP; a
    surface builds the view from the project it holds, maps its arguments
    and renders the outcome. An operation MUST read the project's config,
    what each specforge.json entry enabled, the specforge.lock the compile
    read (absent, read, or unreadable with its E033 problem), the loaded
    declarations and what the surface reports for the project from the
    view, never by reading specforge.json or specforge.lock again. The
    installed binaries, the source files and the recorded test report MUST
    be read and written at the view's root; specforge.lock is written
    there, at the one path the environment reads it from. Without a root, remove, collect and inference
    progress and gaps MUST refuse with no_project; the listings list what
    the view enabled and loaded, and doctor skips the installation checks.
    add and update run before or instead of a compile, so they read
    specforge.json themselves, through the function the compile reads it
    with; add, update and remove MUST refuse a specforge.json the compile
    reports as E069 and an edit cannot go around, with one refusal
    (config_invalid, the E069 reason as its message), before they write
    anything.
  """
  verify unit "an operation that reads or writes the project on disk refuses a view without a root"
  verify unit "the extensions listing reads the config entries from the view, never specforge.json again"
  verify unit "doctor reads the diagnostics its view reports"
  verify unit "list, doctor and remove read the lock the compile read, once"
  verify integration "add, update and remove refuse an unusable specforge.json with one refusal, before they write"
  verify unit "collect maps test results to the entities of its view"
  verify contract "Management Operations over the Project View: management operations hold — project_compiled, one_project_read, root_for_disk"
}

// Called imperatively by validate_provider_refs (which consumes provider_schemes_registered).
// Depends on SchemeRegistryEntry data populated during provider registration.
behavior validate_ref_target_format "Validate Ref Target Format" {
  features   [provider_based_ref_validation]
  invariants [reference_resolution_completeness, zero_domain_knowledge_core, diagnostic_determinism]
  category   validation
  types      [Diagnostic, SchemeRegistryEntry]
  ports      [RefValidator]
  requires {
    scheme_registry_populated "SchemeRegistryEntry data is populated from provider registration"
    ref_validator_available   "RefValidator port is available for identifier validation"
  }
  ensures {
    valid_identifier_passes        "Valid ref identifiers pass validation without diagnostics"
    malformed_identifier_diagnosed "Malformed ref identifiers produce E011 diagnostic"
  }
  contract   """
    When a provider is installed, the validator MUST check that ref
    identifiers match the provider's expected pattern by delegating to the
    RefValidator port's validateIdentifier method. Malformed identifiers
    MUST produce an E011 diagnostic.
  """
  verify unit "valid ref identifier passes"
  verify unit "malformed ref identifier produces E011"
  verify unit "no built-in format patterns exist in core"
  verify contract "Validate Ref Target Format: ref target format validation holds — scheme_registry_populated, ref_validator_available, valid_identifier_passes, malformed_identifier_diagnosed"
}

// Called imperatively by validate_provider_refs (which consumes provider_schemes_registered).
// Depends on SchemeRegistryEntry data populated during provider registration.
behavior validate_provider_kinds "Validate Provider Kinds" {
  features   [provider_based_ref_validation]
  invariants [reference_resolution_completeness, zero_domain_knowledge_core, diagnostic_determinism]
  category   validation
  types      [Diagnostic, SchemeRegistryEntry]
  ports      [RefValidator]
  requires {
    scheme_registry_populated "SchemeRegistryEntry data is populated from provider registration"
    ref_validator_available   "RefValidator port is available for kind validation"
  }
  ensures {
    valid_kind_passes      "Valid scheme and kind combinations pass validation"
    unknown_kind_diagnosed "Unknown kind for a known scheme produces E013 diagnostic listing valid kinds"
  }
  contract   """
    When a ref uses a known scheme but an unregistered kind, the validator
    MUST delegate to the RefValidator port's validateKind method and
    MUST produce an E013 diagnostic listing the valid kinds for that scheme.
  """
  verify unit "valid scheme and kind passes"
  verify unit "valid scheme with unknown kind produces E013"
  verify unit "no built-in kind registrations exist in core"
  verify contract "Validate Provider Kinds: provider kind validation holds — scheme_registry_populated, ref_validator_available, valid_kind_passes, unknown_kind_diagnosed"
}

// -- Registry Behaviors -----

behavior resolve_registry_source "Resolve Registry Source" {
  features   [extension_registry]
  invariants [
    registry_integrity,
    multi_error_collection,
    extension_operation_atomicity,
    offline_first_extension_resolution,
  ]
  category   query
  types      [RegistryConfig, RegistryResponse, CompilerConfig, ExtensionError]
  ports      [RegistryClient]
  consumes   [registries_configured]
  produces   [registry_resolved]
  requires {
    registries_configured_fired "registries_configured event has fired, confirming registry entries are parsed and available"
    registry_client_available   "RegistryClient port is available for network queries"
  }
  ensures {
    scope_routed              "Scope-prefixed specifiers are routed to the matching scope-specific registry"
    default_fallback_used     "Specifiers with no matching scope fall back to the default registry"
    network_error_diagnosed   "Network errors produce ExtensionError diagnostic with retry guidance"
    registry_resolved_emitted "registry_resolved event fires on successful resolution"
  }
  contract   """
    When resolving an extension specifier with @scope/name format, the
    system MUST query the configured registry for that scope. Scope routing
    MUST use the scope_filter field from RegistryConfig entries in
    specforge.json. If no scope-specific registry matches, the system MUST
    fall back to the default registry. "The default registry" means the
    registries entry marked `default_registry: true` in specforge.json:
    SpecForge ships no registry, so no registry URL is a constant in source.
    Network errors MUST produce an ExtensionError diagnostic with retry guidance.
  """
  verify unit "scope-specific registry queried for matching scope"
  verify unit "default registry used when no scope filter matches"
  verify unit "network error produces ExtensionError with retry guidance"
  verify unit "successful query returns RegistryResponse"
  verify integration "unreachable scope-specific registry falls back to next scope"
  verify unit "a fetch requests the name and version it was given, from the registry it was given"
  verify contract "Resolve Registry Source: registry source resolution holds — registries_configured_fired, registry_client_available, scope_routed, default_fallback_used, network_error_diagnosed, registry_resolved_emitted"
}

behavior search_registry "Search Registry" {
  features   [extension_registry]
  invariants [diagnostic_determinism, multi_error_collection, offline_first_extension_resolution]
  category   query
  types      [RegistryConfig, RegistrySearchResult, RegistryResponse, CompilerConfig, ContributesSummary]
  ports      [RegistryClient]
  produces   [registry_search_completed]
  requires {
    registries_available      "At least one registry is configured in specforge.json"
    registry_client_available "RegistryClient port is available for network queries"
  }
  ensures {
    all_registries_queried   "All configured registries are queried with the search term"
    results_deduplicated     "Results from multiple registries are deduplicated by name + version"
    output_deterministic     "Output is sorted by relevance score then extension name"
    search_completed_emitted "registry_search_completed event fires after results are collected"
  }
  maintains {
    partial_failure_resilience "Error from one registry does not abort search of remaining registries"
  }
  contract   """
    When specforge search is invoked, the system MUST query all configured
    registries with the search term. Results MUST be filterable by
    contribution type (entities, validators, renderers, providers,
    collectors, prompts, parsers). Results from multiple registries MUST be merged and
    deduplicated using a composite key of name + version — when the
    same name + version appears from multiple registries, the first
    registry in specforge.json declaration order wins. Output MUST be
    deterministic — sorted by relevance score then extension name.
    With no registry configured, search MUST make no network call and MUST
    fail with E063, whose suggestion names the specforge.json registries key.
  """
  verify unit "with no registry configured, search makes no network call and reports how to configure one"
  verify unit "queries all configured registries"
  verify unit "filters by contribution type"
  verify unit "deduplicates results across registries"
  verify unit "output is deterministic"
  verify unit "error from one registry does not abort search of others"
  verify contract "Search Registry: registry search holds — registries_available, registry_client_available, all_registries_queried, results_deduplicated, output_deterministic, search_completed_emitted, partial_failure_resilience"
}

// CLI entry point: `specforge publish`. Delegates Wasm binary packaging
// to publish_wasm_extension in behaviors/wasm-lifecycle.spec.
behavior publish_to_registry "Publish to Registry" {
  features   [extension_registry]
  invariants [registry_integrity, multi_error_collection, credential_secrecy]
  category   command
  types      [ExtensionDeclaration, RegistryConfig, ExtensionError]
  ports      [RegistryClient, FileSystem]
  produces   [extension_published_to_registry]
  requires {
    declaration_valid         "The binary loads, and the registry build of its declaration alone reports no error"
    wasm_binary_available     "The .wasm component named, or the one its crate directory builds, exists on the filesystem"
    registry_client_available "RegistryClient port is available for upload"
    credentials_available     "Authentication credentials are available for the target registry"
  }
  ensures {
    sha256_computed            "SHA256 hash of .wasm binary is computed and included in the upload"
    duplicate_version_rejected "Duplicate version numbers are rejected unless --force is provided"
    registry_url_returned      "Successful publish returns the registry URL for the published version"
    published_event_emitted    "extension_published_to_registry event fires on successful publish"
  }
  contract   """
    When specforge publish is invoked with a registry target, the system
    MUST load the binary's declaration, refuse it when its registry build
    reports an error, and upload the declaration as the package's manifest,
    with the .wasm binary and its SHA256 hash, before any network call
    deciding whether to refuse; the request MUST be authenticated. The
    registry MUST refuse a manifest that is not an extension declaration,
    and takes the description and keywords it shows from the declaration. Duplicate version numbers MUST be rejected unless --force is
    provided. Successful publish MUST return the registry URL for the
    published version. With no registry configured, publish MUST make no
    network call and MUST fail with E063, whose suggestion names the
    specforge.json registries key.
  """
  verify unit "with no registry configured, publish makes no network call and reports how to configure one"
  verify unit "the declaration is validated before publish"
  verify integration "publish derives the stored declaration from the binary"
  verify unit "publish refuses a binary whose declaration has errors before any network call"
  verify integration "the registry refuses a manifest that is not an extension declaration"
  verify integration "the registry takes a package's description and keywords from its declaration"
  verify unit "SHA256 computed and included in upload"
  verify unit "duplicate version rejected without --force"
  verify unit "successful publish returns registry URL"
  verify unit "unauthenticated publish produces ExtensionError"
  verify unit "the registry refuses a name or version that is not a package name or version"
  verify contract "Publish to Registry: registry publishing holds — declaration_valid, wasm_binary_available, registry_client_available, credentials_available, sha256_computed, duplicate_version_rejected, registry_url_returned, published_event_emitted"
}

behavior verify_registry_integrity "Verify Registry Integrity" {
  features   [extension_registry]
  invariants [registry_integrity, wasm_compile_cache_integrity, offline_first_extension_resolution]
  category   validation
  types      [RegistryResponse, LockFileEntry, TrustLevel, ExtensionError]
  ports      [FileSystem]
  produces   [registry_integrity_verified]
  requires {
    wasm_binary_downloaded      "A .wasm binary has been downloaded from a registry"
    registry_response_available "RegistryResponse with declared SHA256 hash is available"
  }
  ensures {
    hash_verified              "SHA256 hash of downloaded binary matches the declared hash"
    mismatch_aborts            "Hash mismatch produces hard error and aborts installation"
    trust_level_assigned       "Trust level is deterministically assigned based on source type"
    lock_file_updated          "SHA256 hash and trust level are recorded in specforge.lock"
    integrity_verified_emitted "registry_integrity_verified event fires on successful verification"
  }
  contract   """
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
  verify contract "Verify Registry Integrity: registry integrity verification holds — wasm_binary_downloaded, registry_response_available, hash_verified, mismatch_aborts, trust_level_assigned, lock_file_updated, integrity_verified_emitted, receive"
}

// Package trust after the SHA256 check (docs/registry-trust.md). The
// registry is not the trust anchor: these checks use only the bytes
// downloaded, the metadata and manifest served, and the user's pins.

behavior check_registry_reply "Check Registry Reply" {
  features   [extension_registry]
  invariants [registry_reply_binding, registry_integrity]
  category   validation
  types      [RegistryResponse, ExtensionError]
  ports      [RegistryClient]
  requires {
    reply_received "The registry answered a request for name@version and its download passed the SHA256 check"
  }
  ensures {
    reply_names_request    "A reply naming another package or version than the one requested is refused with R-TRUST-004"
    key_id_consistent      "A reply whose key id differs from the key id inside its signature is refused with R-TRUST-004"
    manifest_names_request "A served manifest naming another package or version is refused with R-TRUST-004"
    manifest_fails_closed  "A missing or unreadable served manifest is refused with R-OPS-004, never read as declaring no peers"
    peers_from_manifest    "The peers the served declaration declares are the ones the ADR-0001 diamond gate checks"
    declaration_matches    "A binary whose declaration differs from the served one is refused with R-TRUST-004 naming the first differing category"
  }
  contract   """
    Before a registry package's signature is checked or its key pinned,
    the system MUST refuse the reply when it describes another package or
    version than the one requested, or when its manifest does: the
    signature covers the name and version the reply carries, and the pin
    is keyed by name, so an answer for another package would otherwise be
    verified, pinned and installed in its place (R-TRUST-004). A reply
    whose key id differs from the one inside its signature MUST be
    refused (R-TRUST-004). The served manifest is the package's
    declaration, whose peers decide the diamond gate before anything is
    loaded: a missing manifest, or one that isn't a readable extension
    declaration (one published before declarations, carrying
    manifestVersion, is refused with a suggestion to re-publish it), MUST
    be refused (R-OPS-004) rather than read as declaring no peers. Once
    loaded, the binary's declaration MUST equal the served one, else the
    package is refused (R-TRUST-004) naming the first differing category.
    A refused reply pins no key and installs nothing.
  """
  verify integration "a reply for another package is refused and pins nothing"
  verify integration "a key id the signature does not carry is refused"
  verify integration "a manifest that cannot be read is refused and pins nothing"
  verify integration "the peers the served manifest declares reach the package"
  verify unit "add refuses a package whose binary declares other than its published declaration"
  verify unit "the diamond gate decides on the published declaration's peers"
  verify integration "a package published with a legacy manifest is refused with a re-publish suggestion"
}

behavior verify_publisher_signature "Verify Publisher Signature" {
  features   [extension_registry]
  invariants [publisher_trust, registry_integrity]
  category   validation
  types      [RegistryResponse, ExtensionError]
  ports      [RegistryClient]
  requires {
    reply_checked "The registry reply passed the SHA256 check and names the package requested"
  }
  ensures {
    broken_signature_refused "A signature that is malformed or doesn't verify is refused with R-TRUST-002, even with --allow-unsigned"
    unsigned_refused         "An unsigned package is refused with R-TRUST-001 unless --allow-unsigned is given"
    unsigned_pins_nothing    "An unsigned package accepted with --allow-unsigned pins no key"
  }
  contract   """
    A registry package's Ed25519 publisher signature MUST be verified
    over the canonical payload {name, version, wasmSha256, manifestSha256,
    signedAt} re-derived from the downloaded Wasm bytes and the manifest
    the registry serves, with the public key carried in the signature
    object. A malformed signature, or one that doesn't verify (tampered
    binary or swapped manifest), MUST be refused with R-TRUST-002, and
    --allow-unsigned MUST NOT bypass it. A package with no signature MUST
    be refused with R-TRUST-001 unless the user passes --allow-unsigned;
    one accepted that way pins no key.
  """
  verify integration "a signature over other bytes is refused even with --allow-unsigned"
  verify integration "a swapped manifest breaks the signature"
  verify integration "a malformed signature object is refused"
  verify integration "an unsigned package is refused without --allow-unsigned"
  verify integration "an unsigned package is accepted with --allow-unsigned and pins no key"
}

behavior pin_publisher_key "Pin Publisher Key" {
  features   [extension_registry]
  invariants [publisher_trust]
  category   command
  types      [RegistryResponse, LockFileEntry, ExtensionError]
  ports      [FileSystem]
  requires {
    signature_verified "The package's publisher signature verified"
  }
  ensures {
    first_key_pinned    "The key of the first verified install of a package is pinned in the known-keys store"
    same_key_accepted   "A later install signed by the pinned key is accepted unchanged"
    changed_key_refused "A key other than the pinned one is refused with R-TRUST-003 and both key ids, unless the user consents"
    consent_repins      "Consent to a key change (--yes, or yes at the prompt) re-pins the new key"
    denied_key_refused  "A key on denied_keys is refused with R-TRUST-005, even with consent"
  }
  contract   """
    Trust on first use: the first time a signed package verifies, its
    publisher key id MUST be pinned for the package name in the user's
    known-keys store, and later installs signed by that key MUST be
    accepted. A package signed by another key MUST be refused with
    R-TRUST-003 naming both key ids unless the user consents (--yes, an
    interactive yes, or the key on trusted_keys); --allow-unsigned is not
    consent. Consent MUST re-pin the new key. A key on denied_keys MUST be
    refused with R-TRUST-005 whatever the consent. A refused package
    leaves the pin as it was.
  """
  verify integration "the key of the first verified install is pinned and accepted again"
  verify integration "a package signed by another key than the pinned one is refused"
  verify integration "consent to a key change re-pins the new key"
  verify integration "a denied key is refused even with consent"
}

behavior configure_registries "Configure Registries" {
  features   [extension_registry]
  invariants [
    diagnostic_determinism,
    zero_domain_knowledge_core,
    registry_integrity,
    registry_api_openness,
    offline_first_extension_resolution,
  ]
  category   command
  types      [RegistryConfig, CompilerConfig]
  ports      [FileSystem]
  produces   [registries_configured]
  requires {
    specforge_json_parsed "specforge.json has been parsed and registries array is accessible"
    filesystem_available  "FileSystem port is available for reading configuration"
  }
  ensures {
    registry_entries_created      "All registries array entries are parsed into RegistryConfig objects"
    scope_filters_set             "Scope filters are configured for routing @scope/ specifiers"
    no_registries_diagnosed       "Empty registries configuration produces I003 info diagnostic"
    no_hardcoded_urls             "No registry URLs are compiled-in constants in compiler source"
    registries_configured_emitted "registries_configured event fires after all entries are parsed"
  }
  contract   """
    At startup, the compiler MUST parse the registries array from
    specforge.json into RegistryConfig entries. Each entry MUST have an
    alias (unique identifier), url, and optional scope_filter array.
    When resolving extension specifiers, scope_filter MUST route @scope/
    prefixed specifiers to the matching registry. A configuration with no
    registries array, or with no entry marked `default_registry: true`,
    MUST produce an I003 info diagnostic. SpecForge ships no registry: the
    registries array in specforge.json is the only source of registry URLs,
    and no registry URL is a compiled-in constant (SpecForge does not own
    the specforge.dev domain). A registry is NEVER contacted until the user
    initiates a registry operation. With no registry configured, the
    registry operations (add from a registry, update, search, publish,
    login) MUST make no network call and MUST fail with an E063 diagnostic
    whose suggestion names the specforge.json registries key. Builtin
    extensions and local .wasm files MUST install with no registry
    configured. First-use MUST NOT require network access — registries are
    opt-in configuration, and first use is always local/offline per P8.
  """
  verify unit "registries parsed from specforge.json"
  verify unit "scope_filter routes to correct registry"
  verify unit "no registries configured produces I003 info diagnostic"
  verify unit "scope mismatch falls back to default registry URL"
  verify unit "duplicate alias produces warning"
  verify property "registry API schema is published as open specification"
  verify integration "all registries disabled produces fully offline mode"
  verify integration "first specforge init succeeds without any registry authentication"
  verify unit "builtins and local .wasm files install with no registry configured"
  verify unit "no registry URL on the specforge.dev domain is compiled into non-test source"
  verify contract "Configure Registries: registry configuration holds — specforge_json_parsed, filesystem_available, registry_entries_created, scope_filters_set, no_registries_diagnosed, no_hardcoded_urls, registries_configured_emitted"
}

// ── Registry Authentication ──────────────────────────────────

// Called imperatively during registry operations (resolve, search, publish) —
// not event-driven. Auth is request-time, triggered by 401 responses or
// pre-configured credentials.
behavior authenticate_registry_request "Authenticate Registry Request" {
  features   [registry_authentication]
  invariants [
    registry_integrity,
    multi_error_collection,
    credential_secrecy,
    offline_first_extension_resolution,
  ]
  category   command
  types      [RegistryConfig, RegistryCredential, ExtensionError, RegistryError, AuthMethod]
  ports      [RegistryClient]
  produces   [registry_authenticated]
  requires {
    credential_configured     "A RegistryCredential entry exists for the target registry alias"
    registry_client_available "RegistryClient port is available for authentication requests"
  }
  ensures {
    token_resolved                 "Authentication token is resolved from environment variable or token file"
    auth_header_attached           "Resolved token is attached as Authorization header"
    missing_source_diagnosed       "Unavailable token source produces ExtensionError diagnostic with guidance"
    double_401_diagnosed           "Failed re-resolution after 401 emits E-level diagnostic with login guidance"
    tokens_never_logged            "Raw tokens are never logged or stored in specforge.json"
    cache_fallback_on_network_only "Cache fallback triggers only on network-level failures, not auth failures"
    authenticated_emitted          "registry_authenticated event fires on successful authentication"
  }
  contract   """
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
  verify contract "Authenticate Registry Request: registry authentication holds — credential_configured, registry_client_available, token_resolved, auth_header_attached, missing_source_diagnosed, double_401_diagnosed, tokens_never_logged, cache_fallback_on_network_only, authenticated_emitted"
}

behavior retry_registry_request "Retry Registry Request" {
  features   [registry_authentication]
  invariants [registry_integrity, multi_error_collection, credential_secrecy]
  category   command
  types      [RegistryConfig, RegistryError, ExtensionError]
  ports      [RegistryClient]
  requires {
    registry_request_failed   "A registry request has received a retryable response (429 or timeout)"
    registry_client_available "RegistryClient port is available for retry requests"
  }
  ensures {
    exponential_backoff_applied "429 responses trigger retry with exponential backoff (base 1s, max 30s, max 3 retries)"
    timeout_diagnosed           "Network timeouts produce ExtensionError diagnostic with retry guidance"
    retries_exhausted_emitted   "registry_request_retry_exhausted event fires when max retries exceeded"
  }
  contract   """
    When a registry request receives a 429 (rate limited) response, the
    system MUST retry with exponential backoff (base 1s, max 30s, max
    retries 3). When a request times out (network timeout), the system
    MUST produce an ExtensionError diagnostic with retry guidance. Retry
    logic applies to all registry operations (authentication, download,
    search, publish) uniformly.
  """
  produces   [registry_request_retry_exhausted]
  verify unit "429 response retries with exponential backoff"
  verify unit "network timeout produces ExtensionError with retry guidance"
  verify unit "max retries exceeded produces final error"
  verify contract "Retry Registry Request: registry request retry holds — registry_request_failed, registry_client_available, exponential_backoff_applied, timeout_diagnosed, retries_exhausted_emitted"
}

behavior validate_registry_credentials "Validate Registry Credentials" {
  features   [registry_authentication]
  invariants [registry_integrity, diagnostic_determinism, credential_secrecy]
  category   validation
  types      [RegistryCredential, RegistryConfig, RegistryError]
  ports      [RegistryClient]
  produces   [registry_credentials_validated]
  requires {
    registry_configured       "Target registry is configured in specforge.json"
    registry_client_available "RegistryClient port is available for test authentication request"
  }
  ensures {
    valid_credentials_stored      "Valid credentials stored as RegistryCredential referencing env var or token file path"
    invalid_credentials_diagnosed "Invalid credentials produce error diagnostic with guidance"
    raw_token_never_stored        "Raw token value is never stored in specforge.json"
    credentials_validated_emitted "registry_credentials_validated event fires on successful validation"
  }
  contract   """
    When specforge registry login is invoked, the system MUST validate the
    provided credentials against the target registry by making an authenticated
    test request. Valid credentials MUST be stored as a RegistryCredential
    entry referencing only the environment variable name or token file path —
    never the raw token value. The system MUST confirm successful authentication
    with an info message including the registry alias and authenticated scope.
    With no registry configured, login MUST make no network call and MUST
    fail with E063, whose suggestion names the specforge.json registries key.
  """
  verify unit "with no registry configured, login makes no network call and reports how to configure one"
  verify unit "valid credentials stored as RegistryCredential reference"
  verify unit "invalid credentials produce error with guidance"
  verify unit "raw token never stored in specforge.json"
  verify unit "success message includes registry alias and scope"
  verify contract "Validate Registry Credentials: registry credential validation holds — registry_configured, registry_client_available, valid_credentials_stored, invalid_credentials_diagnosed, raw_token_never_stored, credentials_validated_emitted"
}

behavior logout_registry "Logout Registry" {
  features   [registry_authentication]
  invariants [registry_integrity, diagnostic_determinism, credential_secrecy]
  category   command
  types      [RegistryConfig, RegistryCredential, RegistryError]
  ports      [FileSystem]
  produces   [registry_logged_out]
  requires {
    alias_matches_config "The provided alias matches a RegistryConfig entry's alias field"
    filesystem_available "FileSystem port is available for credential removal"
  }
  ensures {
    credential_removed        "RegistryCredential entry for the matching alias is removed"
    other_credentials_intact  "Credentials for other aliases and scopes remain untouched"
    missing_credential_silent "No credential for the specified alias succeeds silently"
    no_network_requests       "No network requests are made during logout"
    logged_out_emitted        "registry_logged_out event fires after credential removal"
  }
  contract   """
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
  verify contract "Logout Registry: registry logout holds — alias_matches_config, filesystem_available, credential_removed, other_credentials_intact, missing_credential_silent, no_network_requests, logged_out_emitted"
}

behavior support_private_registries "Support Private Registries" {
  features   [registry_authentication]
  invariants [registry_integrity, wasm_sandbox_integrity, credential_secrecy]
  category   command
  types      [RegistryConfig, RegistryCredential, TrustLevel, RegistryResponse]
  ports      [RegistryClient]
  requires {
    credentials_configured    "Registry has configured credentials for authentication"
    registry_client_available "RegistryClient port is available for authenticated fetching"
  }
  ensures {
    authenticated_before_fetch "Authentication occurs before fetching from private registry"
    scope_filter_respected     "Only extensions matching the scope_filter are fetched from authenticated registries"
    trust_level_assigned       "Extensions receive appropriate trust level based on registry trust configuration"
    no_auth_leaks              "Error messages do not leak authentication details"
  }
  contract   """
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
  verify contract "Support Private Registries: private registry support holds — credentials_configured, registry_client_available, authenticated_before_fetch, scope_filter_respected, trust_level_assigned, no_auth_leaks"
  // Observability: error diagnostics for private registry operations delegate
  // to authenticate_registry_request and retry_registry_request for auth and
  // retry details. This behavior owns the scope_filter and trust_level logic.
}
