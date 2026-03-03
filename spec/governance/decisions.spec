// Architecture Decision Records

use invariants/core
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
    Diagnostics are sorted by severity then by file position.
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
    Compiler output (diagnostics, rendered markdown, generated code) must be
    tested against expected output. Manual assertion writing is tedious and
    brittle. Snapshot testing captures output and detects unexpected changes.
  """

  decision """
    Use the insta crate for snapshot testing. Diagnostic output, rendered
    markdown, and generated code stubs are tested via inline and file-based
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
    Three-layer traceability: intent (verify/scenario declarations in .spec
    files), linkage (tests field pointing to real test files), and proof
    (specforge-report.json consumed from test runner plugins with pass/fail
    results). specforge trace validates all three layers.
  """

  consequences [
    "Agents can close the full loop: read spec → generate code → fill tests field → validate",
    "Coverage gating at four levels: declared, linked, executed, passing",
    "Requires test runner plugins to produce specforge-report.json",
    "Additional complexity in the coverage pipeline",
  ]

  invariants [traceability_chain_integrity]
}

decision dual_verify_scenario_syntax "Dual Verify/Scenario Syntax" {
  status   accepted
  date     2026-03-02

  context """
    Verify is a quick one-liner for declaring code-level test expectations.
    Scenario is a structured given/when/then block for acceptance criteria.
    Both serve different purposes and different entity types need different
    constructs.
  """

  decision """
    Both verify and scenario coexist. An entity × syntax matrix determines
    which entities support which constructs: all 5 testable entities accept
    verify, only behavior and capability accept scenario. verify e2e is
    deprecated in favor of scenario.
  """

  consequences [
    "Flexibility: developers choose the right tool for each test declaration",
    "Slightly larger grammar surface to maintain",
    "Clear migration path from verify e2e to scenario",
    "Entity × syntax matrix must be enforced by the validator",
  ]

  invariants [testable_entity_classification]
}

decision scenario_as_agent_prompt "Scenario as Agent Prompt" {
  status   accepted
  date     2026-03-02

  context """
    SpecForge's primary consumer is AI agents that read specs and generate
    code. Scenario blocks need to serve as structured instructions that
    agents can follow to generate tests, not as human-facing Gherkin
    documentation.
  """

  decision """
    Scenario steps serve as structured agent prompts for test generation.
    Agents read scenario → generate test code → fill the tests field →
    specforge trace validates the chain. Scenario syntax is kept minimal
    (given/when/then only, no and/but/examples/background).
  """

  consequences [
    "Agents have clear, structured acceptance criteria to implement",
    "Scenario blocks are actionable, not just documentation",
    "Minimal syntax reduces ambiguity for AI consumers",
    "Human readability is maintained through the given/when/then structure",
  ]

  invariants [traceability_chain_integrity]
}

decision terraform_style_extension_model "Terraform-Style Extension Model" {
  status   superseded
  date     2026-03-01

  context """
    Not every project needs all 16 entity types. A monolithic compiler would
    be bloated for simple projects. An extension model allows progressive
    adoption. Terraform's model (small core + providers) is well-understood.
  """

  decision """
    Three extension mechanisms: plugins (entity model), providers (ref
    validation), and generators (output formats). Core has 8 entities.
    @specforge/product adds 5, @specforge/governance adds 3. Plugins are
    installed via specforge add and declared in specforge.json.
  """

  consequences [
    "Small core for simple projects — 8 entities usable immediately",
    "Progressive adoption — add plugins as projects grow",
    "Cross-plugin references use soft resolution (I004 if plugin missing)",
    "Plugin ecosystem requires stable API and manifest format",
    "Three extension mechanisms to document and maintain",
    "SUPERSEDED by contribution_based_extension_model — PluginKind taxonomy replaced by contributes manifest key",
  ]

  invariants [reference_resolution_completeness]
}

decision wasm_extism_package_runtime "Wasm/Extism Unified Package Runtime" {
  status   accepted
  date     2026-03-03

  context """
    The contribution-based extension model requires a unified runtime for
    all package contributions — entity definitions, validators, generators,
    and providers. The runtime must work uniformly across all SpecForge
    surfaces — CLI, LSP, and MCP. Subprocess JSON-RPC was considered but
    degrades for long-running surfaces: LSP would need to manage child
    process lifecycles per edit, and MCP would face the same overhead.
    Embedded scripting (Lua, Rhai, Starlark) was evaluated but tree-sitter
    grammars compile to C at build time, making parser-level extension
    impossible regardless of scripting engine.
  """

  decision """
    Use WebAssembly via Extism as the unified package runtime for all
    contribution types. Packages expose host functions (specforge.query_graph,
    specforge.emit_diagnostic, specforge.register_entity) — Wasm modules
    never touch the OS directly. Generators use specforge.emit_file(path,
    content) instead of raw filesystem access, with the host validating paths
    and enforcing sandboxing. Providers use specforge.http_get for external
    service validation. Universal .wasm binaries are distributed via npm,
    GitHub Releases, or OCI registries. AOT compilation caches .wasm modules
    for CLI cold start; LSP and MCP keep warm engine instances in-process.
  """

  consequences [
    "Hardware-enforced sandboxing — packages cannot access filesystem or network without host consent",
    "Multi-language package authoring — Rust, Go, TypeScript, Python via Wasm compilation",
    "Universal .wasm binary distribution — same artifact runs on all platforms",
    "Single runtime covers all contribution types and all three surfaces uniformly",
    "+5MB binary size increase from Extism/Wasmtime runtime",
    "Package authors need Wasm toolchain (cargo-component, tinygo, javy, etc.)",
    "10-50ms cold start per package — mitigated by AOT caching for CLI, warm engines for LSP/MCP",
  ]

  invariants [reference_resolution_completeness]
}

decision package_peer_dependencies "Package Peer Dependencies" {
  status   accepted
  date     2026-03-03

  context """
    Packages naturally form dependency chains: @specforge/product entities
    reference core entities (capability bundles feature), governance entities
    reference core entities (decision protects invariant), and third-party
    packages may depend on official package entities. Today, cross-module
    references use implicit soft resolution — if the target module is not
    installed, the resolver emits I004 (info) instead of E001 (error). This
    is insufficient for Wasm packages that explicitly declare entity types
    targeting another package's entities. A third-party @myorg/epic-tracker
    defining an epic entity with a stories field referencing feature must
    fail hard if @specforge/product is not installed, not silently degrade.
  """

  decision """
    Wasm package manifests declare peer_dependencies with semver ranges. The
    host validates all peer dependencies before loading and topologically
    sorts packages for initialization order (core first, then official packages,
    then third-party). All packages share the same in-process graph via
    specforge.query_graph host function — no serialization between packages.
    When a manifest declares a peer dependency that is not installed, the
    host emits a hard error. The existing I004 soft resolution remains for
    implicit cross-module references (fields without a manifest declaration).
  """

  consequences [
    "Explicit dependency chains — package authors declare what they need upfront",
    "Hard error on missing peer dependency vs soft I004 for implicit references",
    "Topological loading order prevents packages from querying entities not yet registered",
    "Zero serialization cost — all Wasm packages share the graph via host function calls",
    "Manifest schema must include peer_dependencies with semver range validation",
    "Circular package dependencies are detected and rejected at load time",
  ]

  invariants [reference_resolution_completeness]
}

decision config_driven_kind_registry "Config-Driven Kind Registry" {
  status   accepted
  date     2026-03-03

  context """
    Wasm plugins register new entity kinds via specforge.register_entity
    during initialization. Currently there is no conflict prevention: two
    plugins can register the same kind name, a plugin can shadow a built-in
    keyword like behavior, and define blocks can collide with plugin kinds.
    The existing FieldEnhancementRegistry solved the analogous problem for
    field-level conflicts but no equivalent exists for entity kind names.
  """

  decision """
    Introduce a 4-layer entity kind conflict prevention system:
    Layer 1 — Registration guard: host function rejects 16 built-in
    keywords and DSL syntax words at register_entity call time.
    Layer 2 — KindRegistry: central registry detects duplicate names
    across plugins and define blocks, emitting E022.
    Layer 3 — Config resolution: specforge.json entity_kind_policy
    (error/priority/namespace) and entity_kinds overrides.
    Layer 4 — Inline qualification: parser recognizes @plugin/kind
    syntax for explicit disambiguation.
  """

  consequences [
    "No silent shadowing of built-in or define-block entity kinds",
    "Deterministic resolution via config-driven policies",
    "Mirrors the enhancement_policy pattern for consistency",
    "Adds entity_kind_policy and entity_kinds to specforge.json surface",
    "Parser must handle @plugin/kind qualified entity keywords",
  ]

  invariants [entity_kind_uniqueness]
}

decision entity_enhancement_model "Entity Enhancement Model" {
  status   accepted
  date     2026-03-03

  context """
    Plugins need to add fields and edges to existing entity types (e.g., a
    hexagonal architecture plugin adding a layer field to behavior). Without
    entity enhancement, plugins are limited to creating new entity types —
    they cannot annotate or extend the built-in entities. This blocks a
    significant class of architectural plugins.
  """

  decision """
    Plugin manifests declare entity enhancements with target entity kind,
    field name, field type, and optional edge mappings. The compiler builds
    a FieldRegistry combining built-in and enhanced fields, threads it
    through the resolve/graph-build/validate pipeline. Conflicts between
    plugins are resolved via configurable policies (error/priority/namespace).
    Built-in field shadowing always produces E018 regardless of policy.
    specforge doctor provides visibility into all enhancements.
  """

  consequences [
    "Plugins can annotate existing entities without forking the grammar",
    "FieldRegistry becomes the single source of truth for field definitions",
    "Conflict policies add configuration surface that must be documented",
    "E018 hard error prevents accidental built-in shadowing",
    "specforge doctor provides actionable conflict resolution",
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
    "CLI cold start <50ms per plugin with cache hit",
    "First-time compilation takes 100-500ms per plugin",
    "Cache directory grows proportionally to installed plugins",
    "Platform migration (e.g., x86 to ARM) invalidates entire cache",
    "Self-healing cache prevents silent corruption from causing failures",
  ]

  invariants [aot_cache_integrity, plugin_load_order_determinism]
}

decision contribution_based_extension_model "Contribution-Based Extension Model" {
  status   accepted
  date     2026-03-03

  context """
    The Terraform-style extension model (ADR terraform_style_extension_model)
    forces each package into a single PluginKind role — plugin, provider, or
    generator. This means a package like @specforge/jira that contributes
    entities, ref validation, AND code generation must be split into three
    separate packages, tripling distribution, versioning, and configuration
    overhead. A 10-expert panel (RES-23) unanimously recommended replacing
    the role-based taxonomy with a contribution-based model.
  """

  decision """
    Replace PluginKind with a contributes manifest key. Each package declares
    what it contributes: entities, validators, generators, providers. The
    compiler routes to namespaced Wasm exports based on contributions.
    Per-call-site host function permissions enforce least-privilege for each
    export — validators get query_graph + emit_diagnostic, generators
    additionally get emit_file, providers additionally get http_get.
    V1 manifests with kind field are auto-migrated with W028 warning.
  """

  consequences [
    "Single package = full integration — no more splitting @specforge/jira into three packages",
    "Contribution-level indexing enables fine-grained enable/disable via config",
    "Lower barrier to entry — one package to author, test, and publish",
    "Per-call-site permissions maintain defense-in-depth despite single-package model",
    "V1 manifest backward compatibility via automatic kind-to-contributes migration",
    "Supersedes terraform_style_extension_model role-based taxonomy",
  ]

  invariants [reference_resolution_completeness]
}

decision adr_package_source_resolution "Package Source Resolution" {
  status   accepted
  date     2026-03-03

  context """
    Packages need to be installable from multiple sources for different
    development workflows. Local development requires loading from
    filesystem paths without publishing. Teams need to share packages
    via git before a registry is available. Production deployments
    require pinned registry versions.
  """

  decision """
    Three package source types: registry (@scope/name@version for npm),
    local (./path for filesystem), and git (git:url#ref for repositories).
    The specifier format supports both string shorthand ("@specforge/product@^1.0")
    and object form ({ source: "local", path: "./my-plugin" }) in
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

decision package_version_pinning "Package Version Pinning" {
  status   accepted
  date     2026-03-03

  context """
    Without version pinning, specforge add at different times produces
    different package versions. This makes builds non-reproducible and
    leads to inconsistent behavior across team members and CI. The problem
    is amplified by the Wasm binary format — unlike source code, binary
    differences are opaque and hard to debug.
  """

  decision """
    specforge.lock pins exact resolved versions with SHA256 wasm_hash for
    each installed package. The lock file format is deterministic — same
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

decision per_call_site_sandbox "Per-Call-Site Sandbox" {
  status   accepted
  date     2026-03-03

  context """
    The contribution-based model means one package can contribute entities,
    generation, and provider validation. A per-package sandbox policy is
    too coarse — a package's validator should not have emit_file access
    just because the same package also contributes a generator. The
    principle of least privilege requires finer-grained permissions.
  """

  decision """
    Host function permissions are enforced per export call site, not per
    package. Each contribution type has a fixed permission set: validators
    get query_graph + emit_diagnostic, generators additionally get emit_file,
    providers additionally get http_get. The runtime checks the current
    export context before dispatching host function calls.
  """

  consequences [
    "Least-privilege per operation — validator cannot write files even if same package has a generator",
    "Defense in depth — compromised validator export cannot escalate to filesystem access",
    "Runtime must track which export is currently executing per call",
    "Permission model is static and predictable — no runtime configuration needed",
  ]

  invariants [wasm_sandbox_integrity]
}
