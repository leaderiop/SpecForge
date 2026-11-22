decision D-096 "Tree-sitter for Parsing" {
  settled true
  origin  project
  text    """
    Use Tree-sitter for the .spec grammar. It provides error recovery,
    incremental parsing, and a mature ecosystem with editor integrations.
    No hand-written recursive descent fallback.
  """
}

decision D-002 "Multi-Error Collection" {
  settled true
  origin  project
  text    """
    The compiler collects all errors, warnings, and info diagnostics during
    a compilation pass and emits them together. No fail-fast behavior.
    Diagnostics are sorted by (file_path, line, column, code) — source
    order within each file, not grouped by severity. This matches the
    rustc convention where developers see diagnostics in the order they
    appear in the source.
  """
}

decision D-093 "String Interning with lasso" {
  settled true
  origin  project
  text    """
    Use the lasso crate for string interning. Intern all entity IDs, file
    paths, and frequently compared strings. Use interned keys for all
    comparison and hashing operations.
  """
}

decision D-027 "Mutable In-Memory Graph" {
  settled true
  origin  project
  text    """
    The in-memory graph is mutable. It supports adding/removing nodes and
    edges. Incremental compilation mutates the graph in place, removing
    stale entries and inserting new ones from re-parsed files.
  """
}

decision D-091 "Snapshot Testing with insta" {
  settled true
  origin  project
  text    """
    Use the insta crate for snapshot testing. Diagnostic output and rendered
    markdown are tested via inline and file-based
    snapshots. cargo insta review for interactive approval.
  """
}

decision D-095 "Three-Layer Traceability Model" {
  settled true
  origin  project
  text    """
    Three-layer traceability: intent (verify declarations and extension-declared test file fields in .spec
    files), linkage (each test names the entity it proves, in its runner's
    idiom), and proof (the pass/fail results `specforge collect` records in
    specforge-report.json). `specforge analyze coverage` scores all three
    layers. (Linkage was first a `tests` field in .spec files; ADR 0002
    moved it into the tests themselves.)
  """
}

decision D-098 "Wasm Component Model Extension Runtime" {
  settled true
  origin  project
  text    """
    Use WebAssembly components (wasmtime Component Model, wasip2 guests)
    as the unified extension runtime for all contribution types. The
    guest contract is the specforge:bridge WIT world: one `call` export
    dispatching the JSON wire protocol by host-function name.
    Extensions expose host functions (specforge.query_graph,
    specforge.emit_diagnostic) — components never touch the OS directly
    beyond their WASI grants.
    Renderers use specforge.emit_file(path, content) instead of raw
    filesystem access, with the host validating paths and enforcing
    sandboxing. Providers use specforge.http_get for external
    service validation. Universal .wasm components are distributed via npm,
    GitHub Releases, or OCI registries. Wasmtime's on-disk compile cache
    makes CLI cold starts cheap; LSP and MCP keep one engine with
    instantiated components alive per session.
  """
}

decision D-014 "Extension Peer Dependencies" {
  settled true
  origin  project
  text    """
    Wasm extension manifests declare peer_dependencies with semver ranges. The
    registry build judges every peer dependency by one rule and loads each
    extension after the peers it declares, otherwise in specforge.json order
    (ADR 0041). All extensions share the same in-process graph via
    specforge.query_graph host function — no serialization between extensions.
    When a manifest declares a peer dependency that is not installed, the
    host emits a hard error. The existing I004 soft resolution remains for
    implicit cross-module references (fields without a manifest declaration).
  """
}

decision D-006 "Config-Driven Kind Registry" {
  settled true
  origin  project
  text    """
    Entity kind names are unique across the loaded extensions: the
    registry build rejects any extension's kind already registered by a
    previously loaded extension, emitting E026, and the first registration
    wins. Entity ids that collide with the grammar's structural keywords
    or an extension's kind keyword are E013. Conflict resolution is the
    extension author's responsibility — rename the kind or declare a peer
    dependency. The compiler never arbitrates domain-level conflicts;
    if two installed extensions declare the same kind, it is a hard error
    at startup, not a runtime policy decision.
  """
}

decision D-011 "Entity Enhancement Model" {
  settled true
  origin  project
  text    """
    Extension manifests declare entity enhancements with target entity kind,
    field name, field type, and optional edge mappings. The compiler builds
    a FieldRegistry combining extension-defined and enhanced fields, threads
    it through the resolve/graph-build/validate pipeline. An enhancement
    never overwrites a field the kind already has: the first registration
    in load order wins. specforge doctor provides visibility into all
    enhancements.
  """
}

decision D-097 "Wasm Compile Cache Strategy" {
  settled true
  origin  project
  text    """
    Use wasmtime's native on-disk compilation cache, configured when the
    engine is constructed (CacheConfig on Config::cache — after Engine
    creation the setting is inert, so the constructor is the only wiring
    point). The cache directory is selected by SPECFORGE_WASMTIME_CACHE
    (default $HOME/.cache/specforge/wasmtime; 'off' disables) with a
    500MB files-total-size soft limit. Entries are keyed by component
    bytes and engine config; integrity and cleanup are engine-owned.
    Host-side concerns stay separate: installed-binary integrity is the
    specforge.lock hash pin (E070) at load time.
  """
}

decision D-007 "Contribution-Based Extension Model" {
  settled true
  origin  project
  text    """
    Structured manifest with typed objects: entity_kinds EntityKindDescriptor[],
    validation_rules ValidationRulePattern[], edge_types EdgeTypeDescriptor[].
    Each extension declares a contributes key listing what it provides:
    entities, validators, renderers, providers, parsers. The compiler routes to
    namespaced Wasm exports based on contributions. Per-call-site host
    function permissions enforce least-privilege for each export.
  """
}

decision D-001 "Extension Source Resolution" {
  settled true
  origin  project
  text    """
    Three extension source types: registry (@scope/name@version for npm),
    local (./path for filesystem), and git (git:url#ref for repositories).
    The specifier format supports both string shorthand ("@specforge/product@^1.0")
    and object form ({ source: "local", path: "./my-extension" }) in
    specforge.json. Resolution produces a manifest + .wasm binary pair.
  """
}

decision D-015 "Extension Version Pinning" {
  settled true
  origin  project
  text    """
    specforge.lock pins exact resolved versions with SHA256 wasm_hash for
    each installed extension. The lock file format is deterministic — same
    inputs always produce byte-identical output. Integrity verification
    compares .wasm binary hashes against lock entries. specforge update
    explicitly bumps versions and refreshes the lock file.
  """
}

decision D-092 "SpecForge is a Specification Language" {
  settled true
  origin  project
  text    """
    SpecForge is a specification language that provides structured context
    for AI agents and humans to do their jobs better. SpecForge is
    domain-agnostic — it serves coding agents, compliance agents, PM agents,
    and any domain that benefits from validated entity graphs. The `verify`
    keyword declares test intent inside testable entity blocks. The renderer
    contribution type in Wasm extensions produces non-code outputs only
    (reports, dashboards, traceability matrices). The `collect` command and
    all spec validation/coverage features provide test traceability.
  """
}

decision D-100 "Zero-Entity Core Architecture" {
  settled true
  origin  project
  text    """
    The SpecForge compiler core has ZERO built-in entity types. All domain
    vocabulary comes from installable extensions. The core is a pure typed-graph
    engine: it parses any keyword-name-body block via the generic_entity_block
    grammar rule, resolves references, builds graphs, and validates constraints.
    Extensions declare entity_kinds, edge_types, validation_rules, and testability
    flags in their manifest. @specforge/software provides 6 entity kinds
    (behavior, invariant, feature, event, type, port),
    @specforge/product provides 5, @specforge/governance provides 3 — together
    reproducing all 14 domain entity kinds (6+5+3). ref remains a core
    structural keyword with a dedicated grammar rule. New domain
    extensions: @specforge/atomic-design, @specforge/compliance, @specforge/api-design,
    etc. This is the Terraform-exact model: core has zero domain knowledge.
  """
}

decision D-088 "Per-Call-Site Sandbox" {
  settled true
  origin  project
  text    """
    Host function permissions are enforced per export call site, not per
    extension. Each contribution type has a fixed permission set: validators
    get query_graph + emit_diagnostic, renderers additionally get emit_file,
    providers additionally get http_get, parsers get read_file +
    emit_diagnostic + add_graph_node + add_graph_edge (but NOT query_graph,
    since they run during graph construction). The runtime checks the current
    export context before dispatching host function calls.
  """
}

decision D-004 "Auth Failure vs Offline Fallback" {
  settled true
  origin  project
  text    """
    Authentication failures (HTTP 401/403) MUST NOT fall back to cached
    binaries. Only network-level failures (DNS, TCP, TLS errors) trigger
    the offline cache fallback path. When a registry returns 401 or 403,
    the system MUST emit an E-level diagnostic identifying the registry
    and extension, and MUST NOT load the extension from cache. This treats
    auth rejection as an explicit server decision that the client must
    respect, even when a cached copy exists locally.
  """
}

decision D-022 "Gherkin as Extension-Declared Field" {
  settled true
  origin  project
  text    """
    gherkin is removed from the core grammar. It becomes a regular
    extension-declared field: @specforge/software declares a gherkin field
    with type string_list and file_reference=true on entity kinds that
    support BDD traceability. The supportsGherkin manifest flag is removed.
    The GherkinList AST type is removed — gherkin fields are parsed as
    standard StringList values. File existence was validated by
    validate_file_reference_paths on any field with file_reference=true;
    that behavior and file_reference are gone, and a field now declares
    the @specforge/checks:file check instead (ck_files_family, ADR 0336),
    which reports E016. verify REMAINS as a
    core grammar construct — it is a domain-agnostic traceability primitive.
  """
}

decision D-005 "Builtin Vocabulary, Generated Beside the Builtins" {
  settled true
  origin  project
  text    """
    The candidate vocabulary of the builtins (each kind keyword with every
    builtin that declares it, and each enhancement field with its one
    declaring extension) is generated by revendor from
    their pinned declarations, embedded by specforge-component, and handed to
    the registry build by the session. E024, W151 and W020 (and the reference
    I004 after plan 06) read it for their suggestion only. It decides no check's
    diagnostic and no ownership (ADR 0096): the compiler does not use it to
    validate, resolve, or register entity kinds. Beside the suggestions it decides
    one thing: whether a kind that an extension's route, context section, judged
    chain or facts input reaches is an unloaded optional peer's, which makes that
    part inert instead of E118, W112 or W021 (ADR 0331 D7, ADR 0369 D2). The
    vocabulary is a convenience layer for P8 compliance, not a domain
    knowledge backdoor (ADR 0122).
  """
}

decision D-017 "Finite Validation Pattern Kinds" {
  settled true
  origin  project
  text    """
    The declarative validation pattern kinds are graph-structural primitives
    that operate on the generic entity graph. They are finite because they
    correspond to fundamental graph operations (node degree, field presence,
    cycle detection, file existence). For validation logic that cannot be
    expressed as a declarative pattern, extensions MAY export Wasm validator
    functions using the emit_diagnostic host function. This is the explicit
    escape hatch. The pattern kind set MAY grow when new graph-structural
    primitives are identified, but MUST NOT grow to accommodate domain-specific
    checks — those belong in Wasm validators.
  """
}

decision D-026 "Multi-Event Consumption Pattern" {
  settled true
  origin  project
  text    """
    Behaviors that act as barrier-joins (waiting for multiple prerequisite
    events) are preferred over splitting into single-event consumers when
    the behavior's contract is semantically indivisible. The consumes list
    on a behavior declares a join barrier: ALL listed events MUST fire
    before the behavior executes. This is documented in each behavior's
    contract with an explicit barrier comment. Splitting would force
    intermediate state management and partial execution, which is more
    error-prone than a clean barrier-join.
  """
}

decision D-013 "Extension File Parsers" {
  settled true
  origin  project
  text    """
    A new contribution type "parser" allows extensions to read and parse
    domain-specific files referenced by file-reference fields, injecting
    structured data back into the graph. Parser contributions declare which
    file patterns they handle (e.g., *.feature, *.proto, *.yaml). When the
    compiler encounters a file-reference field matching a registered pattern,
    it dispatches to the owning extension's parser export.

    Parser host function permissions (per-call-site, extending ADR
    per_call_site_sandbox): read_file (scoped to spec root, read-only,
    restricted to patterns declared in manifest), emit_diagnostic,
    add_graph_node, add_graph_edge. Parsers do NOT get query_graph,
    emit_file, or http_get.

    query_graph is excluded because: (1) parsers extract and inject
    content — consistency checking is the validator's job (validators
    CAN query_graph); (2) other parsers may not have run yet, so
    query results are incomplete and ordering-dependent; (3) edges
    to not-yet-existing targets are valid — validation catches
    dangling edges after all parsers complete; (4) granting
    query_graph would blur the parser/validator boundary, encouraging
    parsers to embed domain-specific validation logic.

    The read_file host function enforces: path must be under spec root,
    path must match the parser's declared file patterns, read-only access
    only, maximum file size enforced by SandboxPolicy (default 1MB).

    Compilation pipeline placement: parsers run AFTER initial .spec parsing
    and reference resolution, but BEFORE validation. This lets parsed
    content participate in the full validation pass.
  """
}

decision D-090 "Progressive Design-by-Contract Adoption" {
  settled true
  origin  project
  text    """
    Apply DbC progressively: start with 8 critical-path behaviors identified
    by the spec audit, then expand in subsequent audit cycles. Priority
    criteria: (1) behaviors that gate pipeline phase transitions, (2) behaviors
    with complex pre-conditions that are easy to violate, (3) behaviors
    referenced by multiple invariants. Each audit cycle identifies the next
    batch of behaviors for DbC application based on these criteria.
  """
}

decision D-025 "Migration as Core Infrastructure" {
  settled true
  origin  project
  text    """
    Migration is core infrastructure, not domain knowledge. It operates on
    .spec file syntax structure (indentation, field ordering, block layout)
    before extensions are loaded. Migration transforms are purely syntactic —
    they do not inspect entity kinds, field semantics, or domain vocabulary.
    Extension-specific migration is delegated to extension authors via Wasm
    migration hooks (ExtensionDeclaration.migration_hook).

    This parallels the formatting precedent: core owns generic CST-level
    operations; extensions contribute domain-specific behavior via hooks.
    The P7 test — "does this require a compiler change when a new domain
    appears?" — is satisfied: adding @specforge/compliance requires zero
    changes to the migration engine.

    Migration types (FormatVersion, MigrationResult, MigrationBackup,
    MigrationSummary) are extracted to types/migration.spec to keep
    types/core.spec focused on fundamental compiler data shapes.

    See also: zero_entity_core_architecture ADR, formatting P7 justification
    in features/formatting.spec.
  """
}

decision D-008 "Diagnostic Code I016 Allocation" {
  settled true
  origin  project
  text    """
    Reallocate core schema-cache-not-found to I016 (first reserved slot). Follows non-contiguous pattern established by Wasm I013 and Migration W053.
  """
}

decision D-024 "Graph Protocol Version Management" {
  settled true
  origin  project
  text    """
    Schema version is derived automatically from registry state, against
    the schema the last export recorded. Major = something a reader of the
    recorded schema relies on was removed or reshaped; minor = something
    was added; patch = documentation or provenance changed (ADR 0067).
    First export without a cache = version 1.0.0. A requested version is
    admitted when its readers read the current schema (same major, not
    newer); there is no configured supported range.
  """
}

decision D-003 "Surface Contribution Model" {
  settled true
  origin  project
  text    """
    Extensions declare surface contributions in their manifest's surfaces
    field with three contribution types: commands[] (CLI), mcp_tools[]
    (MCP tools), mcp_resources[] (MCP resources). The compiler discovers
    contributions from manifests at startup (manifest-driven, no code
    execution). Wasm modules are loaded lazily on first invocation.

    Naming conventions: CLI exports use cmd__{id}, MCP exports use
    mcp__{name}. CLI commands are auto-promoted to MCP tools with the
    specforge.{ext_short}.{cmd_id} naming convention, giving agents
    automatic access to all extension CLI commands.

    Every surface export runs with no capability; a per-contribution
    sandbox override was dropped (ADR 0037): with nothing granted it had
    nothing to restrict.

    Phase 1 covers CLI commands, MCP tools, and MCP resources. LSP
    providers are deferred to Phase 2.

    P7 compliance: Surface contribution dispatch (register, validate, dispatch
    behaviors) is core infrastructure, not domain logic. This parallels
    call_extension_exports, the generic mechanism by which the core calls
    extension-provided Wasm exports without inspecting content. The content of each surface contribution is extension-defined;
    the dispatch mechanism is structural plumbing. Extracting dispatch into
    a separate extension would create a bootstrap paradox: the dispatch
    extension would need to be loaded before any other extension could
    register surfaces. Per the Terraform analogy: Terraform's provider
    loading is core, not a provider itself.
  """
}

decision D-099 "All Extensions Execute as wasip2 Components" {
  settled true
  origin  project
  text    """
    Every extension - builtin or third-party - is a WIT-typed wasip2
    component executed through a single wasmtime component engine
    (crates/runtime/component). The native tier is deleted; a gate test
    prevents its reintroduction. The guest boundary is the
    specforge:bridge world (call: name, export-name, input -> result).
    Every call gets a deterministic fuel budget and a wall-clock
    deadline, and every instance a memory limit (ADR 0037).
    The committed blobs (specforge_component::committed: the builtins
    under extensions/<name>/wasm/, embedded at compile time, the test
    fixtures under fixtures/<name>/wasm/, and each example project's own
    extension under examples/<name>/extension/wasm/) each carry an inputs.json
    fingerprint of the sources they were built from; `cargo run -p xtask
    --bin revendor` rebuilds and re-vendors every one of them
    (wasm32-wasip2 target) in a staged workspace that holds each crate and
    its path crates at their repo-relative paths, with the cargo home
    remapped, so a blob's bytes depend only on its sources and the
    toolchain, never on the checkout path. `build-blobs --check` fails
    when a committed blob drifts from its sources, and `--verify` rebuilds
    every blob and fails when one differs.
  """
}

decision D-012 "A Project Example's Own Extension Is a Committed Blob" {
  settled true
  origin  project
  text    """
    Each example project's own extension is listed in specforge_component::committed::EXAMPLES and vendored at examples/<name>/extension/wasm/ beside its inputs.json; the blob stage drops a staged crate's own [workspace] and [profile] tables; the gate expands prefix/*/rest patterns once, so the extension is rustfmt- and wasm-clippy-checked and fed to the builtins check.
  """
}

decision D-094 "Tests Belong to @specforge/testing and One Extension per Runner" {
  settled true
  origin  project
  text    """
    A runner-agnostic @specforge/testing builtin owns the test vocabulary:
    testability of kinds and their verify kinds (contributed to other
    extensions' kinds through enhancements), W004/W009, coverage, and the
    normalized result format. One extension per test runner
    (@specforge/cargo-test, @specforge/vitest first) declares a collector:
    how to detect the runner, one command to run it, and a pure export that
    maps its report to entity results. The compiler still never executes
    anything; `specforge collect` runs a runner extension's declared
    command after per-project consent. Tests link to entities by
    annotation in the runner's own idiom; the spec-side `tests` field is
    retired.
  """
}

decision D-019 "Split Formal Analysis from Software Extension" {
  settled true
  origin  project
  text    """
    Extract all formal analysis content from @specforge/software into a
    new @specforge/formal extension. The formal extension operates through
    entity_enhancements on @specforge/software entities for inline
    condition blocks (requires/ensures/maintains). This follows the
    "extensions over built-ins" principle (P5) and keeps the software
    extension focused on core entity registration. Users opt into formal
    analysis by installing @specforge/formal and setting warning_level=strict.
  """
}

decision D-018 "Expand Formal Extension to 4 Entity Kinds" {
  settled true
  origin  project
  text    """
    Expand @specforge/formal to 3 entity kinds: property, axiom,
    protocol. All 3 are testable=false, supports_verify=false.
    Add 3 new edge types: AssumedBy (invariant → axiom), Satisfies
    (behavior → property), FollowsProtocol (event → protocol).
    PropertyDependsOn (property → invariant) added for property-invariant
    dependency tracking.

    Remove the arbitrary 20-entity budget cap. Quality and readability
    over artificial limits. New entity total: 2 structural + 20 domain
    = 22.

    Renumber formal diagnostic codes from W042-W044 to W058-W060 to
    resolve collision with @specforge/product. New diagnostic range:
    W058-W068 for the formal extension.

    Later (ADR 0040): the range was not kept. W059-W060 were removed
    with the condition entity kind, W058 (feature coverage mismatch)
    was trimmed for want of a sound algorithm, and W060-W062 are core
    codes now. Formal's entity validation uses W123-W136.
  """
}

decision D-020 "Add Refinement and Process Entity Kinds" {
  settled true
  origin  project
  text    """
    Add refinement and process as entity kinds 4 and 5. Both testable=false,
    supports_verify=false. 4 new edge types: RefinesTo, RefinementChainLink,
    ParticipatesIn, ProcessComposition. 2 new error codes: E041 (refinement
    chain cycle), E042 (process composition cycle). 6 new warnings: W069-W074,
    since renumbered W131-W136 (refinement W131-W133, process W134-W136).
    Entity enhancements: behavior gets refinement field, event gets process field.
    Total: 5 entity kinds, 8 edge types, 23 project entities.
  """
}

decision D-021 "Rename Formal Methods Terminology" {
  settled true
  origin  project
  text    """
    Rename all formal methods vocabulary to accurately describe the
    structural analysis being performed:
    - "Design by Contract" -> "Structured Conditions"
    - "B-Method Refinement" -> "Specification Layering"
    - "CSP Event Flow Analysis" -> "Event Graph Linting"
    - "Verification Obligations" -> "Coverage Tracking Items"
    - "FormalityLevel" -> "SpecificationDepthLevel"
    - "Deadlock detection" -> "Unmitigated cycle detection"
    - "Livelock risk" -> "Unmitigated retry cycle"
    - "Starvation risk" -> "Asymmetric connectivity warning"
    - E033 downgraded to W058 (structural check, not semantic verification);
      W058 was later trimmed (ADR 0040)
    - E034 now checks for mitigations before firing; the mitigation is a
      declared sync on a member of the cycle (ADR 0040)
  """
}

decision D-010 "Drop-Based Test Guard" {
  settled true
  origin  project
  text    """
    Use a Drop-based TestGuard. On creation, record entity ID and test name.
    On drop, check std::thread::panicking() — false means pass, true means
    fail, inverted under #[should_panic] (the attribute reads the
    expectation). Write results to target/specforge/ via atexit handler.
  """
}

decision D-009 "Double Underscore Entity Separator" {
  settled true
  origin  project
  text    """
    Use double underscore (__) as the separator between entity ID and
    description slug: {entity_id}__{description_slug}. Example:
    validate_input__rejects_empty_name. The entity ID is everything
    before the first __.
  """
}

decision D-089 "Phased Rust Delivery" {
  settled true
  origin  project
  text    """
    Three-phase delivery: Phase 1 ships test collection with
    convention-based mapping (zero Rust dependencies). Phase 2 ships the
    proc macro crate for explicit annotation. Phase 3 adds advanced
    features (watch, LSP, suspect links).
  """
}

decision D-056 "Nine Entity Kinds" {
  settled true
  origin  project
  text    """
    Register 9 entity kinds: feature, journey, deliverable, milestone, module,
    term, persona, channel, release. All have testable=false — they are planning
    constructs tested indirectly through behavior chains from domain extensions.
    Connected by 20 edge types forming a complete traceability graph from user
    need to shipped artifact. Release was added in v1.1 to coordinate
    multi-deliverable shipping with ReleaseIncludesDeliverable,
    ReleaseCompletesMilestone and ReleaseDependsOn edges.
  """
}

decision D-063 "Persona and Channel as First-Class Entities" {
  settled true
  origin  project
  text    """
    Declare persona and channel as first-class entity kinds in
    @specforge/product. Journeys reference them via JourneyTargetsPersona and
    JourneyUsesChannel edges. Both are testable=false. Unreferenced detection
    via I046 (persona) and I047 (channel) at info level to support
    incremental adoption.
  """
}

decision D-046 "Feature as Domain-Neutral Product Concept" {
  settled true
  origin  project
  text    """
    Feature entities use domain-neutral problem/solution framing only.
    No domain-specific fields in the base feature type. Domain extensions
    reference features via their own fields and peer dependencies, creating
    cross-extension edges. Product features work standalone.
  """
}

decision D-030 "All Product Entities Non-Testable" {
  settled true
  origin  project
  text    """
    All 9 product entity kinds declare testable=false in the manifest.
    Product entities are tested indirectly through the behavior chains
    from domain extensions that reference features. Direct testability
    would create false obligations on
    planning constructs that have no executable contract.
  """
}

decision D-083 "TermReferencesRelatedTerm Edge Restricted to Term-to-Term" {
  settled true
  origin  project
  text    """
    Only term-to-term references in see_also produce TermReferencesRelatedTerm edges.
    Cross-kind references are rejected (E022): see_also accepts only terms.
    Terms are navigation aids for
    vocabulary consistency, not dependency-tracking nodes. Cross-kind
    edges from terms would pollute dependency analysis, cycle detection,
    and graph traversal with non-structural relationships.
  """
}

decision D-028 "Acceptance Criteria as Free-Form Prose" {
  settled true
  origin  project
  text    """
    The acceptance field on features is free-form string[] — each entry
    is a prose acceptance criterion with no enforced structure. Machine
    verification of feature completeness belongs to behavior verify
    statements, not feature acceptance criteria. This preserves domain
    neutrality: any domain can express acceptance in its own vocabulary.
  """
}

decision D-048 "Interaction Model as Metadata" {
  settled true
  origin  project
  text    """
    The interaction_model field is metadata for tooling and AI agents, not
    a flow-step validation constraint. interaction_model is an open
    identifier with suggested words (ADR 0279), and the compiler does
    NOT enforce that journey flow steps are compatible with the
    referenced channel's interaction model. Flow-step compatibility is
    domain-specific and belongs to domain extensions or agent reasoning.
  """
}

decision D-035 "Milestone Completion via Feature Status" {
  settled true
  origin  project
  text    """
    Milestone completion uses the FeatureStatus field: done_count / total_features.
    Features with status=done are complete. No cross-extension edges are needed.
  """
}
