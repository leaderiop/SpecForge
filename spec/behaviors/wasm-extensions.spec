// Extension entity kinds, enhancements, conflicts, queries, contributions,
// collectors, discovery, lock files, and doctor

use invariants/wasm
use types/wasm
use types/zero-entity-core
use types/config
use types/errors
use types/graph
use ports/inbound
use ports/outbound
use events/wasm-extensions

// -- Query Extensions -----

behavior provide_extension_query_extensions "Provide Extension Query Extensions" {
  invariants [host_function_type_safety]
  types      [ManifestV2, QueryExtension, QueryFileKind, ExtensionError]
  ports      [WasmRuntime]

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

}

behavior compose_query_files_from_extensions "Compose Query Files From Extensions" {
  invariants [extension_load_order_determinism]
  types      [QueryExtension, QueryFileKind]
  ports      [WasmRuntime]

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

}

// -- Entity Kind Conflict Prevention -----

behavior reject_reserved_entity_kind "Reject Reserved Entity Kind" {
  invariants [entity_kind_uniqueness]
  types      [HostFunctionBinding, ManifestV2]
  ports      [WasmRuntime]

  contract """
    The KindRegistry MUST reject entity kind names that match structural
    DSL keywords parsed by dedicated grammar rules: spec, use, define, ref,
    verify, gherkin, true, false. These are the ONLY core-reserved words —
    they have dedicated grammar rules or are literal tokens and cannot be
    used as entity kind names. Domain keywords like behavior, feature,
    invariant, etc. are NOT reserved because they come from extensions. An
    extension registering "behavior" is valid (e.g., a software-domain
    extension provides the "behavior" keyword). Gherkin sub-keywords
    (scenario, given, when, then) are NOT core-reserved — they are reserved
    by the extension that provides gherkin support (e.g., @specforge/software)
    via the extension's own reserved_keywords manifest field. Rejection MUST
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

}

// User-facing conflict resolution layer. Distinct from detect_duplicate_entity_kinds
// (behaviors/zero-entity-validation.spec) which handles registry-level detection
// during manifest loading. This behavior handles the policy-based resolution UI.
behavior detect_entity_kind_collision "Detect Entity Kind Collision" {
  invariants [entity_kind_uniqueness]
  types      [ManifestV2, ExtensionError]

  contract """
    The KindRegistry MUST detect when two extensions attempt to register
    the same entity kind name. Collisions with structural keywords (spec,
    use, define) MUST always produce an unresolved conflict (E023).
    Collisions with define block kinds MUST always produce an unresolved
    conflict (E022). Collisions between extension kinds MUST be recorded
    for policy-based resolution via entity_kind_policy in specforge.json.
  """

  produces [entity_kind_conflict_detected]

  verify unit "two extensions registering same kind produces conflict"
  verify unit "collision with structural keyword produces E023"
  verify unit "collision with define block produces E022"
  verify unit "no false positive for different kind names"

}

behavior resolve_entity_kind_conflict_via_config "Resolve Entity Kind Conflict Via Config" {
  invariants [entity_kind_uniqueness]
  types      [ManifestV2, ExtensionError]

  contract """
    When entity kind conflicts between extensions are detected, the
    compiler MUST apply the configured entity_kind_policy from
    specforge.json. With policy "error" (default), unresolved
    conflicts MUST produce E026 diagnostics. With policy "priority",
    the first extension in the extensions array MUST win and a W027
    warning MUST be emitted. With policy "namespace", conflicting
    kinds MUST be prefixed with the extension short name. Explicit
    entity_kinds overrides MUST take precedence over any policy.
    The namespace prefix is derived from the extension name: for scoped
    names like @scope/name, the prefix is "name"; for unscoped names,
    the extension name itself is the prefix. Qualified kind names use
    the format "prefix.kind_name".
  """

  produces [entity_kind_conflict_resolved]

  verify unit "error policy produces E026 for unresolved conflicts"
  verify unit "priority policy selects first extension and emits W027"
  verify unit "namespace policy prefixes conflicting kinds"
  verify unit "explicit override takes precedence over policy"

}

behavior qualify_entity_kind_inline "Qualify Entity Kind Inline" {
  invariants [entity_kind_uniqueness]
  types      [ManifestV2, ExtensionError]

  contract """
    The parser MUST recognize the @extension/kind syntax as a qualified
    entity keyword. When encountered, the compiler MUST extract the
    extension name and kind name, resolve via the KindRegistry's
    resolve_qualified method, and produce an EntityKind::Custom with
    the qualified name. Unresolved qualified names MUST produce an
    error diagnostic.
  """

  produces [entity_kind_qualified]

  verify unit "parser recognizes @extension/kind syntax"
  verify unit "qualified kind resolves to correct extension"
  verify unit "unresolved qualified name produces error"

}

// -- Entity Enhancement -----

behavior load_extension_manifest "Load Extension Manifest" {
  invariants [extension_load_order_determinism]
  types      [ManifestV2, ExtensionError]
  ports      [FileSystem]
  produces   [manifest_loaded]

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
      7. initialize_wasm_extension — call initialize() export
    Steps 1-6 are declarative (manifest-driven). Step 7 is the first
    point at which extension code executes. This sequence is repeated
    per extension in topological order (see topological_sort_extensions).
  """

  verify unit "sidecar JSON parsed into ManifestV2"
  verify unit "malformed sidecar produces ExtensionError"
  verify unit "bundled extensions loaded from bundled resources directory"
  verify unit "initialization follows documented 7-step sequence"

}

behavior register_entity_enhancements "Register Entity Enhancements" {
  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
  types      [ManifestV2, FieldEnhancement, DynamicEdgeType]

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

}

behavior detect_enhancement_conflicts "Detect Enhancement Conflicts" {
  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
  types      [EnhancementConflict, FieldEnhancement, EnhancedFieldType, EnumFieldType, ReferenceFieldType]

  contract """
    During enhancement registration, the compiler MUST detect when two
    extensions register the same field name for the same entity kind. Each
    conflict MUST be recorded with both extension identities and the
    conflicting field types. Conflicts with grammar-level constructs
    (entity title, verify, gherkin) MUST always produce a hard
    error (E018). Conflicts between extensions MUST be resolved according
    to the configured enhancement_policy.
  """

  produces [enhancement_conflict_detected]

  verify unit "same (entity, field) from two extensions produces conflict"
  verify unit "conflict with grammar-level construct produces E018"
  verify unit "conflict record includes both extension identities"
  verify unit "no false positives for same field on different entities"

}

behavior resolve_enhancement_conflicts "Resolve Enhancement Conflicts" {
  invariants [enhancement_field_uniqueness]
  types      [EnhancementConflict, ConflictResolution, EnhancementPolicy]

  contract """
    When enhancement conflicts are detected, the compiler MUST apply
    the configured enhancement policy. With policy "error" (default),
    unresolved conflicts MUST produce E017 diagnostics. With policy
    "priority", the first extension in the extensions array MUST win and
    a W026 warning MUST be emitted. With policy "namespace", conflicting
    fields MUST be prefixed with the extension short name. Explicit
    enhancement_overrides in specforge.json MUST take precedence over
    any policy.
  """

  produces [enhancement_conflict_resolved]

  verify unit "error policy produces E017 for unresolved conflicts"
  verify unit "priority policy selects first extension and emits W026"
  verify unit "namespace policy prefixes conflicting fields"
  verify unit "explicit override takes precedence over policy"

}

// -- Contribution Model -----

behavior dispatch_contribution_exports "Dispatch Contribution Exports" {
  invariants [extension_load_order_determinism]
  types      [ManifestV2, ExtensionContributions]
  ports      [WasmRuntime]

  contract """
    When an extension declares contributions in its manifest, the compiler
    MUST route calls to the extension's namespaced Wasm exports based on
    the contribution type. Entity contributions MUST call initialize()
    and validate(). Renderer contributions (for non-code outputs such
    as reports, dashboards, traceability matrices) MUST call render().
    Provider contributions MUST call validate_ref(). Missing exports
    for declared contributions MUST produce E020.
  """

  produces [contribution_exports_dispatched]

  verify unit "entity contributions dispatched to initialize() and validate()"
  verify unit "renderer contributions dispatched to render()"
  verify unit "provider contributions dispatched to validate_ref()"
  verify unit "missing export for declared contribution produces E020"

}

behavior enforce_per_call_site_permissions "Enforce Per-Call-Site Permissions" {
  invariants [wasm_sandbox_integrity]
  types      [ManifestV2, SandboxPolicy]
  ports      [WasmRuntime]

  contract """
    Host function permissions MUST be enforced per export call site, not
    per extension. An extension's validator export MUST only access query_graph
    and emit_diagnostic. An extension's renderer export MUST additionally
    access emit_file. An extension's provider export MUST additionally access
    http_get. An extension's entity contribution exports MUST only access
    query_graph, add_graph_node, and add_graph_edge. Collector contributions
    MUST only access query_graph and emit_file. Calls to unauthorized host
    functions MUST be rejected.
  """

  produces [contribution_permission_denied]

  verify unit "validator export limited to query_graph and emit_diagnostic"
  verify unit "renderer export additionally allows emit_file"
  verify unit "provider export additionally allows http_get"
  verify unit "unauthorized host function call is rejected"

}

behavior validate_contribution_exports "Validate Contribution Exports" {
  invariants [host_function_type_safety]
  types      [ManifestV2, ExtensionError]
  ports      [WasmRuntime]

  contract """
    After loading an extension, the compiler MUST verify that the .wasm binary
    exports all functions required by its declared contributions. Missing
    exports MUST produce an E020 diagnostic listing the expected export
    names. Extra exports beyond declared contributions MUST be ignored.
  """

  produces [contribution_export_validation_failed]

  verify unit "all declared contribution exports present passes"
  verify unit "missing contribution export produces E020"
  verify unit "extra exports beyond contributions are ignored"

}

behavior toggle_extension_contributions "Toggle Extension Contributions" {
  invariants [extension_load_order_determinism]
  types      [ManifestV2, ExtensionContributions]
  ports      [CompilerApi]

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

}

// -- Collector Contribution Behaviors -----

behavior register_collector_contributions "Register Collector Contributions" {
  invariants [extension_load_order_determinism]
  types      [ManifestV2, CollectorContribution, ExtensionContributions, ExtensionError]
  ports      [WasmRuntime]

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

}

// NOTE: auto_detect_collector does not produce an event because dispatch is
// CLI-initiated (specforge collect), not event-driven. The CLI command
// directly selects and dispatches the collector — there is no intermediate
// event between detection and dispatch.
behavior auto_detect_collector "Auto-Detect Collector" {
  invariants [extension_load_order_determinism]
  types      [CollectorContribution, CollectorAutoDetect]
  ports      [FileSystem]

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

  verify unit "file pattern match selects collector"
  verify unit "env var match selects collector"
  verify unit "first match wins when multiple match"
  verify unit "no match emits I013 with available collectors"

}

behavior dispatch_collector "Dispatch Collector" {
  invariants [wasm_sandbox_integrity, extension_isolation]
  types      [CollectorContribution, CollectorDispatchInput, CollectorReport, ExtensionError, WasmTrapInfo]
  ports      [WasmRuntime, FileSystem]

  contract """
    When a collector is selected, the system MUST call the collector's
    declared Wasm export with the test report path and the set of known
    entity IDs as input. The collector Wasm function MUST return a
    CollectorReport. Wasm traps during collector execution MUST be caught
    and reported as ExtensionError diagnostics without affecting the
    compilation pipeline.
  """

  produces [collector_dispatched]

  verify unit "calls collector Wasm export with report path and entity IDs"
  verify unit "returns CollectorReport on success"
  verify unit "Wasm trap caught and reported as ExtensionError"

}

behavior validate_collector_output "Validate Collector Output" {
  invariants [collector_output_conformance]
  types      [CollectorReport, CollectorStats, ExtensionError]

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

}

behavior ingest_collector_report "Ingest Collector Report" {
  invariants [collector_output_conformance]
  types      [CollectorReport, CollectorReportEntry, Graph]
  ports      [FileSystem]

  contract """
    After validation, the system MUST associate each CollectorReportEntry
    with its corresponding entity node in the graph, update coverage
    metadata on the entity node, and write the merged report to
    specforge-report.json. Entries with unknown entity IDs (already
    warned by validate_collector_output) MUST be included in the
    unmapped_tests section of the output.
  """

  produces [collector_report_ingested]

  verify unit "entries associated with entity nodes"
  verify unit "coverage metadata updated on entity"
  verify unit "merged report written to specforge-report.json"
  verify unit "unknown entity entries in unmapped_tests"

}

// -- Discovery & Configuration -----

behavior discover_extensions "Discover Extensions" {
  invariants [extension_load_order_determinism]
  types      [ExtensionSource, ManifestV2, ExtensionError]
  ports      [WasmRuntime]

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

}

behavior run_doctor_check "Run Doctor Check" {
  invariants [enhancement_field_uniqueness, entity_kind_uniqueness]
  types      [ManifestV2, EnhancementConflict, FieldEnhancement]
  ports      [FileSystem]

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

}

// -- Extension Source Resolution -----

behavior parse_extension_specifier "Parse Extension Specifier" {
  invariants [registry_integrity]
  types      [ManifestV2, ExtensionSpecifier, ExtensionSource, ExtensionError]

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

}

behavior resolve_extension_source "Resolve Extension Source" {
  invariants [registry_integrity]
  types      [ManifestV2, ExtensionSpecifier, ExtensionSource, ExtensionError]
  ports      [WasmRuntime, FileSystem]

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

}

// -- Lock File Management -----

behavior write_lock_file "Write Lock File" {
  invariants [extension_load_order_determinism]
  types      [ManifestV2, LockFile, LockFileEntry]
  ports      [FileSystem]

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

}

behavior read_lock_file "Read Lock File" {
  invariants [extension_load_order_determinism]
  types      [ManifestV2, LockFile, LockFileEntry, ExtensionError]
  ports      [FileSystem]

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

}

// ── Extension Update ──────────────────────────────────────────

behavior update_all_extensions "Update All Extensions" {
  invariants [wasm_sandbox_integrity, peer_dependency_satisfaction, aot_cache_integrity, extension_operation_atomicity]
  types      [ManifestV2, LockFileEntry, ExtensionError]
  ports      [WasmRuntime]

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

}

behavior refresh_lock_file "Refresh Lock File" {
  invariants [aot_cache_integrity, registry_integrity]
  types      [LockFileEntry, ManifestV2]
  ports      [WasmRuntime]

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

}
