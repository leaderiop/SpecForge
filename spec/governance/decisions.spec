// Architecture Decision Records

use invariants/core

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
  status   accepted
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
    installed via specforge add and declared in specforge.spec.
  """

  consequences [
    "Small core for simple projects — 8 entities usable immediately",
    "Progressive adoption — add plugins as projects grow",
    "Cross-plugin references use soft resolution (I004 if plugin missing)",
    "Plugin ecosystem requires stable API and manifest format",
    "Three extension mechanisms to document and maintain",
  ]

  invariants [reference_resolution_completeness]
}
