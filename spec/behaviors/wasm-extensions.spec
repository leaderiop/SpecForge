// Extension entity kinds, enhancements, conflicts, queries, contributions,
// collectors, discovery, lock files, and doctor

use "invariants/wasm"
use "invariants/validation"
use "invariants/extensions"
use "types/wasm"
use "types/zero-entity-core"
use "types/config"
use "types/errors"
use "types/graph"
use "ports/inbound"
use "ports/outbound"
use "events/wasm-extensions"
// -- Query Extensions -----

behavior provide_extension_query_extensions "Provide Extension Query Extensions" {
  invariants [host_function_type_safety]
  category   query
  types      [ManifestV2, QueryExtension, QueryFileKind, ExtensionError]
  ports      [WasmRuntime]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming manifests with queryExtensions are available"
  }

  ensures {
    query_extensions_loaded_emitted "query_extensions_loaded event is emitted after valid patterns are stored"
    invalid_patterns_warned "invalid query patterns produce warning diagnostic without blocking extension loading"
    patterns_stored "valid patterns are stored alongside extension registration data"
  }

  contract """
    When an extension manifest declares queryExtensions, the compiler
    MUST extract the .scm query patterns and make them available to
    the LSP and editor tooling. Query patterns MUST be validated for
    syntax correctness at extension load time by parsing them with
    tree_sitter::Query::new(). Invalid patterns MUST produce a
    warning diagnostic without blocking extension loading. Valid patterns
    MUST be stored alongside the extension's registration data for
    retrieval during query composition.
  """

  produces [query_extensions_loaded]

  verify unit "valid query extension stored in extension registration"
  verify unit "invalid query pattern produces warning diagnostic"
  verify unit "invalid pattern does not block extension loading"
  verify unit "query extensions extracted from manifest"
  verify contract "requires/ensures consistency for extension query extension loading"

}

behavior compose_query_files_from_extensions "Compose Query Files From Extensions" {
  invariants [extension_load_order_determinism]
  category   query
  types      [QueryExtension, QueryFileKind]
  consumes   [query_extensions_loaded]

  requires {
    query_extensions_loaded_fired "query_extensions_loaded event has fired, confirming all extension query patterns are available"
  }

  ensures {
    query_files_composed_emitted "query_files_composed event is emitted with the final composed query"
    composition_deterministic "same set of extensions always produces the same final query"
    base_queries_first "base queries appear first in composed output, extensions appended"
  }

  contract """
    The LSP MUST compose final query files by concatenating base
    queries with extension query extensions in extension load order. The
    composition MUST follow the string concatenation pattern: base
    queries first, extensions appended. Extension patterns with #match?
    predicates for entity keywords MUST work correctly in the composed
    query. The composed query MUST be re-validated after concatenation
    to catch cross-pattern conflicts. Composition MUST be deterministic
    — the same set of extensions always produces the same final query.
  """

  produces [query_files_composed]

  verify unit "base queries come first in composed output"
  verify unit "extension query extensions appended in load order"
  verify unit "#match? predicates work in composed query"
  verify unit "composition is deterministic across runs"
  verify contract "requires/ensures consistency for query file composition"

}

// -- Entity Kind Conflict Prevention -----

behavior reject_reserved_entity_kind "Reject Reserved Entity Kind" {
  invariants [entity_kind_uniqueness]
  category   command
  types      [KindRegistryEntry, ManifestV2]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming entity kind registrations are pending"
  }

  ensures {
    reserved_entity_kind_rejected_emitted "reserved_entity_kind_rejected event is emitted when a structural keyword collision is detected"
    rejection_before_registration "rejection returns error to calling extension before the kind is registered"
    invalid_identifiers_rejected "invalid identifier characters are rejected"
  }

  contract """
    The KindRegistry MUST reject entity kind names that match structural
    DSL keywords parsed by dedicated grammar rules: spec, use, define, ref,
    verify, true, false. These are the ONLY core-reserved words —
    they have dedicated grammar rules or are literal tokens and cannot be
    used as entity kind names. Domain keywords like behavior, feature,
    invariant, etc. are NOT reserved because they come from extensions. An
    extension registering "behavior" is valid (e.g., a software-domain
    extension provides the "behavior" keyword). Extension-specific keywords
    (e.g., gherkin, scenario, given, when, then) are NOT core-reserved —
    they are reserved by their owning extension via the extension's own
    reserved_keywords manifest field. Rejection MUST
    return an error to the calling extension before the kind is registered.
    Invalid identifier characters MUST also be rejected.
  """

  produces [reserved_entity_kind_rejected]

  verify unit "rejects structural keyword 'spec'"
  verify unit "rejects DSL syntax word 'define'"
  verify unit "rejects literal token 'true'"
  verify unit "accepts domain keyword 'behavior' from extension"
  verify unit "accepts valid custom kind name"
  verify unit "rejects invalid identifier characters"
  verify unit "rejects keyword reserved by another extension via reserved_keywords manifest field"
  verify unit "extension reserving 'scenario' prevents other extensions from using it as a kind"
  verify contract "requires/ensures consistency for reserved entity kind rejection"

}

// User-facing conflict resolution layer. Distinct from detect_duplicate_entity_kinds
// (behaviors/zero-entity-validation.spec) which handles registry-level detection
// during manifest loading. This behavior handles the policy-based resolution UI.
behavior detect_entity_kind_collision "Detect Entity Kind Collision" {
  invariants [entity_kind_uniqueness]
  category   validation
  types      [ManifestV2, ExtensionError, EntityKindConflict]
  consumes   [extension_manifests_loaded]

  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all entity kind declarations are available for collision checking"
  }

  ensures {
    entity_kind_conflict_detected_emitted "entity_kind_conflict_detected event is emitted when a collision is found"
    all_collision_types_checked "structural keyword collisions (E023), define block collisions (E022), and inter-extension collisions (E026) are all checked"
  }

  contract """
    The host MUST detect when two extensions attempt to register the same
    entity kind name. This behavior acts as the orchestrator for all kind
    collision checks: it delegates to reject_reserved_entity_kind for
    structural keyword collisions (E023), checks define block collisions
    directly (E022), and delegates to detect_duplicate_entity_kinds
    (behaviors/zero-entity-validation.spec) for inter-extension kind
    collisions (E026). The compiler never arbitrates domain-level
    conflicts.
  """

  produces [entity_kind_conflict_detected]

  verify unit "two extensions registering same kind produces conflict"
  verify unit "collision with structural keyword produces E023"
  verify unit "collision with define block produces E022"
  verify unit "no false positive for different kind names"
  verify contract "requires/ensures consistency for entity kind collision detection"

}


// -- Entity Enhancement -----

behavior load_extension_manifest "Load Extension Manifest" {
  invariants [extension_load_order_determinism]
  category   command
  types      [ManifestV2, ExtensionError]
  ports      [FileSystem]
  produces   [manifest_loaded]

  requires {
    extension_discovered "installed extension has been discovered with a known path to its sidecar manifest.json"
    filesystem_available "FileSystem port is available for reading manifest files"
  }

  ensures {
    manifest_loaded_emitted "manifest_loaded event is emitted after successful parse of sidecar manifest.json"
    malformed_manifest_diagnosed "malformed manifests produce ExtensionError diagnostic"
    initialization_sequence_followed "per-extension initialization follows the documented 7-step sequence"
  }

  contract """
    When the compiler discovers an installed extension, it MUST locate
    and parse the extension's sidecar manifest.json file alongside the
    .wasm binary. Malformed manifests MUST produce a ExtensionError
    diagnostic. There are no hardcoded manifest factory methods —
    all extensions, including all installed extensions, are loaded from
    their sidecar manifests.
    Bundled extensions are loaded from the compiler's bundled resources
    directory.

    INITIALIZATION SEQUENCE: The guaranteed per-extension initialization
    order is:
      1. load_extension_manifest — parse sidecar manifest.json
      2. validate_extension_manifest — validate schema and required fields
      3. register_entity_kinds_from_manifest — populate KindRegistry
      4. register_edge_types_from_manifest — populate edge type registry
      5. register_validation_rules_from_manifest — populate validation rules
      6. register_entity_enhancements — populate FieldRegistry enhancements
      6.5. register_surface_contributions — populate SurfaceRegistry (CLI commands, MCP tools, MCP resources)
      7. initialize_wasm_extension — call initialize() export
    Steps 1-6 are declarative (manifest-driven). Step 7 is the first
    point at which extension code executes. This sequence is repeated
    per extension in topological order (see topological_sort_extensions).
  """

  verify unit "sidecar JSON parsed into ManifestV2"
  verify unit "malformed sidecar produces ExtensionError"
  verify unit "bundled extensions loaded from bundled resources directory"
  verify unit "initialization follows documented 7-step sequence"
  verify contract "requires/ensures consistency for extension manifest loading"

}

behavior register_entity_enhancements "Register Entity Enhancements" {
  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
  category   command
  types      [ManifestV2, FieldEnhancement, DynamicEdgeType]

  requires {
    manifests_validated "all extension manifests have been validated and entity kinds registered"
  }

  ensures {
    enhancement_registered_emitted "enhancement_registered event is emitted after fields are registered in FieldRegistry"
    registration_before_resolve "registration completes before the resolve phase begins"
    registration_order_deterministic "registration order follows extensions array order in specforge.json"
  }

  contract """
    When an extension manifest declares entity enhancements, the compiler
    MUST parse the enhancement declarations, validate that the target
    entity kinds exist, register the field-to-edge mappings in the
    FieldRegistry, and register any dynamic edge types. Registration
    MUST happen before the resolve phase begins. The order of
    registration MUST follow the extensions array order in specforge.json.
  """

  produces [enhancement_registered]

  verify unit "enhancement fields registered in FieldRegistry"
  verify unit "unknown target entity kind produces error"
  verify unit "enhanced reference fields create graph edges"
  verify unit "enhanced data fields participate in type validation"
  verify unit "registration order follows extensions array"
  verify contract "requires/ensures consistency for entity enhancement registration"

}

behavior detect_enhancement_conflicts "Detect Enhancement Conflicts" {
  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
  category   validation
  types      [EnhancementConflict, FieldEnhancement, EnhancedFieldType, EnumFieldType, ReferenceFieldType]

  requires {
    enhancements_being_registered "enhancement registration is in progress with field-to-entity mappings being processed"
  }

  ensures {
    enhancement_conflict_detected_emitted "enhancement_conflict_detected event is emitted when two extensions register the same field for the same entity kind"
    grammar_conflicts_hard_error "conflicts with grammar-level constructs always produce hard error E018"
    conflict_record_complete "conflict record includes both extension identities and conflicting field types"
  }

  contract """
    During enhancement registration, the compiler MUST detect when two
    extensions register the same field name for the same entity kind. Each
    conflict MUST be recorded with both extension identities and the
    conflicting field types. Conflicts with grammar-level constructs
    (entity title, verify) MUST always produce a hard
    error (E018). Conflicts between extensions MUST be resolved according
    to the configured enhancement_policy.
  """

  produces [enhancement_conflict_detected]

  verify unit "same (entity, field) from two extensions produces conflict"
  verify unit "conflict with grammar-level construct produces E018"
  verify unit "conflict record includes both extension identities"
  verify unit "no false positives for same field on different entities"
  verify contract "requires/ensures consistency for enhancement conflict detection"

}

behavior resolve_enhancement_conflicts "Resolve Enhancement Conflicts" {
  invariants [enhancement_field_uniqueness]
  category   query
  types      [EnhancementConflict, ConflictResolution, EnhancementPolicy]
  consumes   [enhancement_conflict_detected]

  requires {
    enhancement_conflict_detected_fired "enhancement_conflict_detected event has fired, confirming conflicts exist to resolve"
  }

  ensures {
    enhancement_conflict_resolved_emitted "enhancement_conflict_resolved event is emitted after policy is applied"
    error_policy_enforced "with error policy, unresolved conflicts produce E017 diagnostics"
    overrides_precedence "explicit enhancement_overrides in specforge.json take precedence over policy"
  }

  contract """
    When enhancement conflicts are detected, the compiler MUST apply
    the configured enhancement policy. With policy "error" (default and
    only v1 policy), unresolved conflicts MUST produce E017 diagnostics.
    Explicit enhancement_overrides in specforge.json MUST take precedence
    over the policy. Additional policies (priority, namespace) are
    deferred to a future phase.
  """

  produces [enhancement_conflict_resolved]

  verify unit "error policy produces E017 for unresolved conflicts"
  verify unit "explicit override takes precedence over policy"
  verify contract "requires/ensures consistency for enhancement conflict resolution"

}

// -- Contribution Model -----

behavior dispatch_contribution_exports "Dispatch Contribution Exports" {
  invariants [extension_load_order_determinism, renderer_output_restriction]
  category   query
  types      [ManifestV2, ExtensionContributions, ExtensionError]
  ports      [WasmRuntime]
  consumes   [contribution_exports_validated, contribution_toggled, collector_report_ingested]

  requires {
    contribution_exports_validated_fired "contribution_exports_validated event has fired, confirming all declared exports exist"
    wasm_runtime_available "WasmRuntime port is available for calling contribution exports"
  }

  ensures {
    contribution_exports_dispatched_emitted "contribution_exports_dispatched event is emitted after all contributions are called"
    missing_export_diagnosed "missing exports for declared contributions produce E020"
    renderers_refreshed_on_ingestion "renderer contributions are re-dispatched after collector_report_ingested, but not entity/validator/provider/parser"
  }

  contract """
    When an extension declares contributions in its manifest, the compiler
    MUST route calls to the extension's namespaced Wasm exports based on
    the contribution type. Entity contributions MUST call initialize()
    and validate(). Validator contributions MUST call validate().
    Renderer contributions (for non-code outputs such as reports,
    dashboards, traceability matrices) MUST call render().
    Provider contributions MUST call validate_ref(). Parser
    contributions MUST call parse() — parsers run AFTER .spec parsing
    and reference resolution but BEFORE validation, per ADR
    extension_file_parsers. Missing exports for declared contributions
    MUST produce E020.

    This behavior handles compile-time contributions only (entities,
    validators, renderers, providers, parsers, collectors). Surface
    contributions (CLI commands, MCP tools, MCP resources) are dispatched
    by the surface-contributions behaviors (dispatch_surface_command,
    dispatch_surface_mcp_tool, dispatch_surface_mcp_resource).

    Dispatch MUST NOT begin until validate_contribution_exports has
    completed for the extension — this ensures all declared exports
    exist before any are called.

    When triggered by collector_report_ingested, dispatch MUST re-invoke
    renderer contributions only — refreshing outputs (reports, dashboards,
    traceability matrices) with updated coverage metadata. Entity, validator,
    provider, and parser contributions are NOT re-dispatched on collector
    ingestion.
  """

  produces [contribution_exports_dispatched]

  verify unit "entity contributions dispatched to initialize() and validate()"
  verify unit "validator contributions dispatched to validate()"
  verify unit "renderer contributions dispatched to render()"
  verify unit "provider contributions dispatched to validate_ref()"
  verify unit "missing export for declared contribution produces E020"
  verify unit "dispatch waits for validate_contribution_exports to complete"
  verify unit "parser contribution exports dispatched before validation phase"
  verify unit "parser contribution receives read_file, emit_diagnostic, add_graph_node, add_graph_edge only"
  verify unit "renderer contributions re-dispatched after collector_report_ingested"
  verify contract "requires/ensures consistency for contribution export dispatch"

}

behavior enforce_per_call_site_permissions "Enforce Per-Call-Site Permissions" {
  invariants [wasm_sandbox_integrity]
  category   command
  types      [ManifestV2, SandboxPolicy]
  ports      [WasmRuntime]

  requires {
    sandbox_policy_ready "sandbox policy has been computed for the extension"
    contribution_type_known "the contribution type of the current export call site is known"
  }

  ensures {
    contribution_permission_denied_emitted "contribution_permission_denied event is emitted when unauthorized host function is called"
    per_call_site_enforced "permissions are enforced per export call site, not per extension"
    unauthorized_calls_rejected "calls to unauthorized host functions are rejected"
  }

  contract """
    Host function permissions MUST be enforced per export call site, not
    per extension. An extension's validator export MUST only access query_graph
    and emit_diagnostic. An extension's renderer export MUST additionally
    access emit_file. An extension's provider export MUST additionally access
    http_get. An extension's entity contribution exports MUST only access
    query_graph, add_graph_node, and add_graph_edge. Collector contributions
    MUST only access query_graph and emit_file. An extension's parser
    contribution exports MUST only access emit_diagnostic, add_graph_node,
    add_graph_edge, and read_file. Parsers do NOT get query_graph because
    they run during graph construction, not after (per ADR
    extension_file_parsers). Calls to unauthorized host functions MUST be
    rejected.

    Surface contributions (cmd__, mcp__ exports) have their own sandbox
    enforcement via enforce_surface_sandbox (behaviors/surface-contributions.spec).
    This behavior covers compile-time contribution call sites only.
  """

  produces [contribution_permission_denied]

  verify unit "validator export limited to query_graph and emit_diagnostic"
  verify unit "renderer export additionally allows emit_file"
  verify unit "provider export additionally allows http_get"
  verify unit "entity contribution export limited to query_graph, add_graph_node, and add_graph_edge"
  verify unit "collector contribution export limited to query_graph and emit_file"
  verify unit "parser contribution export limited to emit_diagnostic, add_graph_node, add_graph_edge, and read_file"
  verify unit "unauthorized host function call is rejected"
  verify contract "requires/ensures consistency for per-call-site permission enforcement"

}

behavior validate_contribution_exports "Validate Contribution Exports" {
  invariants [host_function_type_safety]
  category   validation
  types      [ManifestV2, ExtensionError]
  ports      [WasmRuntime]

  requires {
    extension_loaded_ready "extension .wasm binary has been loaded into the runtime"
    manifest_contributions_declared "extension manifest declares compile-time contributions to validate"
  }

  ensures {
    contribution_exports_validated_emitted "contribution_exports_validated event is emitted when all declared exports are present"
    contribution_export_validation_failed_emitted "contribution_export_validation_failed event is emitted when exports are missing"
    missing_exports_diagnosed "missing exports produce E020 diagnostic listing expected export names"
  }

  contract """
    After loading an extension, the compiler MUST verify that the .wasm binary
    exports all functions required by its declared compile-time contributions.
    Missing exports MUST produce an E020 diagnostic listing the expected export
    names. Extra exports beyond declared contributions MUST be ignored.
    Surface contribution exports (cmd__, mcp__) are validated separately by
    validate_surface_exports (behaviors/surface-contributions.spec).
  """

  produces [contribution_exports_validated, contribution_export_validation_failed]

  verify unit "all declared contribution exports present passes"
  verify unit "missing contribution export produces E020"
  verify unit "extra exports beyond contributions are ignored"
  verify contract "requires/ensures consistency for contribution export validation"

}

behavior toggle_extension_contributions "Toggle Extension Contributions" {
  invariants [extension_load_order_determinism]
  category   command
  types      [ManifestV2, ExtensionContributions]
  ports      [CompilerApi]

  requires {
    extension_loaded_ready "extension is loaded and initialized before contributions can be toggled"
    config_available "specforge.json configuration is available for reading contribution toggle state"
  }

  ensures {
    contribution_toggled_emitted "contribution_toggled event is emitted after toggle state is applied"
    disabled_contributions_skipped "disabled contributions are skipped during dispatch"
    sole_provider_warned "disabling the only entity provider for a kind produces W028 warning"
  }

  contract """
    The specforge.json configuration MUST support enabling or disabling
    individual contributions from an extension. Disabled contributions MUST
    be skipped during dispatch. The extension MUST still be loaded and
    initialized — only the disabled contribution exports are not called.
    Disabling the only entity provider for a kind MUST produce a W028
    warning listing the affected entity kind.
  """

  produces [contribution_toggled]

  verify unit "disabled contribution is skipped during dispatch"
  verify unit "extension still loaded when some contributions disabled"
  verify unit "re-enabled contribution resumes normal dispatch"
  verify unit "disabling only entity provider for a kind produces W028"
  verify contract "requires/ensures consistency for extension contribution toggling"

}

// -- Collector Contribution Behaviors -----

// Cross-ref: collector workflow spans multiple behaviors:
// register_collector_contributions → auto_detect_collector → dispatch_collector →
// validate_collector_output → ingest_collector_report. CLI entry point is
// specforge collect; MCP entry point is provide_mcp_collect_tool
// (behaviors/mcp-operations.spec).
behavior register_collector_contributions "Register Collector Contributions" {
  invariants [extension_load_order_determinism]
  category   query
  types      [ManifestV2, CollectorContribution, ExtensionContributions, ExtensionError, CollectorEntityMapping, EntityMappingStrategy, CollectorTestStatus]
  ports      [WasmRuntime]

  requires {
    manifest_declares_collectors "extension manifest declares collectors=true in contributes section"
    wasm_runtime_available "WasmRuntime port is available for validating collector Wasm exports"
  }

  ensures {
    collector_registered_emitted "collector_registered event is emitted after successful registration"
    duplicate_names_diagnosed "duplicate collector names across extensions produce E029"
    missing_exports_diagnosed "missing Wasm exports produce E020"
  }

  contract """
    When an extension manifest declares collectors=true in its contributes
    section and includes collector contribution entries, the compiler MUST
    parse each CollectorContribution from the manifest, register it in a
    collector registry keyed by name, validate that the declared Wasm export
    exists in the .wasm binary, and detect duplicate collector names across
    extensions (E029). Registration MUST happen during the extension
    initialization phase in topological order.
  """

  produces [collector_registered]

  verify unit "collector contribution parsed from manifest"
  verify unit "collector registered in collector registry"
  verify unit "missing Wasm export produces E020"
  verify unit "duplicate collector name produces E029"
  verify contract "requires/ensures consistency for collector contribution registration"

}

// NOTE: auto_detect_collector does not produce an event because dispatch is
// CLI-initiated (specforge collect), not event-driven. The CLI command
// directly selects and dispatches the collector — there is no intermediate
// event between detection and dispatch.
behavior auto_detect_collector "Auto-Detect Collector" {
  invariants [extension_load_order_determinism]
  category   validation
  types      [CollectorContribution, CollectorAutoDetect]
  ports      [FileSystem]
  consumes   [collector_registered]

  requires {
    collector_registered_fired "collector_registered event has fired, confirming collectors are available for auto-detection"
    filesystem_available "FileSystem port is available for matching file patterns against project directory"
  }

  ensures {
    first_match_selected "the first matching collector is selected when multiple match"
    no_match_diagnosed "if no collector matches, I013 info diagnostic is emitted listing available collectors"
  }

  contract """
    When specforge collect is invoked without an explicit collector name,
    the system MUST iterate over all registered collectors and match their
    auto_detect criteria against the project. File patterns MUST be matched
    against the project directory. Environment variables MUST be checked
    for presence. The first matching collector MUST be selected. If no
    collector matches, the system MUST emit an I013 info diagnostic
    listing available collectors.

    This behavior does not produce an event because dispatch is CLI-initiated
    (specforge collect), not event-driven — the CLI command directly selects
    and dispatches the collector.
  """

  produces []

  verify unit "file pattern match selects collector"
  verify unit "env var match selects collector"
  verify unit "first match wins when multiple match"
  verify unit "no match emits I013 with available collectors"
  verify contract "requires/ensures consistency for collector auto-detection"

}

behavior dispatch_collector "Dispatch Collector" {
  invariants [wasm_sandbox_integrity, extension_isolation]
  category   query
  types      [CollectorContribution, CollectorDispatchInput, CollectorReport, ExtensionError, WasmTrapInfo]
  ports      [WasmRuntime, FileSystem]

  requires {
    collector_selected "a collector has been selected (explicitly or via auto-detection)"
    wasm_runtime_available "WasmRuntime port is available for calling collector Wasm export"
  }

  ensures {
    collector_dispatched_emitted "collector_dispatched event is emitted after collector execution completes"
    traps_caught "Wasm traps during collector execution are caught and reported as ExtensionError"
    no_external_processes "no external processes or system commands are spawned during dispatch"
  }

  contract """
    When a collector is selected, the system MUST call the collector's
    declared Wasm export with the test report path and the set of known
    entity IDs as input. The collector Wasm function MUST return a
    CollectorReport. Wasm traps during collector execution MUST be caught
    and reported as ExtensionError diagnostics without affecting the
    compilation pipeline. The collector Wasm export runs within the sandbox.
    The system MUST NOT spawn external processes or invoke system commands
    during collector dispatch. All evidence parsing occurs within the Wasm
    sandbox boundary.
  """

  produces [collector_dispatched]

  verify unit "calls collector Wasm export with report path and entity IDs"
  verify unit "returns CollectorReport on success"
  verify unit "Wasm trap caught and reported as ExtensionError"
  verify unit "collector dispatch spawns no external processes"
  verify contract "requires/ensures consistency for collector dispatch"

}

behavior validate_collector_output "Validate Collector Output" {
  invariants [collector_output_conformance]
  category   validation
  types      [CollectorReport, CollectorStats, ExtensionError]
  consumes   [collector_dispatched]

  requires {
    collector_dispatched_fired "collector_dispatched event has fired, confirming collector has returned a report"
  }

  ensures {
    collector_output_validated_emitted "collector_output_validated event is emitted when report passes schema validation"
    unknown_entities_warned "unknown entity IDs referenced in entries produce W029 warning"
    stats_consistency_checked "inconsistent stats (total != entries.length) produce W030 warning"
  }

  contract """
    After a collector returns its report, the system MUST validate the
    output against the specforge-report/v1 schema. Entity IDs referenced
    in entries MUST be checked against the graph — unknown entity IDs
    MUST produce a W029 warning. Stats consistency MUST be verified:
    stats.total MUST equal entries.length, stats.mapped + stats.unmapped
    MUST equal stats.total. Inconsistent stats MUST produce a W030
    warning.
  """

  produces [collector_output_validated]

  verify unit "valid report passes schema validation"
  verify unit "unknown entity ID produces W029"
  verify unit "inconsistent stats produce W030"
  verify unit "missing schema field produces hard error"
  verify contract "requires/ensures consistency for collector output validation"

}

behavior ingest_collector_report "Ingest Collector Report" {
  invariants [collector_output_conformance]
  category   query
  types      [CollectorReport, CollectorReportEntry, Graph]
  ports      [FileSystem]
  consumes   [collector_output_validated]

  requires {
    collector_output_validated_fired "collector_output_validated event has fired, confirming report passed schema validation"
    graph_available "compiled graph is available for associating entries with entity nodes"
  }

  ensures {
    collector_report_ingested_emitted "collector_report_ingested event is emitted after entries are associated and report is written"
    coverage_metadata_updated "coverage metadata is updated on entity nodes in the graph"
    merged_report_written "merged report is written to specforge-report.json"
    unmapped_entries_preserved "entries with unknown entity IDs are included in unmapped_tests section"
  }

  contract """
    This behavior MUST NOT be invoked until validate_collector_output
    completes successfully. After validation, the system MUST associate
    each CollectorReportEntry with its corresponding entity node in the
    graph, update coverage metadata on the entity node, and write the
    merged report to specforge-report.json. Entries with unknown entity IDs (already
    warned by validate_collector_output) MUST be included in the
    unmapped_tests section of the output.
  """

  produces [collector_report_ingested]

  verify unit "entries associated with entity nodes"
  verify unit "coverage metadata updated on entity"
  verify unit "merged report written to specforge-report.json"
  verify unit "unknown entity entries in unmapped_tests"
  verify contract "requires/ensures consistency for collector report ingestion"

}

// -- Discovery & Configuration -----

behavior discover_extensions "Discover Extensions" {
  invariants [extension_load_order_determinism, registry_integrity, offline_first_extension_resolution]
  category   command
  types      [ExtensionSource, ManifestV2, ExtensionError]
  ports      [WasmRuntime]

  requires {
    registries_configured "at least one registry source (npm, OCI, GitHub Releases) is configured"
  }

  ensures {
    extensions_discovered_emitted "extensions_discovered event is emitted with aggregated discovery results"
    network_failure_graceful "network failures produce warning diagnostic without aborting discovery"
    results_complete "results include extension name, available versions, description, and source registry"
  }

  contract """
    The system MUST query configured registries to discover available
    extensions and check for updates to installed extensions. Discovery
    MUST search all configured registry sources (npm, OCI, GitHub Releases)
    and aggregate results. For each installed extension, the system MUST
    check whether a newer version exists that satisfies the declared semver
    range. Discovery results MUST include extension name, available versions,
    description, and source registry. Network failures MUST produce a
    warning diagnostic without aborting the discovery process. Specifier
    parsing is handled by parse_extension_specifier — this behavior is
    responsible for the registry query and result aggregation.
  """

  produces [extensions_discovered]

  verify unit "queries configured registries for available extensions"
  verify unit "checks for updates to installed extensions"
  verify unit "aggregates results across multiple registries"
  verify unit "network failure produces warning without aborting"
  verify contract "requires/ensures consistency for extension discovery"

}

behavior run_doctor_check "Run Doctor Check" {
  // Doctor REPORTS on invariant violations — it does not ENFORCE them.
  // Enforcement is done by the behaviors listed in each invariant's enforced_by.
  category   validation
  invariants [diagnostic_determinism]
  types      [ManifestV2, EnhancementConflict, FieldEnhancement]
  ports      [FileSystem]
  consumes   [enhancement_registered, wasm_trap_caught]

  requires {
    enhancement_registered_fired "enhancement_registered event has fired, confirming FieldRegistry is built"
    filesystem_available "FileSystem port is available for reading extension manifests"
  }

  ensures {
    doctor_check_completed_emitted "doctor_check_completed event is emitted after all checks finish"
    report_produced "human-readable report listing extensions, enhancements, and conflicts is produced"
    json_output_supported "--json flag produces machine-readable JSON output for CI integration"
  }

  contract """
    When specforge doctor is invoked, the system MUST load all extension
    manifests, build the FieldRegistry, detect all conflicts, and
    produce a human-readable report listing installed extensions, their
    enhancements, any conflicts with actionable resolution suggestions,
    and additional checks (shadowed fields, unknown target entities,
    edge label conflicts). The --json flag MUST produce machine-readable
    JSON output for CI integration.
  """

  produces [doctor_check_completed]

  verify unit "doctor lists all installed extensions with enhancement counts"
  verify unit "doctor lists all enhancements grouped by entity kind"
  verify unit "doctor reports conflicts with resolution suggestions"
  verify unit "doctor detects shadowed grammar-level constructs"
  verify unit "doctor --json produces valid JSON output"
  verify contract "requires/ensures consistency for doctor check"

}

// -- Extension Source Resolution -----

behavior parse_extension_specifier "Parse Extension Specifier" {
  invariants [registry_integrity]
  category   command
  types      [ExtensionSpecifier, ExtensionSource, ExtensionError]

  requires {
    specifier_string_provided "a raw extension specifier string is provided for parsing"
  }

  ensures {
    extension_specifier_parsed_emitted "extension_specifier_parsed event is emitted with structured source descriptor"
    invalid_specifier_diagnosed "invalid specifiers produce ExtensionError diagnostic with expected format"
  }

  contract """
    The system MUST parse extension specifier strings into structured
    source descriptors. Supported formats: "@scope/name@version" for
    registry extensions, "./path" for local extensions, and "git:url#ref"
    for git-sourced extensions. Invalid specifiers MUST produce a
    ExtensionError diagnostic with the expected format.
  """

  produces [extension_specifier_parsed]

  verify unit "@scope/name@version parsed as registry source"
  verify unit "./path parsed as local source"
  verify unit "git:url#ref parsed as git source"
  verify unit "invalid specifier produces ExtensionError"
  verify contract "requires/ensures consistency for extension specifier parsing"

}

behavior resolve_extension_source "Resolve Extension Source" {
  invariants [registry_integrity]
  category   query
  types      [ManifestV2, ExtensionSpecifier, ExtensionSource, ExtensionError]
  ports      [FileSystem, RegistryClient]
  consumes   [extension_specifier_parsed]

  requires {
    extension_specifier_parsed_fired "extension_specifier_parsed event has fired, confirming structured source descriptor is available"
    source_ports_available "FileSystem and RegistryClient ports are available for resolution"
  }

  ensures {
    extension_source_resolved_emitted "extension_source_resolved event is emitted with concrete manifest and .wasm binary"
    resolution_failure_diagnosed "resolution failures produce ExtensionError diagnostic"
  }

  contract """
    Given a parsed extension specifier, the system MUST resolve it to a
    concrete manifest and .wasm binary. Registry sources MUST query the
    registry API. Local sources MUST read from the filesystem. Git sources
    MUST clone or fetch the repository at the specified ref. Resolution
    failures MUST produce a ExtensionError diagnostic.
  """

  produces [extension_source_resolved]

  verify unit "registry source resolves via registry API"
  verify unit "local source resolves from filesystem"
  verify unit "git source resolves from repository"
  verify unit "resolution failure produces ExtensionError"
  verify contract "requires/ensures consistency for extension source resolution"

}

// -- Lock File Management -----

behavior write_lock_file "Write Lock File" {
  invariants [extension_load_order_determinism, registry_integrity]
  category   command
  types      [ManifestV2, LockFile, LockFileEntry]
  ports      [FileSystem]

  requires {
    extensions_resolved "all extensions have been resolved with exact versions and wasm hashes"
    filesystem_available "FileSystem port is available for writing lock file"
  }

  ensures {
    lock_file_written_emitted "lock_file_written event is emitted after successful write"
    output_deterministic "same inputs always produce byte-identical lock file output"
    write_atomic "lock file is written atomically to prevent corruption"
  }

  contract """
    After resolving all extensions, the system MUST write a specforge.lock
    file containing the exact resolved version and SHA256 wasm_hash for
    each installed extension. The lock file format MUST be deterministic —
    same inputs always produce byte-identical output. The resolved_at
    timestamp is metadata excluded from the determinism guarantee. The
    lock file MUST be written atomically to prevent corruption.
  """

  produces [lock_file_written]

  verify unit "lock file contains exact versions and wasm hashes"
  verify unit "lock file output is deterministic"
  verify unit "lock file written atomically"
  verify contract "requires/ensures consistency for lock file writing"

}

behavior read_lock_file "Read Lock File" {
  invariants [extension_load_order_determinism]
  category   command
  types      [ManifestV2, LockFile, LockFileEntry, ExtensionError]
  ports      [FileSystem]
  consumes   [all_files_parsed]

  requires {
    all_files_parsed_fired "all_files_parsed event has fired, confirming spec files are parsed"
    filesystem_available "FileSystem port is available for reading lock file"
  }

  ensures {
    lock_file_read_emitted "lock_file_read event is emitted after lock file is processed"
    locked_versions_used "locked versions are used instead of resolving from sources when lock file exists"
    malformed_lock_graceful "malformed lock files produce warning and fall back to fresh resolution"
  }

  contract """
    When a specforge.lock file exists, the system MUST use locked versions
    instead of resolving from sources. Missing lock entries for declared
    extensions MUST trigger resolution and lock file update. Malformed lock
    files MUST produce a warning and fall back to fresh resolution.
  """

  produces [lock_file_read]

  verify unit "locked versions used when lock file exists"
  verify unit "missing lock entry triggers resolution"
  verify unit "malformed lock file produces warning and falls back"
  verify contract "requires/ensures consistency for lock file reading"

}

// ── Extension Update ──────────────────────────────────────────

behavior update_all_extensions "Update All Extensions" {
  invariants [wasm_sandbox_integrity, peer_dependency_satisfaction, aot_cache_integrity, extension_operation_atomicity]
  category   command
  types      [ManifestV2, LockFileEntry, ExtensionError]
  ports      [WasmRuntime]

  requires {
    extensions_installed "at least one extension is installed with a valid manifest"
    registries_reachable "configured registries are reachable for version checks"
  }

  ensures {
    batch_update_completed_emitted "batch_update_completed event is emitted after all upgrades are applied"
    semver_constraints_respected "upgrades respect semver constraints, major bumps skipped without --major"
    aot_caches_recompiled "AOT caches are recompiled for all updated extensions"
    atomic_rollback_on_failure "if any upgrade fails, all changes are rolled back"
  }

  contract """
    When specforge update is invoked, the system MUST check all installed
    extensions for newer versions by querying their configured registries.
    The system MUST upgrade each extension to the latest compatible version
    respecting semver constraints. Major version bumps MUST be skipped unless
    --major is specified. After upgrading, the system MUST recompile AOT
    caches for updated extensions and refresh the lock file. Peer dependency
    conflicts introduced by upgrades MUST be detected and reported before
    applying changes. If any upgrade fails, the system MUST roll back all
    changes and report the failure.
  """

  produces [batch_update_completed]

  verify unit "newer versions detected from registry"
  verify unit "semver-compatible upgrades applied"
  verify unit "major version skipped without --major flag"
  verify unit "AOT cache recompiled for updated extensions"
  verify unit "lock file refreshed after update"
  verify unit "peer dependency conflicts detected before applying"
  verify unit "failed upgrade rolls back all changes"
  verify contract "requires/ensures consistency for batch extension update"

}

behavior refresh_lock_file "Refresh Lock File" {
  invariants [aot_cache_integrity, registry_integrity]
  category   command
  types      [LockFileEntry, ManifestV2]
  ports      [WasmRuntime, FileSystem]

  requires {
    lock_file_exists "specforge.lock file exists with entries to refresh"
    registries_reachable "configured registries are reachable for re-resolution"
  }

  ensures {
    lock_file_refreshed_emitted "lock_file_refreshed event is emitted after lock file is regenerated"
    versions_unchanged "pinned versions are not changed during refresh"
    hashes_verified "SHA256 hashes of all installed .wasm binaries are verified against lock entries"
  }

  contract """
    When specforge update --lock is invoked, the system MUST re-resolve all
    extension specifiers from their configured registries without changing
    pinned versions. The system MUST verify SHA256 hashes of all installed
    .wasm binaries against the lock file entries. Mismatched hashes MUST
    produce a warning diagnostic. The lock file MUST be regenerated with
    current resolution metadata including timestamps and registry URLs.
  """

  produces [lock_file_refreshed]

  verify unit "specifiers re-resolved without version changes"
  verify unit "SHA256 hashes verified against lock entries"
  verify unit "mismatched hash produces warning"
  verify unit "lock file regenerated with current metadata"
  verify contract "requires/ensures consistency for lock file refresh"

}
