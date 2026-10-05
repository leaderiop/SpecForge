// Extension entity kinds, enhancements, conflicts, queries, contributions,
// collectors, discovery, lock files, and doctor

use "events/wasm-extensions"
use "invariants/extensions"
use "invariants/validation"
use "invariants/wasm"
use "ports/inbound"
use "ports/outbound"
use "types/config"
use "types/errors"
use "types/graph"
use "types/wasm"
use "types/zero-entity-core"

// -- Query Extensions -----

behavior provide_extension_query_extensions "Provide Extension Query Extensions" {
  features   [extension_query_contributions]
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
    invalid_patterns_warned         "invalid query patterns produce warning diagnostic without blocking extension loading"
    patterns_stored                 "valid patterns are stored alongside extension registration data"
  }
  contract   """
    When an extension manifest declares queryExtensions, the compiler
    MUST extract the .scm query patterns and make them available to
    the LSP and editor tooling. Query patterns MUST be validated for
    syntax correctness at extension load time by parsing them with
    tree_sitter::Query::new(). Invalid patterns MUST produce a
    warning diagnostic without blocking extension loading. Valid patterns
    MUST be stored alongside the extension's registration data for
    retrieval during query composition.
  """
  produces   [query_extensions_loaded]
  verify unit "valid query extension stored in extension registration"
  verify unit "invalid query pattern produces warning diagnostic"
  verify unit "invalid pattern does not block extension loading"
  verify unit "query extensions extracted from manifest"
  verify contract "Provide Extension Query Extensions: extension query extension loading holds — extension_manifests_loaded_fired, query_extensions_loaded_emitted, invalid_patterns_warned, patterns_stored"
}

behavior compose_query_files_from_extensions "Compose Query Files From Extensions" {
  features   [extension_query_contributions]
  invariants [extension_load_order_determinism]
  category   query
  types      [QueryExtension, QueryFileKind]
  consumes   [query_extensions_loaded]
  requires {
    query_extensions_loaded_fired "query_extensions_loaded event has fired, confirming all extension query patterns are available"
  }
  ensures {
    query_files_composed_emitted "query_files_composed event is emitted with the final composed query"
    composition_deterministic    "same set of extensions always produces the same final query"
    base_queries_first           "base queries appear first in composed output, extensions appended"
  }
  contract   """
    The LSP MUST compose final query files by concatenating base
    queries with extension query extensions in extension load order. The
    composition MUST follow the string concatenation pattern: base
    queries first, extensions appended. Extension patterns with #match?
    predicates for entity keywords MUST work correctly in the composed
    query. The composed query MUST be re-validated after concatenation
    to catch cross-pattern conflicts. Composition MUST be deterministic
    — the same set of extensions always produces the same final query.
  """
  produces   [query_files_composed]
  verify unit "base queries come first in composed output"
  verify unit "extension query extensions appended in load order"
  verify unit "#match? predicates work in composed query"
  verify unit "composition is deterministic across runs"
  verify contract "Compose Query Files From Extensions: query file composition holds — query_extensions_loaded_fired, query_files_composed_emitted, composition_deterministic, base_queries_first"
}

// -- Entity Kind Conflict Prevention -----

behavior reject_reserved_entity_kind "Reject Reserved Entity Kind" {
  features   [entity_kind_conflict_prevention]
  invariants [entity_kind_uniqueness]
  category   command
  types      [KindRegistryEntry, ManifestV2]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming entity kind registrations are pending"
  }
  ensures {
    reserved_entity_kind_rejected_emitted "reserved_entity_kind_rejected event is emitted when a structural keyword collision is detected"
    rejection_before_registration         "rejection returns error to calling extension before the kind is registered"
    invalid_identifiers_rejected          "invalid identifier characters are rejected"
  }
  contract   """
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
  produces   [reserved_entity_kind_rejected]
  verify unit "rejects structural keyword 'spec'"
  verify unit "rejects DSL syntax word 'define'"
  verify unit "rejects literal token 'true'"
  verify unit "accepts domain keyword 'behavior' from extension"
  verify unit "accepts valid custom kind name"
  verify unit "rejects invalid identifier characters"
  verify unit "rejects keyword reserved by another extension via reserved_keywords manifest field"
  verify unit "extension reserving 'scenario' prevents other extensions from using it as a kind"
  verify contract "Reject Reserved Entity Kind: reserved entity kind rejection holds — extension_manifests_loaded_fired, reserved_entity_kind_rejected_emitted, rejection_before_registration, invalid_identifiers_rejected"
}

// User-facing conflict resolution layer. Distinct from detect_duplicate_entity_kinds
// (behaviors/zero-entity-validation.spec) which handles registry-level detection
// during manifest loading. This behavior handles the policy-based resolution UI.
behavior detect_entity_kind_collision "Detect Entity Kind Collision" {
  features   [entity_kind_conflict_prevention]
  invariants [entity_kind_uniqueness]
  category   validation
  types      [ManifestV2, ExtensionError, EntityKindConflict]
  consumes   [extension_manifests_loaded]
  requires {
    extension_manifests_loaded_fired "extension_manifests_loaded event has fired, confirming all entity kind declarations are available for collision checking"
  }
  ensures {
    entity_kind_conflict_detected_emitted "entity_kind_conflict_detected event is emitted when a collision is found"
    all_collision_types_checked           "structural keyword collisions (E023) and inter-extension collisions (E026) are all checked"
  }
  contract   """
    The host MUST detect when two extensions attempt to register the same
    entity kind name. This behavior acts as the orchestrator for all kind
    collision checks: it delegates to reject_reserved_entity_kind for
    structural keyword collisions (E023), and delegates to
    detect_duplicate_entity_kinds
    (behaviors/zero-entity-validation.spec) for inter-extension kind
    collisions (E026). The compiler never arbitrates domain-level
    conflicts.
  """
  produces   [entity_kind_conflict_detected]
  verify unit "two extensions registering same kind produces conflict"
  verify unit "collision with structural keyword produces E023"
  verify unit "no false positive for different kind names"
  verify contract "Detect Entity Kind Collision: entity kind collision detection holds — extension_manifests_loaded_fired, entity_kind_conflict_detected_emitted, all_collision_types_checked"
}

// -- Entity Enhancement -----

behavior load_extension_manifest "Load Extension Manifest" {
  features   [extension_manifest]
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
    manifest_loaded_emitted          "manifest_loaded event is emitted after successful parse of sidecar manifest.json"
    malformed_manifest_diagnosed     "malformed manifests produce ExtensionError diagnostic"
    initialization_sequence_followed "per-extension initialization follows the documented 7-step sequence"
  }
  contract   """
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
  verify contract "Load Extension Manifest: extension manifest loading holds — extension_discovered, filesystem_available, manifest_loaded_emitted, malformed_manifest_diagnosed, initialization_sequence_followed"
}

behavior register_entity_enhancements "Register Entity Enhancements" {
  features   [entity_enhancement]
  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
  category   command
  types      [ManifestV2, FieldEnhancement, DynamicEdgeType]
  requires {
    manifests_validated "all extension manifests have been validated and entity kinds registered"
  }
  ensures {
    enhancement_registered_emitted   "enhancement_registered event is emitted after fields are registered in FieldRegistry"
    registration_before_resolve      "registration completes before the resolve phase begins"
    registration_order_deterministic "registration order follows extensions array order in specforge.json"
  }
  contract   """
    When an extension manifest declares entity enhancements, the compiler
    MUST parse the enhancement declarations, validate that the target
    entity kinds exist (an unknown target kind is an I004 info diagnostic), register the field-to-edge mappings in the
    FieldRegistry, and register any dynamic edge types. Registration
    MUST happen before the resolve phase begins. The order of
    registration MUST follow the extensions array order in specforge.json.
    An enhancement that names another extension as the target kind's owner
    is conditional: when that owner is not loaded the enhancement MUST be
    skipped without a diagnostic, because the project does not use it.
    An enhancement field MUST NOT overwrite a field the kind already
    declares.
  """
  produces   [enhancement_registered]
  verify unit "enhancement fields registered in FieldRegistry"
  verify unit "unknown target kind produces I004 info diagnostic"
  verify unit "enhancement of a kind owned by an extension that is not loaded is skipped silently"
  verify unit "an enhancement with verify kinds makes its target kind testable"
  verify unit "enhanced reference fields create graph edges"
  verify unit "enhanced data fields participate in type validation"
  verify unit "registration order follows extensions array"
  verify unit "enhancement field does NOT overwrite existing kind-level field"
  verify contract "Register Entity Enhancements: entity enhancement registration holds — manifests_validated, enhancement_registered_emitted, registration_before_resolve, registration_order_deterministic"
}

behavior detect_enhancement_conflicts "Detect Enhancement Conflicts" {
  features   [entity_enhancement]
  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
  category   validation
  types      [
    EnhancementConflict,
    FieldEnhancement,
    EnhancedFieldType,
    EnumFieldType,
    ReferenceFieldType,
  ]
  requires {
    enhancements_being_registered "enhancement registration is in progress with field-to-entity mappings being processed"
  }
  ensures {
    enhancement_conflict_detected_emitted "enhancement_conflict_detected event is emitted when two extensions register the same field for the same entity kind"
    grammar_conflicts_hard_error          "conflicts with grammar-level constructs always produce hard error E018"
    conflict_record_complete              "conflict record includes both extension identities and conflicting field types"
  }
  contract   """
    During enhancement registration, the compiler MUST detect when two
    extensions register the same field name for the same entity kind. Each
    conflict MUST be recorded with both extension identities and the
    conflicting field types. Conflicts with grammar-level constructs
    (entity title, verify) MUST always produce a hard
    error (E018). Conflicts between extensions MUST be resolved according
    to the configured enhancement_policy.
  """
  produces   [enhancement_conflict_detected]
  verify unit "same (entity, field) from two extensions produces conflict"
  verify unit "conflict with grammar-level construct produces E018"
  verify unit "conflict record includes both extension identities"
  verify unit "no false positives for same field on different entities"
  verify contract "Detect Enhancement Conflicts: enhancement conflict detection holds — enhancements_being_registered, enhancement_conflict_detected_emitted, grammar_conflicts_hard_error, conflict_record_complete"
}

behavior resolve_enhancement_conflicts "Resolve Enhancement Conflicts" {
  features   [entity_enhancement]
  invariants [enhancement_field_uniqueness]
  category   query
  types      [EnhancementConflict, ConflictResolution, EnhancementPolicy]
  consumes   [enhancement_conflict_detected]
  requires {
    enhancement_conflict_detected_fired "enhancement_conflict_detected event has fired, confirming conflicts exist to resolve"
  }
  ensures {
    enhancement_conflict_resolved_emitted "enhancement_conflict_resolved event is emitted after policy is applied"
    error_policy_enforced                 "with error policy, unresolved conflicts produce E017 diagnostics"
    overrides_precedence                  "explicit enhancement_overrides in specforge.json take precedence over policy"
  }
  contract   """
    When enhancement conflicts are detected, the compiler MUST apply
    the configured enhancement policy. With policy "error" (default and
    only v1 policy), unresolved conflicts MUST produce E017 diagnostics.
    Explicit enhancement_overrides in specforge.json MUST take precedence
    over the policy. Additional policies (priority, namespace) are
    deferred to a future phase.
  """
  produces   [enhancement_conflict_resolved]
  verify unit "error policy produces E017 for unresolved conflicts"
  verify unit "explicit override takes precedence over policy"
  verify contract "Resolve Enhancement Conflicts: enhancement conflict resolution holds — enhancement_conflict_detected_fired, enhancement_conflict_resolved_emitted, error_policy_enforced, overrides_precedence"
}

// -- Contribution Model -----

behavior dispatch_contribution_exports "Dispatch Contribution Exports" {
  features   [contribution_based_extensions]
  invariants [extension_load_order_determinism, renderer_output_restriction]
  category   query
  types      [ManifestV2, ExtensionContributions, ExtensionError]
  ports      [WasmRuntime]
  consumes   [contribution_exports_validated, contribution_toggled, collector_report_ingested]
  requires {
    contribution_exports_validated_fired "contribution_exports_validated event has fired, confirming all declared exports exist"
    wasm_runtime_available               "WasmRuntime port is available for calling contribution exports"
  }
  ensures {
    contribution_exports_dispatched_emitted "contribution_exports_dispatched event is emitted after all contributions are called"
    missing_export_diagnosed                "missing exports for declared contributions produce E020"
    renderers_refreshed_on_ingestion        "renderer contributions are re-dispatched after collector_report_ingested, but not entity/validator/provider/parser"
  }
  contract   """
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
  produces   [contribution_exports_dispatched]
  verify unit "entity contributions dispatched to initialize() and validate()"
  verify unit "validator contributions dispatched to validate()"
  verify unit "renderer contributions dispatched to render()"
  verify unit "provider contributions dispatched to validate_ref()"
  verify unit "missing export for declared contribution produces E020"
  verify unit "dispatch waits for validate_contribution_exports to complete"
  verify unit "parser contribution exports dispatched before validation phase"
  verify unit "parser contribution receives read_file, emit_diagnostic, add_graph_node, add_graph_edge only"
  verify unit "renderer contributions re-dispatched after collector_report_ingested"
  verify contract "Dispatch Contribution Exports: contribution export dispatch holds — contribution_exports_validated_fired, wasm_runtime_available, contribution_exports_dispatched_emitted, missing_export_diagnosed, renderers_refreshed_on_ingestion"
}

// -- Check-Phase Passes and the Build Cache -----

behavior run_check_phase_passes "Run Check-Phase Passes" {
  features   [contribution_based_extensions]
  invariants [extension_load_order_determinism]
  category   validation
  types      [ManifestV2, Diagnostic, WasmTrapInfo]
  ports      [WasmRuntime]
  requires {
    graph_checked "the graph is built and its checks (core validation, the registry checks and the extensions' validation rules) have run"
  }
  ensures {
    runs_every_compile     "a pass declared with phase check runs on every compile, after the graph checks"
    reported_by_check      "its diagnostics join the compile's, with the codes and severities the pass returns, so specforge check, watch, the LSP and MCP report them"
    analyze_only_otherwise "a pass without phase check runs only under analyze, and analyze does not run a check pass"
    declared_order         "check passes run in the order their after/before constraints give, extension by extension in load order"
    trap_is_diagnostic     "a check pass that traps or answers output that does not parse is E028, never a crash"
    entity_span_attached   "a pass diagnostic with no span that names an entity gets that entity's span"
  }
  contract   """
    An extension declares its compiler passes in `__describe passes`
    (name, after, before, phase). A pass declared with `phase: "check"`
    is part of the compile: the shared compiled project runs it after the
    graph checks, so every surface reports what it finds and `specforge
    check` exits 1 on its errors. Which passes are check passes is read
    once, when the extensions load; a compile with none costs nothing.

    The pass export `__pass_<name>` receives the same input an analyze
    pass does (`entities`, `edges`), with no `test_results` or
    `proved_claims` (a compile has neither) and with `previous`, the
    statuses of the build cache (read_build_cache). It answers the same
    output: diagnostics, bare or as `{diagnostics, summary}`; the summary
    is ignored. Each diagnostic keeps the code and severity the pass gave
    it. One with no span that carries `entity: "<id>"` gets the span of
    that entity. A trap or an answer that does not parse is E028 naming
    the pass; the other passes still run.

    A pass with any other phase, or none, runs only under `specforge
    analyze`, which skips check passes: their findings are the compile's.
  """
  produces   []
  verify unit "a pass declared for the check phase runs on every compile"
  verify unit "a check pass's diagnostics are reported by specforge check"
  verify unit "a session reports a check pass's diagnostics after an update"
  verify unit "a pass without the check phase runs only under analyze"
  verify unit "check passes run in their declared after/before order"
  verify unit "a trapping check pass is a diagnostic, not a crash"
  verify unit "a pass diagnostic naming an entity gets that entity's span"
}

behavior write_build_cache "Write the Build Cache" {
  features [ci_integration]
  category command
  types    [Graph, Diagnostic]
  ports    [FileSystem]
  requires {
    project_compiled "the project compiled and its diagnostics are known"
  }
  ensures {
    statuses_recorded "check --cache writes every entity whose kind declares a lifecycle field, with its kind and that field's value, to specforge-cache.json at the project root"
    deterministic     "the file is byte-identical for the same sources"
    opt_in            "check without --cache never writes the file, and no other command or surface writes it"
    clean_builds_only "the file is written only when check exits 0; otherwise the previous file is left as it was"
  }
  contract """
    The build cache is the explicit, opt-in record of the lifecycle states
    of one build, so history rules (status transitions) compare against a
    declared input, never hidden state. `specforge check --cache` writes
    it after the compile, to `specforge-cache.json` beside
    `specforge.json`:
    `{"format": 1, "statuses": {"<entity id>": {"kind": "<kind>",
    "status": "<status>"}}}`, entities sorted by id, pretty-printed with
    a final newline. Only entities whose kind declares a `lifecycle_field`
    (ADR 0009) and that give it a text value are recorded, the value under
    `status` whatever the field is called; no field name is known to the
    cache. The file is replaced atomically (written beside, then
    renamed). A check that exits non-zero (errors, or warnings under
    `--strict`) does not write it: a broken build is not a baseline.
    A --severity filter does not change whether the file is written.
    CI may commit the file to check transitions across builds.
  """
  produces []
  verify unit "check --cache records each entity's kind and lifecycle state"
  verify unit "a kind without a lifecycle field is not recorded"
  verify unit "the cache file is deterministic"
  verify integration "check without --cache never writes the cache"
  verify unit "check --cache with errors leaves the cache untouched"
  verify unit "check --strict --cache with warnings leaves the cache untouched"
  verify unit "a check that passes records the cache and says so"
  verify integration "MCP validate never writes the build cache"
}

behavior read_build_cache "Read the Build Cache" {
  features [ci_integration]
  category validation
  types    [Diagnostic]
  ports    [FileSystem]
  requires {
    check_pass_loaded "at least one check-phase pass is loaded"
  }
  ensures {
    previous_given     "when specforge-cache.json parses, every check pass receives its statuses as previous"
    absent_is_none     "without the file, previous is absent"
    invalid_is_warning "a file that cannot be read or parsed, or declares another format, is W144 and previous is absent"
    passes_only        "the file is read on each compile with a check pass, by every surface, and never otherwise"
  }
  contract """
    When the project root has `specforge-cache.json`, each compile reads
    it as a declared input and hands it to the check passes as
    `previous: {"statuses": {"<entity id>": {"kind", "status"}}}`. With
    no file, `previous` is absent, so history rules stay silent on a
    first build. A file that cannot be read, is not valid JSON, or has a
    `format` other than 1 is a W144 warning and `previous` is absent. The
    file is read on every compile that runs a check pass (check, watch,
    the LSP, MCP), so a cache another command just wrote is seen by the
    next compile; with no check pass loaded, it is not read.
  """
  produces []
  verify unit "check passes receive the cached statuses as previous"
  verify unit "without a cache file previous is absent"
  verify unit "an invalid cache file is W144 and previous is absent"
}

behavior enforce_per_call_site_permissions "Enforce Per-Call-Site Permissions" {
  features   [contribution_based_extensions]
  invariants [wasm_sandbox_integrity]
  category   command
  types      [ManifestV2, SandboxPolicy]
  ports      [WasmRuntime]
  requires {
    sandbox_policy_ready    "sandbox policy has been computed for the extension"
    contribution_type_known "the contribution type of the current export call site is known"
  }
  ensures {
    contribution_permission_denied_emitted "contribution_permission_denied event is emitted when unauthorized host function is called"
    per_call_site_enforced                 "permissions are enforced per export call site, not per extension"
    unauthorized_calls_rejected            "calls to unauthorized host functions are rejected"
  }
  contract   """
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

    Surface contributions (cmd__, mcp__ exports) are granted no
    capability at all (invariant surface_sandbox_ceiling,
    behaviors/surface-contributions.spec). This behavior covers
    compile-time contribution call sites only.
  """
  produces   [contribution_permission_denied]
  verify unit "validator export limited to query_graph and emit_diagnostic"
  verify unit "renderer export additionally allows emit_file"
  verify unit "provider export additionally allows http_get"
  verify unit "entity contribution export limited to query_graph, add_graph_node, and add_graph_edge"
  verify unit "collector contribution export limited to query_graph and emit_file"
  verify unit "parser contribution export limited to emit_diagnostic, add_graph_node, add_graph_edge, and read_file"
  verify unit "unauthorized host function call is rejected"
  verify contract "Enforce Per-Call-Site Permissions: per-call-site permission enforcement holds — sandbox_policy_ready, contribution_type_known, contribution_permission_denied_emitted, per_call_site_enforced, unauthorized_calls_rejected"
}

behavior validate_contribution_exports "Validate Contribution Exports" {
  features   [contribution_based_extensions]
  invariants [host_function_type_safety]
  category   validation
  types      [ManifestV2, ExtensionError]
  ports      [WasmRuntime]
  requires {
    extension_loaded_ready          "extension .wasm binary has been loaded into the runtime"
    manifest_contributions_declared "extension manifest declares compile-time contributions to validate"
  }
  ensures {
    contribution_exports_validated_emitted        "contribution_exports_validated event is emitted when all declared exports are present"
    contribution_export_validation_failed_emitted "contribution_export_validation_failed event is emitted when exports are missing"
    missing_exports_diagnosed                     "missing exports produce E020 diagnostic listing expected export names"
  }
  contract   """
    After loading an extension, the compiler MUST verify that the .wasm binary
    exports all functions required by its declared compile-time contributions.
    Missing exports MUST produce an E020 diagnostic listing the expected export
    names. Extra exports beyond declared contributions MUST be ignored.
    Surface contribution exports (cmd__, mcp__) are not checked at load:
    a missing one is an E028 when dispatched (ADR 0011).
  """
  produces   [contribution_exports_validated, contribution_export_validation_failed]
  verify unit "all declared contribution exports present passes"
  verify unit "missing contribution export produces E020"
  verify unit "extra exports beyond contributions are ignored"
  verify contract "Validate Contribution Exports: contribution export validation holds — extension_loaded_ready, manifest_contributions_declared, contribution_exports_validated_emitted, contribution_export_validation_failed_emitted, missing_exports_diagnosed"
}

behavior toggle_extension_contributions "Toggle Extension Contributions" {
  features   [contribution_based_extensions]
  invariants [extension_load_order_determinism]
  category   command
  types      [ManifestV2, ExtensionContributions]
  ports      [CompilerApi]
  requires {
    extension_loaded_ready "extension is loaded and initialized before contributions can be toggled"
    config_available       "specforge.json configuration is available for reading contribution toggle state"
  }
  ensures {
    contribution_toggled_emitted   "contribution_toggled event is emitted after toggle state is applied"
    disabled_contributions_skipped "disabled contributions are skipped during dispatch"
    sole_provider_warned           "disabling the only entity provider for a kind produces W145 warning"
  }
  contract   """
    The specforge.json configuration MUST support enabling or disabling
    individual contributions from an extension. Disabled contributions MUST
    be skipped during dispatch. The extension MUST still be loaded and
    initialized — only the disabled contribution exports are not called.
    Disabling the only entity provider for a kind MUST produce a W145
    warning listing the affected entity kind.
  """
  produces   [contribution_toggled]
  verify unit "disabled contribution is skipped during dispatch"
  verify unit "extension still loaded when some contributions disabled"
  verify unit "re-enabled contribution resumes normal dispatch"
  verify unit "disabling only entity provider for a kind produces W145"
  verify contract "Toggle Extension Contributions: extension contribution toggling holds — extension_loaded_ready, config_available, contribution_toggled_emitted, disabled_contributions_skipped, sole_provider_warned"
}

// -- Collector Contribution Behaviors -----

// Cross-ref: the collect flow (ADR 0002) spans these behaviors:
// register_collector_contributions → auto_detect_collector →
// approve_collector_command → run_collector_command → dispatch_collector →
// ingest_collector_report. The CLI entry point is `specforge collect`; the
// MCP entry point is provide_mcp_collect_tool (behaviors/mcp-operations.spec).
behavior register_collector_contributions "Register Collector Contributions" {
  features   [test_result_collection]
  invariants [extension_load_order_determinism]
  category   query
  types      [ManifestV2, CollectorContribution, CollectorAutoDetect]
  ports      [WasmRuntime]
  requires {
    manifest_declares_collectors "an enabled extension's handshake raises the collectors flag and its describe payload lists collectors"
  }
  ensures {
    collectors_listed       "every declared collector is listed with its extension, export, detection files, command and report location"
    default_report_location "a collector that names no report location reads .specforge/reports/<name>.json"
  }
  contract   """
    Collectors come from the `collectors` describe payload of each enabled
    extension, in manifest order. A collector names its pure export
    (`collect__<name>`), the project-root files that select it, the argv
    that runs its test runner (elements may contain the `{report}`
    placeholder) and the report file or directory the runner writes,
    relative to the project root. A collector without a report location
    reads `.specforge/reports/<name>.json`.
  """
  produces   [collector_registered]
  verify unit "collector contribution parsed from manifest"
  verify contract "Register Collector Contributions: collector contribution registration holds — manifest_declares_collectors, collectors_listed, default_report_location"
}

// NOTE: auto_detect_collector does not produce an event because dispatch is
// CLI-initiated (specforge collect), not event-driven.
behavior auto_detect_collector "Auto-Detect Collector" {
  features   [test_result_collection]
  invariants [extension_load_order_determinism]
  category   validation
  types      [CollectorContribution, CollectorAutoDetect]
  ports      [FileSystem]
  consumes   [collector_registered]
  requires {
    collector_registered_fired "collector_registered event has fired, confirming collectors are available for auto-detection"
  }
  ensures {
    all_matches_selected    "every collector whose detection files exist at the project root is selected"
    runner_flag_selects_one "--runner selects exactly the collector with that name or extension"
    single_collector_used   "a single enabled collector is used without detection"
    no_match_diagnosed      "no collector, no match among several or an unknown --runner is E058, listing the available collectors"
  }
  contract   """
    Without `--runner`, `specforge collect` selects every collector whose
    detection files exist at the project root; the last segment of a
    detection pattern may use `*` wildcards (`vitest.config.*`). A project
    with Rust and TypeScript tests therefore collects from both runners.
    `--runner` names one collector by name or extension. When only one
    collector is enabled, it is selected without detection: detection
    only decides between several. When nothing can be
    selected, the command fails with E058 and lists the collectors that are
    available.
  """
  produces   []
  verify unit "file pattern match selects collector"
  verify unit "wildcards match within the last path segment"
  verify unit "no match emits E058 with available collectors"
  verify integration "a single enabled collector is used without detection"
  verify contract "Auto-Detect Collector: collector auto-detection holds — collector_registered_fired, all_matches_selected, runner_flag_selects_one, single_collector_used, no_match_diagnosed"
}

behavior approve_collector_command "Approve Collector Command" {
  features   [test_result_collection]
  invariants [extension_isolation]
  category   validation
  types      [CollectorContribution]
  ports      [FileSystem]
  requires {
    command_declared "the selected collector declares a command"
  }
  ensures {
    consent_outside_project   "approvals are stored in the user-level consent store, never in the project"
    changed_command_reprompts "an approval covers one project, extension, collector and exact argv; any change asks again"
    non_interactive_refuses   "without a terminal, or with JSON output, an unapproved command is refused with E059 unless --yes is passed"
  }
  contract   """
    `specforge collect` runs a collector's command only after the user
    approves it. It shows the extension, the project root and the exact
    command, asks once, and remembers the answer per project, extension,
    collector and argv in `~/.specforge/collector-consent.json`
    (`$SPECFORGE_CONSENT_FILE` overrides). The store lives outside the
    project, so a cloned repository can't approve its own commands. A
    changed command asks again. Without a terminal, or with `--format
    json`, nothing is asked: an unapproved command fails with E059, `--yes`
    runs it without recording an approval, and `--no-run` parses an
    existing report. The MCP server never prompts and runs only approved
    commands.
  """
  produces   []
  verify unit "consent is keyed by project, extension and command"
  verify integration "unapproved command without a terminal fails with E059 and runs nothing"
  verify integration "--yes runs the declared command"
  verify contract "Approve Collector Command: consent holds — command_declared, consent_outside_project, changed_command_reprompts, non_interactive_refuses"
}

behavior run_collector_command "Run Collector Command" {
  features   [test_result_collection]
  invariants [extension_isolation]
  category   command
  types      [CollectorContribution]
  ports      [FileSystem]
  requires {
    command_approved "approve_collector_command allowed the command"
  }
  ensures {
    report_inside_project  "a report location outside the project (absolute or with ..) is refused with E058"
    stale_report_ignored   "an earlier run's report is never read: a report file is removed before the command starts, and only report-directory files written during the run are read"
    report_path_exported   "the command runs in the project root with SPECFORGE_REPORT set to the absolute report path and {report} expanded"
    failing_tests_recorded "a non-zero exit is not an error when a report was written; no report is E045"
  }
  contract   """
    The host, not the extension, runs the approved command: extensions stay
    pure wasm with no process access. The report location must stay inside
    the project. So that an old report can't pass for a new run, the host
    removes a report file before running, and reads only the `*.json` files
    of a report directory that were written after the command started (it
    deletes nothing there: other tools may keep files in it). The command runs
    in the project root with stdin closed, `{report}` expanded in its
    arguments and `SPECFORGE_REPORT` set to the absolute report path. Its
    output goes to the terminal, to stderr under `--format json`, and
    nowhere under MCP. Test runners exit non-zero when tests fail, so the
    exit status only matters when no report was written, which is E045.
    A collector that declares `capture: "stdout"` gets the command's
    standard output too: it still reaches the terminal (or stderr) as it
    is produced, and is also written to `<collector>.stdout.txt` inside a
    report directory (next to a report file), which `--no-run` reads back.
    A captured output counts as a report. Any other `capture` value is
    E058.
  """
  produces   []
  verify unit "report path must stay inside the project"
  verify unit "run ignores stale reports and sets the report env"
  verify unit "command line expands the report placeholder"
  verify unit "a captured stdout is kept in every output mode"
  verify unit "an unknown capture is refused"
  verify contract "Run Collector Command: collector command execution holds — command_approved, report_inside_project, stale_report_ignored, report_path_exported, failing_tests_recorded"
}

behavior dispatch_collector "Dispatch Collector" {
  features   [test_result_collection]
  invariants [wasm_sandbox_integrity, extension_isolation]
  category   query
  types      [CollectorContribution, CollectorDispatchInput, CollectorReport, WasmTrapInfo]
  ports      [WasmRuntime, FileSystem]
  requires {
    report_available "the collector's report exists: a file, or a directory of *.json files"
  }
  ensures {
    collector_dispatched_emitted "collector_dispatched event is emitted after the export returns"
    report_files_passed          "the export receives every report file with its project-relative path and text"
    traps_reported               "a trap or malformed answer is E028"
  }
  contract   """
    The host reads the report (the file itself, or every `*.json` file
    directly inside a report directory, in path order) and passes the files
    to the collector's pure export as `{"reports": [{"path", "content"}],
    "stdout"?}`, `stdout` being the captured standard output when the
    collector declares one.
    The export answers with test results grouped by entity:
    `{"entity_results": [{"entity_id", "test_results": [{"name", "status",
    "verify"?, "duration_ms"?}]}], "unlinked"?: [{"name", "path",
    "status"}]}`, where status is passed, failed or skipped and `unlinked`
    lists tests the report doesn't link to an entity. A trap or an answer that doesn't parse is E028. `--no-run`
    and `--report` skip running and dispatch an existing report.
  """
  produces   [collector_dispatched]
  verify unit "reads a file or every json file in a directory"
  verify contract "Dispatch Collector: collector dispatch holds — report_available, collector_dispatched_emitted, report_files_passed, traps_reported"
}

behavior ingest_collector_report "Ingest Collector Report" {
  features   [test_result_collection]
  invariants [collector_output_conformance]
  category   query
  types      [CollectorReport, Graph]
  ports      [FileSystem]
  consumes   [collector_dispatched]
  requires {
    collector_dispatched_fired "collector_dispatched event has fired with the export's answer"
    graph_available            "the compiled graph is available to check entity IDs"
  }
  ensures {
    collector_report_ingested_emitted "collector_report_ingested event is emitted after the report is written"
    runner_results_replaced           "a runner's earlier results are replaced; other runners' results are kept"
    unknown_entities_warned           "results for undeclared entities are dropped with W115"
    skipped_not_recorded              "skipped tests are counted but not recorded"
    merged_report_written             "the merged results are written to specforge-report.json, which analyze reads by default"
  }
  contract   """
    Each runner's answer is merged into `specforge-report.json`: the
    runner's earlier results are removed and its new ones added, so several
    runners can report on one project. Every recorded test carries its
    name, `pass` or `fail`, the `verify` obligation it names and the runner
    that recorded it. Results for entities the graph doesn't declare are
    dropped with a W115 warning. Skipped tests prove nothing, so they're
    counted but not recorded. `specforge analyze` reads the written report
    without `--test-results`.
  """
  produces   [collector_report_ingested]
  verify unit "merge replaces only the same runner"
  verify integration "collect then analyze scores the recorded tests"
  verify contract "Ingest Collector Report: collector report ingestion holds — collector_dispatched_fired, graph_available, collector_report_ingested_emitted, runner_results_replaced, unknown_entities_warned, skipped_not_recorded, merged_report_written"
}

behavior resolve_test_conventions "Resolve Test Naming Conventions" {
  features   [test_result_collection]
  invariants [collector_output_conformance]
  category   query
  consumes   [collector_dispatched]
  requires {
    unlinked_returned "the collector returned the tests its report doesn't link"
  }
  ensures {
    double_underscore_linked "entity_id__slug links the test to that entity"
    module_linked            "otherwise the innermost module named after an entity links it"
    obligation_by_slug       "the rest of the name proves the one obligation whose slug it is"
    ambiguity_warned         "a name that splits into several entities is W137 and not linked"
    unresolved_silent        "a test no convention links is left out without a diagnostic"
  }
  contract   """
    A test the collector's report doesn't link (a plain `#[test]` seen in
    libtest's output, say) is linked by its name before the merge. When
    the part of the test's own name before a `__` is a declared entity ID,
    the test belongs to that entity and the rest of the name is checked
    against its obligations; otherwise the innermost enclosing module
    named after an entity takes it, with the whole name checked. The test
    proves the obligation whose slug equals the name's slug, when exactly
    one does, and otherwise the entity alone. A name whose `__` splits
    give more than one declared entity is W137 and stays unlinked. Tests
    no rule links are dropped silently, since plain tests are the norm.
    `specforge collect` reports how many tests it linked this way.
  """
  produces   []
  verify unit "a double underscore splits the entity from the obligation"
  verify unit "an entity with single underscores is not split"
  verify unit "the innermost module named after an entity links its tests"
  verify unit "a double underscore takes precedence over the module"
  verify unit "an obligation matches only when its slug is unique"
  verify unit "a name that splits into several entities is W137"
  verify unit "a test no convention links is left out silently"
  verify integration "collect links plain tests by naming convention"
  verify integration "a plain cargo test proves obligations by naming convention"
}

behavior slug_obligation_text "Slug an Obligation Text" {
  features   [test_result_collection]
  invariants [collector_output_conformance]
  category   query
  ensures {
    one_algorithm "the host and the specforge-test crate slug a text the same way"
  }
  contract   """
    A test named after an obligation carries the obligation's slug:
    `<=`, `>=`, `<` and `>` become `lte`, `gte`, `lt` and `gt`, spaces
    become underscores, ASCII letters are lowercased, every other character
    outside `[a-z0-9_]` is dropped, runs of underscores collapse to one and
    leading or trailing underscores are trimmed. The host and the
    `specforge-test` crate each carry this algorithm and are held to the
    same test vectors, so a name that links in one links in the other.
  """
  verify unit "slug matches the shared test vectors"
}

// -- Discovery & Configuration -----

behavior discover_extensions "Discover Extensions" {
  features   [wasm_extension_maintenance]
  invariants [
    extension_load_order_determinism,
    registry_integrity,
    offline_first_extension_resolution,
  ]
  category   command
  types      [ExtensionSource, ManifestV2, ExtensionError]
  ports      [WasmRuntime]
  requires {
    registries_configured "at least one registry source (npm, OCI, GitHub Releases) is configured"
  }
  ensures {
    extensions_discovered_emitted "extensions_discovered event is emitted with aggregated discovery results"
    network_failure_graceful      "network failures produce warning diagnostic without aborting discovery"
    results_complete              "results include extension name, available versions, description, and source registry"
  }
  contract   """
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
  produces   [extensions_discovered]
  verify unit "queries configured registries for available extensions"
  verify unit "checks for updates to installed extensions"
  verify unit "aggregates results across multiple registries"
  verify unit "network failure produces warning without aborting"
  verify contract "Discover Extensions: extension discovery holds — registries_configured, extensions_discovered_emitted, network_failure_graceful, results_complete"
}

behavior run_doctor_check "Run Doctor Check" {
  features   [entity_enhancement]
  // Doctor REPORTS on invariant violations — it does not ENFORCE them.
  // Enforcement is done by the behaviors listed in each invariant's enforced_by.
  category   validation
  invariants [diagnostic_determinism]
  types      [ManifestV2, EnhancementConflict, FieldEnhancement]
  ports      [FileSystem]
  consumes   [enhancement_registered, wasm_trap_caught]
  requires {
    enhancement_registered_fired "enhancement_registered event has fired, confirming FieldRegistry is built"
    filesystem_available         "FileSystem port is available for reading extension manifests"
  }
  ensures {
    doctor_check_completed_emitted "doctor_check_completed event is emitted after all checks finish"
    report_produced                "human-readable report listing extensions, enhancements, and conflicts is produced"
    json_output_supported          "--json flag produces machine-readable JSON output for CI integration"
  }
  contract   """
    When specforge doctor is invoked, the system MUST load all extension
    manifests, build the FieldRegistry, detect all conflicts, and
    produce a human-readable report listing installed extensions, their
    enhancements, any conflicts with actionable resolution suggestions,
    and additional checks (shadowed fields, unknown target entities,
    edge label conflicts). An enabled extension that fails to load (E028:
    not installed; E033: its binary no longer matches the lock) MUST be
    reported as an error. A remediation that names a command MUST name
    one the user can run as written. A finding whose diagnostic offers no
    suggestion of its own MUST quote the catalogue's explanation of its
    code. The --json flag MUST produce machine-readable JSON
    output for CI integration.
  """
  produces   [doctor_check_completed]
  verify unit "doctor lists all installed extensions with enhancement counts"
  verify unit "doctor lists all enhancements grouped by entity kind"
  verify unit "doctor reports conflicts with resolution suggestions"
  verify unit "doctor detects shadowed grammar-level constructs"
  verify unit "doctor --json produces valid JSON output"
  verify unit "doctor reports an extension that fails to load (E028, E033) as an error"
  verify unit "a peer whose installed version doctor cannot compare is remedied with a runnable command"
  verify unit "a finding without its own suggestion quotes the catalogued explanation"
  verify contract "Run Doctor Check: doctor check holds — enhancement_registered_fired, filesystem_available, doctor_check_completed_emitted, report_produced, json_output_supported"
}

// -- Extension Source Resolution -----

behavior parse_extension_specifier "Parse Extension Specifier" {
  features   [wasm_extension_installation]
  invariants [registry_integrity]
  category   command
  types      [ExtensionSpecifier, ExtensionSource, ExtensionError]
  requires {
    specifier_string_provided "a raw extension specifier string is provided for parsing"
  }
  ensures {
    extension_specifier_parsed_emitted "extension_specifier_parsed event is emitted with structured source descriptor"
    invalid_specifier_diagnosed        "invalid specifiers produce ExtensionError diagnostic with expected format"
  }
  contract   """
    The system MUST parse extension specifier strings into structured
    source descriptors. Supported formats: "@scope/name@version" for
    registry extensions, "./path" for local extensions, and "git:url#ref"
    for git-sourced extensions. Invalid specifiers MUST produce a
    ExtensionError diagnostic with the expected format.
  """
  produces   [extension_specifier_parsed]
  verify unit "@scope/name@version parsed as registry source"
  verify unit "./path parsed as local source"
  verify unit "git:url#ref parsed as git source"
  verify unit "invalid specifier produces ExtensionError"
  verify contract "Parse Extension Specifier: extension specifier parsing holds — specifier_string_provided, extension_specifier_parsed_emitted, invalid_specifier_diagnosed"
}

behavior resolve_extension_source "Resolve Extension Source" {
  features   [wasm_extension_installation]
  invariants [registry_integrity]
  category   query
  types      [ManifestV2, ExtensionSpecifier, ExtensionSource, ExtensionError]
  ports      [FileSystem, RegistryClient]
  consumes   [extension_specifier_parsed]
  requires {
    extension_specifier_parsed_fired "extension_specifier_parsed event has fired, confirming structured source descriptor is available"
    source_ports_available           "FileSystem and RegistryClient ports are available for resolution"
  }
  ensures {
    extension_source_resolved_emitted "extension_source_resolved event is emitted with concrete manifest and .wasm binary"
    resolution_failure_diagnosed      "resolution failures produce ExtensionError diagnostic"
  }
  contract   """
    Given a parsed extension specifier, the system MUST resolve it to a
    concrete manifest and .wasm binary. Registry sources MUST query the
    registry API. Local sources MUST read from the filesystem. Git sources
    MUST clone or fetch the repository at the specified ref. Resolution
    failures MUST produce a ExtensionError diagnostic.
  """
  produces   [extension_source_resolved]
  verify unit "registry source resolves via registry API"
  verify unit "local source resolves from filesystem"
  verify unit "git source resolves from repository"
  verify unit "resolution failure produces ExtensionError"
  verify contract "Resolve Extension Source: extension source resolution holds — extension_specifier_parsed_fired, source_ports_available, extension_source_resolved_emitted, resolution_failure_diagnosed"
}

// -- Lock File Management -----

behavior write_lock_file "Write Lock File" {
  features   [wasm_lock_management]
  invariants [extension_load_order_determinism, registry_integrity]
  category   command
  types      [ManifestV2, LockFile, LockFileEntry]
  ports      [FileSystem]
  requires {
    extensions_resolved  "all extensions have been resolved with exact versions and wasm hashes"
    filesystem_available "FileSystem port is available for writing lock file"
  }
  ensures {
    lock_file_written_emitted "lock_file_written event is emitted after successful write"
    output_deterministic      "same inputs always produce byte-identical lock file output"
    write_atomic              "lock file is written atomically to prevent corruption"
  }
  contract   """
    After resolving all extensions, the system MUST write a specforge.lock
    file containing the exact resolved version and SHA256 wasm_hash for
    each installed extension. The lock file format MUST be deterministic —
    same inputs always produce byte-identical output. The resolved_at
    timestamp is metadata excluded from the determinism guarantee. The
    lock file MUST be written atomically to prevent corruption.
  """
  produces   [lock_file_written]
  verify unit "lock file contains exact versions and wasm hashes"
  verify unit "lock file output is deterministic"
  verify unit "lock file written atomically"
  verify contract "Write Lock File: lock file writing holds — extensions_resolved, filesystem_available, lock_file_written_emitted, output_deterministic, write_atomic"
}

behavior read_lock_file "Read Lock File" {
  features   [wasm_lock_management]
  invariants [extension_load_order_determinism]
  category   command
  types      [ManifestV2, LockFile, LockFileEntry, ExtensionError]
  ports      [FileSystem]
  consumes   [all_files_parsed]
  requires {
    all_files_parsed_fired "all_files_parsed event has fired, confirming spec files are parsed"
    filesystem_available   "FileSystem port is available for reading lock file"
  }
  ensures {
    lock_file_read_emitted  "lock_file_read event is emitted after lock file is processed"
    locked_versions_used    "locked versions are used instead of resolving from sources when lock file exists"
    malformed_lock_graceful "malformed lock files produce warning and fall back to fresh resolution"
  }
  contract   """
    When a specforge.lock file exists, the system MUST use locked versions
    instead of resolving from sources. Missing lock entries for declared
    extensions MUST trigger resolution and lock file update. Malformed lock
    files MUST produce a warning and fall back to fresh resolution.
  """
  produces   [lock_file_read]
  verify unit "locked versions used when lock file exists"
  verify unit "missing lock entry triggers resolution"
  verify unit "malformed lock file produces warning and falls back"
  verify contract "Read Lock File: lock file reading holds — all_files_parsed_fired, filesystem_available, lock_file_read_emitted, locked_versions_used, malformed_lock_graceful"
}

// ── Extension Update ──────────────────────────────────────────

behavior update_all_extensions "Update All Extensions" {
  features   [wasm_extension_maintenance]
  invariants [
    wasm_sandbox_integrity,
    peer_dependency_satisfaction,
    wasm_compile_cache_integrity,
    extension_operation_atomicity,
  ]
  category   command
  types      [ManifestV2, LockFileEntry, ExtensionError]
  ports      [WasmRuntime]
  requires {
    extensions_installed "at least one extension is installed with a valid manifest"
    registries_reachable "configured registries are reachable for version checks"
  }
  ensures {
    batch_update_completed_emitted "batch_update_completed event is emitted after all upgrades are applied"
    semver_constraints_respected   "upgrades respect semver constraints, major bumps skipped without --major"
    lock_hashes_refreshed          "updated lock entries record the new binary hashes"
    atomic_rollback_on_failure     "if any upgrade fails, all changes are rolled back"
  }
  contract   """
    When specforge update is invoked, the system MUST check all installed
    extensions for newer versions by querying their configured registries.
    The system MUST upgrade each extension to the latest compatible version
    respecting semver constraints. Major version bumps MUST be skipped unless
    --major is specified. After upgrading, the system MUST refresh the lock
    file with the new binary hashes. Peer dependency
    conflicts introduced by upgrades MUST be detected and reported before
    applying changes. If any upgrade fails, the system MUST roll back all
    changes and report the failure. Only registry installs are updated: an
    extension installed from a local file (source local:<path>) MUST NOT be
    replaced from a registry. When a registry-installed extension is
    to be updated and no registry is configured in specforge.json, update
    MUST make no network call and MUST fail with E063, whose suggestion
    names the registries key.
  """
  produces   [batch_update_completed]
  verify unit "with no registry configured, update makes no network call and reports how to configure one"
  verify unit "newer versions detected from registry"
  verify unit "semver-compatible upgrades applied"
  verify unit "major version skipped without --major flag"
  verify unit "lock file records new binary hashes after update"
  verify unit "peer dependency conflicts detected before applying"
  verify unit "failed upgrade rolls back all changes"
  verify integration "update never replaces a locally installed extension from a registry"
  verify contract "Update All Extensions: batch extension update holds — extensions_installed, registries_reachable, batch_update_completed_emitted, semver_constraints_respected, lock_hashes_refreshed, atomic_rollback_on_failure"
}

behavior refresh_lock_file "Refresh Lock File" {
  features   [wasm_lock_management]
  invariants [wasm_compile_cache_integrity, registry_integrity]
  category   command
  types      [LockFileEntry, ManifestV2]
  ports      [WasmRuntime, FileSystem]
  requires {
    lock_file_exists     "specforge.lock file exists with entries to refresh"
    registries_reachable "configured registries are reachable for re-resolution"
  }
  ensures {
    lock_file_refreshed_emitted "lock_file_refreshed event is emitted after lock file is regenerated"
    versions_unchanged          "pinned versions are not changed during refresh"
    hashes_verified             "SHA256 hashes of all installed .wasm binaries are verified against lock entries"
  }
  contract   """
    When specforge update --lock is invoked, the system MUST re-resolve all
    extension specifiers from their configured registries without changing
    pinned versions. The system MUST verify SHA256 hashes of all installed
    .wasm binaries against the lock file entries. Mismatched hashes MUST
    produce a warning diagnostic. The lock file MUST be regenerated with
    current resolution metadata including timestamps and registry URLs.
  """
  produces   [lock_file_refreshed]
  verify unit "specifiers re-resolved without version changes"
  verify unit "SHA256 hashes verified against lock entries"
  verify unit "mismatched hash produces warning"
  verify unit "lock file regenerated with current metadata"
  verify contract "Refresh Lock File: lock file refresh holds — lock_file_exists, registries_reachable, lock_file_refreshed_emitted, versions_unchanged, hashes_verified"
}
