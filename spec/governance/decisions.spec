// Core Architecture Decision Records
//
// Decisions about the compiler engine, extension model, and infrastructure.
// Extension-specific decisions live in their respective extension directories
// under spec/extensions/.

use invariants/core
use invariants/extensions
use invariants/wasm

decision tree_sitter_for_parsing "Tree-sitter for Parsing" {
  status   accepted
  date     2026-03-01

  context """
    SpecForge needs a parser that supports error recovery (partial ASTs on
    syntax errors), incremental re-parsing for watch mode, and source span
    tracking for diagnostics and LSP features. Options considered: hand-written
    recursive descent, nom/pest/chumsky combinators, or Tree-sitter.
  """

  decision """
    Use Tree-sitter for the .spec grammar. It provides error recovery,
    incremental parsing, and a mature ecosystem with editor integrations.
    No hand-written recursive descent fallback.
  """

  consequences [
    "Error recovery out of the box — partial ASTs on syntax errors",
    "Incremental reparsing for watch mode and LSP",
    "Editor highlighting via existing Tree-sitter integrations",
    "Grammar development requires learning Tree-sitter DSL",
    "Build dependency on Tree-sitter CLI for grammar generation",
  ]

  invariants [multi_error_collection, incremental_correctness]
}

decision adr_multi_error_collection "Multi-Error Collection" {
  status   accepted
  date     2026-03-01

  context """
    Compilers that halt on the first error force users to fix issues one by
    one. This is slow for projects with hundreds of .spec files. The compiler
    needs to collect all diagnostics in a single pass.
  """

  decision """
    The compiler collects all errors, warnings, and info diagnostics during
    a compilation pass and emits them together. No fail-fast behavior.
    Diagnostics are sorted by (file_path, line, column, code) — source
    order within each file, not grouped by severity. This matches the
    rustc convention where developers see diagnostics in the order they
    appear in the source.
  """

  consequences [
    "Users see all issues at once — faster fix cycles",
    "Diagnostic collection requires careful error handling to avoid cascading",
    "Memory proportional to diagnostic count, not file count",
    "Deterministic output order regardless of processing order",
  ]

  invariants [multi_error_collection, diagnostic_determinism]
}

decision string_interning_with_lasso "String Interning with lasso" {
  status   accepted
  date     2026-03-01

  context """
    Entity IDs and file paths are compared frequently during resolution and
    validation. Character-by-character comparison is O(n) per comparison.
    Interning reduces this to O(1) pointer comparison.
  """

  decision """
    Use the lasso crate for string interning. Intern all entity IDs, file
    paths, and frequently compared strings. Use interned keys for all
    comparison and hashing operations.
  """

  consequences [
    "O(1) string comparison for entity IDs",
    "Reduced memory for duplicate strings across files",
    "Additional complexity for interning lifecycle management",
    "All string comparison code must use interned keys, not raw strings",
  ]

  invariants [string_interning_consistency]
}

decision mutable_in_memory_graph "Mutable In-Memory Graph" {
  status   accepted
  date     2026-03-01

  context """
    Watch mode and LSP require incrementally updating the graph when files
    change. An immutable graph would require full rebuilds on every change.
    A mutable graph allows adding/removing nodes and edges in place.
  """

  decision """
    The in-memory graph is mutable. It supports adding/removing nodes and
    edges. Incremental compilation mutates the graph in place, removing
    stale entries and inserting new ones from re-parsed files.
  """

  consequences [
    "Watch mode and LSP can update incrementally — <100ms latency",
    "Graph mutation code must maintain consistency (no dangling edges)",
    "No persistent immutable snapshots — debugging requires graph dumps",
    "Concurrent access requires synchronization in LSP mode",
  ]

  invariants [incremental_correctness]
}

decision snapshot_testing_with_insta "Snapshot Testing with insta" {
  status   accepted
  date     2026-03-01

  context """
    Compiler output (diagnostics, rendered markdown, JSON graph output) must be
    tested against expected output. Manual assertion writing is tedious and
    brittle. Snapshot testing captures output and detects unexpected changes.
  """

  decision """
    Use the insta crate for snapshot testing. Diagnostic output and rendered
    markdown are tested via inline and file-based
    snapshots. cargo insta review for interactive approval.
  """

  consequences [
    "Fast test authoring — capture output, approve once",
    "Automatic regression detection on output changes",
    "Snapshot files add to repository size",
    "Format changes require mass snapshot updates",
  ]
}

decision three_layer_traceability_model "Three-Layer Traceability Model" {
  status   accepted
  date     2026-03-02

  context """
    Spec coverage based solely on counting verify declarations does not
    prove that tests exist, run, or pass. Teams need end-to-end proof
    that every testable entity has been implemented and tested.
  """

  decision """
    Three-layer traceability: intent (verify/gherkin declarations in .spec
    files), linkage (tests field pointing to real test files), and proof
    (specforge-report.json consumed from test runner extensions with pass/fail
    results). specforge trace validates all three layers.
  """

  consequences [
    "Agents can close the full loop: read spec → generate code → fill tests field → validate",
    "Coverage measured at four levels within the three-layer model: declared, linked, executed, passing",
    "Requires test runner extensions to produce specforge-report.json",
    "Additional complexity in the coverage pipeline",
  ]

  invariants [traceability_chain_integrity]
}

decision wasm_extism_extension_runtime "Wasm/Extism Unified Extension Runtime" {
  status   accepted
  date     2026-03-03

  context """
    The contribution-based extension model requires a unified runtime for
    all extension contributions — entity definitions, validators, renderers,
    and providers. The runtime must work uniformly across all SpecForge
    surfaces — CLI, LSP, and MCP. Subprocess JSON-RPC was considered but
    degrades for long-running surfaces: LSP would need to manage child
    process lifecycles per edit, and MCP would face the same overhead.
    Embedded scripting (Lua, Rhai, Starlark) was evaluated but tree-sitter
    grammars compile to C at build time, making parser-level extension
    impossible regardless of scripting engine.
  """

  decision """
    Use WebAssembly via Extism as the unified extension runtime for all
    contribution types. Extensions expose host functions (specforge.query_graph,
    specforge.emit_diagnostic) — Wasm modules never touch the OS directly.
    Renderers use specforge.emit_file(path, content) instead of raw
    filesystem access, with the host validating paths and enforcing
    sandboxing. Providers use specforge.http_get for external
    service validation. Universal .wasm binaries are distributed via npm,
    GitHub Releases, or OCI registries. AOT compilation caches .wasm modules
    for CLI cold start; LSP and MCP keep warm engine instances in-process.
  """

  consequences [
    "Hardware-enforced sandboxing — extensions cannot access filesystem or network without host consent",
    "Multi-language extension authoring — Rust, Go, TypeScript, Python via Wasm compilation",
    "Universal .wasm binary distribution — same artifact runs on all platforms",
    "Single runtime covers all contribution types and all three surfaces uniformly",
    "+5MB binary size increase from Extism/Wasmtime runtime",
    "Extension authors need Wasm toolchain (cargo-component, tinygo, javy, etc.)",
    "10-50ms cold start per extension — mitigated by AOT caching for CLI, warm engines for LSP/MCP",
  ]

  invariants [reference_resolution_completeness]
}

decision extension_peer_dependencies "Extension Peer Dependencies" {
  status   accepted
  date     2026-03-03

  context """
    Extensions naturally form dependency chains: @specforge/product entities
    reference core entities (capability bundles feature), governance entities
    reference core entities (decision protects invariant), and third-party
    extensions may depend on official extension entities. Today, cross-module
    references use implicit soft resolution — if the target module is not
    installed, the resolver emits I004 (info) instead of E001 (error). This
    is insufficient for Wasm extensions that explicitly declare entity types
    targeting another extension's entities. A third-party @myorg/epic-tracker
    defining an epic entity with a stories field referencing feature must
    fail hard if @specforge/product is not installed, not silently degrade.
  """

  decision """
    Wasm extension manifests declare peer_dependencies with semver ranges. The
    host validates all peer dependencies before loading and topologically
    sorts extensions for initialization order (core first, then official extensions,
    then third-party). All extensions share the same in-process graph via
    specforge.query_graph host function — no serialization between extensions.
    When a manifest declares a peer dependency that is not installed, the
    host emits a hard error. The existing I004 soft resolution remains for
    implicit cross-module references (fields without a manifest declaration).
  """

  consequences [
    "Explicit dependency chains — extension authors declare what they need upfront",
    "Hard error on missing peer dependency vs soft I004 for implicit references",
    "Topological loading order prevents extensions from querying entities not yet registered",
    "Zero serialization cost — all Wasm extensions share the graph via host function calls",
    "Manifest schema must include peer_dependencies with semver range validation",
    "Circular extension dependencies are detected and rejected at load time",
  ]

  invariants [reference_resolution_completeness]
}

decision config_driven_kind_registry "Config-Driven Kind Registry" {
  status   accepted
  date     2026-03-03

  context """
    Wasm extensions declare entity kinds in their manifests, and can add
    graph node instances at runtime via specforge.add_graph_node.
    Currently there is no conflict prevention: two extensions can declare
    the same kind name, an extension can shadow a structural keyword like
    spec, and define blocks can collide with extension kinds. The existing
    FieldEnhancementRegistry solved the analogous problem for field-level
    conflicts but no equivalent exists for entity kind names.
  """

  decision """
    Introduce a 4-layer entity kind conflict prevention system:
    Layer 1 — Registration guard: host function rejects structural
    keywords (spec, use, define, ref, verify, gherkin, scenario, given,
    when, then, true, false) and DSL syntax words
    at manifest registration time.
    Layer 2 — KindRegistry: central registry detects duplicate names
    across extensions and define blocks, emitting E022.
    Layer 3 — Config resolution: specforge.json entity_kind_policy
    (error/priority/namespace) and entity_kinds overrides.
    Layer 4 — Inline qualification: parser recognizes @extension/kind
    syntax for explicit disambiguation.
  """

  consequences [
    "No silent shadowing of structural or define-block entity kinds",
    "Deterministic resolution via config-driven policies",
    "Mirrors the enhancement_policy pattern for consistency",
    "Adds entity_kind_policy and entity_kinds to specforge.json surface",
    "Parser must handle @extension/kind qualified entity keywords",
  ]

  invariants [entity_kind_uniqueness]
}

decision entity_enhancement_model "Entity Enhancement Model" {
  status   accepted
  date     2026-03-03

  context """
    Extensions need to add fields and edges to existing entity types (e.g., a
    hexagonal architecture extension adding a layer field to behavior). Without
    entity enhancement, extensions are limited to creating new entity types —
    they cannot annotate or extend entities from other extensions. This blocks
    a significant class of architectural extensions.
  """

  decision """
    Extension manifests declare entity enhancements with target entity kind,
    field name, field type, and optional edge mappings. The compiler builds
    a FieldRegistry combining extension-defined and enhanced fields, threads
    it through the resolve/graph-build/validate pipeline. Conflicts between
    extensions are resolved via configurable policies (error/priority/namespace).
    Core structural field shadowing always produces E018 regardless of policy.
    specforge doctor provides visibility into all enhancements.
  """

  consequences [
    "Extensions can annotate existing entities without forking the grammar",
    "FieldRegistry becomes the single source of truth for field definitions",
    "Conflict policies add configuration surface that must be documented",
    "E018 hard error prevents accidental structural field shadowing",
    "specforge doctor provides actionable conflict resolution",
    "Enhancement conflicts detected at manifest load time (startup), not lazily during validation",
  ]

  invariants [enhancement_field_uniqueness, enhancement_builtin_precedence]
}

decision aot_compilation_strategy "AOT Compilation Strategy" {
  status   accepted
  date     2026-03-03

  context """
    Wasm modules can be executed via JIT (compile-on-first-call) or AOT
    (compile-ahead-of-time and cache). JIT has zero upfront cost but slower
    first-call latency. AOT has an upfront compilation cost but subsequent
    loads are fast. For a batch compiler like SpecForge where cold start
    matters (CLI, CI), AOT is preferred.
  """

  decision """
    AOT compilation with content-addressed caching. The .wasm binary is
    hashed with SHA256 and the compiled artifact is stored in
    .specforge/cache/ using the hash as the filename. The platform triple
    is included in the cache key to prevent cross-platform misuse. On
    cache hit, the pre-compiled artifact is loaded directly. Cache entries
    are self-healing: corruption is detected by re-hashing and corrupted
    entries are evicted and recompiled automatically.
  """

  consequences [
    "CLI cold start <50ms per extension with cache hit",
    "First-time compilation takes 100-500ms per extension",
    "Cache directory grows proportionally to installed extensions",
    "Platform migration (e.g., x86 to ARM) invalidates entire cache",
    "Self-healing cache prevents silent corruption from causing failures",
  ]

  invariants [aot_cache_integrity, extension_load_order_determinism]
}

decision contribution_based_extension_model "Contribution-Based Extension Model" {
  status   accepted
  date     2026-03-03

  context """
    Extensions need a structured manifest format with per-entity metadata
    such as testable flags, field type definitions, and structured
    validation rule patterns. A 10-expert panel (RES-23) unanimously
    recommended a contribution-based model with structured manifest
    objects for richer extension metadata.
  """

  decision """
    Structured manifest with typed objects: entity_kinds ManifestEntityKind[],
    validation_rules ValidationRulePattern[], edge_types ManifestEdgeType[].
    Each extension declares a contributes key listing what it provides:
    entities, validators, renderers, providers. The compiler routes to
    namespaced Wasm exports based on contributions. Per-call-site host
    function permissions enforce least-privilege for each export.
  """

  consequences [
    "Single extension = full integration — no more splitting @specforge/jira into three extensions",
    "Contribution-level indexing enables fine-grained enable/disable via config",
    "Lower barrier to entry — one extension to author, test, and publish",
    "Per-call-site permissions maintain defense-in-depth despite single-extension model",
  ]

  invariants [reference_resolution_completeness]
}

decision adr_extension_source_resolution "Extension Source Resolution" {
  status   accepted
  date     2026-03-03

  context """
    Extensions need to be installable from multiple sources for different
    development workflows. Local development requires loading from
    filesystem paths without publishing. Teams need to share extensions
    via git before a registry is available. Production deployments
    require pinned registry versions.
  """

  decision """
    Three extension source types: registry (@scope/name@version for npm),
    local (./path for filesystem), and git (git:url#ref for repositories).
    The specifier format supports both string shorthand ("@specforge/product@^1.0")
    and object form ({ source: "local", path: "./my-extension" }) in
    specforge.json. Resolution produces a manifest + .wasm binary pair.
  """

  consequences [
    "Local development without publishing — ./path resolves to filesystem",
    "Git sharing before registry availability — git:url#ref for early adoption",
    "Reproducible production builds via pinned registry versions",
    "Three source types to implement and test",
    "Object form provides escape hatch for complex configurations",
  ]

  invariants [reference_resolution_completeness]
}

decision extension_version_pinning "Extension Version Pinning" {
  status   accepted
  date     2026-03-03

  context """
    Without version pinning, specforge add at different times produces
    different extension versions. This makes builds non-reproducible and
    leads to inconsistent behavior across team members and CI. The problem
    is amplified by the Wasm binary format — unlike source code, binary
    differences are opaque and hard to debug.
  """

  decision """
    specforge.lock pins exact resolved versions with SHA256 wasm_hash for
    each installed extension. The lock file format is deterministic — same
    inputs always produce byte-identical output. Integrity verification
    compares .wasm binary hashes against lock entries. specforge update
    explicitly bumps versions and refreshes the lock file.
  """

  consequences [
    "Reproducible builds — same lock file produces identical compilation behavior",
    "Tamper detection via SHA256 integrity verification of .wasm binaries",
    "Explicit update workflow prevents accidental version drift",
    "Lock file must be committed to version control",
    "First install on a clean checkout requires network access to resolve",
  ]

  invariants [reference_resolution_completeness]
}

decision specforge_is_specification_language "SpecForge is a Specification Language" {
  status   accepted
  date     2026-03-04

  context """
    The primary consumer of SpecForge specs is AI agents that read
    specifications and generate code themselves. SpecForge provides
    structured context; agents produce output. These are fundamentally
    different jobs — conflating them creates friction: drift between
    generated and actual code, ownership conflicts over generated files,
    and a generator that must keep up with every target language's idioms.
  """

  decision """
    SpecForge is a specification language that helps AI agents generate
    better code. SpecForge provides structured context — AI agents generate
    code. The `verify` keyword declares test intent inside testable entity
    blocks. The renderer contribution type in Wasm extensions produces
    non-code outputs only (reports, dashboards, traceability matrices).
    The `collect` command and all spec validation/coverage features
    provide test traceability.
  """

  consequences [
    "Specs are pure declarations — no file paths coupling specs to implementation",
    "AI agents have clear ownership of code generation from specs",
    "No drift between generated code and specs — agents regenerate as needed",
    "Renderer contribution type produces non-code outputs only",
    "collect command and coverage pipeline provide test traceability",
  ]

  invariants [traceability_chain_integrity]
}

decision zero_entity_core_architecture "Zero-Entity Core Architecture" {
  status   accepted
  date     2026-03-04

  context """
    SpecForge's 14 domain entity kinds (6 software + 5 product + 3 governance)
    plus 2 structural keywords (spec, ref) — 16 entity-bearing keywords total — limit the tool to
    software engineering. The vision of being the structured context standard
    for ALL AI agents requires supporting any domain: UI design (atomic design),
    compliance (regulation, control, evidence), data pipelines, API design,
    infrastructure, and domains not yet imagined. Hardcoding entity types into
    the compiler core prevents this expansion.
  """

  decision """
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

  consequences [
    "SpecForge becomes domain-agnostic — any industry can use it with appropriate extensions",
    "Projects declare domain extensions in specforge.json (@specforge/software + @specforge/product + @specforge/governance)",
    "Extension manifest declares entity_kinds, edge_types, validation_rules, and testability",
    "Parser already supports generic_entity_block — no grammar changes needed",
    "KindRegistry, FieldRegistry, WasmEntityKind, Custom(String) already exist — ~60% ready",
    "Validation rules become declarative patterns interpreted by core, not hardcoded passes",
    "Community can create domain extensions without modifying the compiler",
    "Estimated 5-7 weeks implementation in 7 phases",
  ]

  invariants [reference_resolution_completeness]
}

decision per_call_site_sandbox "Per-Call-Site Sandbox" {
  status   accepted
  date     2026-03-03

  context """
    The contribution-based model means one extension can contribute entities,
    rendering, and provider validation. A per-extension sandbox policy is
    too coarse — an extension's validator should not have emit_file access
    just because the same extension also contributes a renderer. The
    principle of least privilege requires finer-grained permissions.
  """

  decision """
    Host function permissions are enforced per export call site, not per
    extension. Each contribution type has a fixed permission set: validators
    get query_graph + emit_diagnostic, renderers additionally get emit_file,
    providers additionally get http_get. The runtime checks the current
    export context before dispatching host function calls.
  """

  consequences [
    "Least-privilege per operation — validator cannot write files even if same extension has a renderer",
    "Defense in depth — compromised validator export cannot escalate to filesystem access",
    "Runtime must track which export is currently executing per call",
    "Permission model is static and predictable — no runtime configuration needed",
  ]

  invariants [wasm_sandbox_integrity]
}

decision auth_failure_vs_offline_fallback "Auth Failure vs Offline Fallback" {
  status   accepted
  date     2026-03-07

  context """
    Registry authentication failures (401/403) and network-level failures
    (DNS resolution, TCP timeout, TLS handshake) are distinct error classes.
    The offline_first_extension_resolution invariant guarantees cached
    extensions work without network access, but does not specify behavior
    when the network IS reachable and the server actively rejects credentials.
    Using cached binaries after an auth rejection could allow use of
    extensions the user is no longer authorized to access (revoked license,
    removed from team, expired trial).
  """

  decision """
    Authentication failures (HTTP 401/403) MUST NOT fall back to cached
    binaries. Only network-level failures (DNS, TCP, TLS errors) trigger
    the offline cache fallback path. When a registry returns 401 or 403,
    the system MUST emit an E-level diagnostic identifying the registry
    and extension, and MUST NOT load the extension from cache. This treats
    auth rejection as an explicit server decision that the client must
    respect, even when a cached copy exists locally.
  """

  consequences [
    "Security boundary: revoked credentials immediately prevent extension use",
    "Inconvenience when registry has transient auth issues — no silent fallback",
    "Clear separation: network failures are infrastructure, auth failures are access control",
    "Error messages must distinguish auth failure from network failure for actionable guidance",
  ]

  invariants [offline_first_extension_resolution, credential_secrecy]
}

