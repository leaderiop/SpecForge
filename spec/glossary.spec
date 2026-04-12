// SpecForge Glossary — Ubiquitous language for the project
//
// This glossary contains terms for the core compiler (pipeline stages,
// entity model primitives, extension model, graph protocol, Wasm runtime,
// editor support) AND extension-specific domain terms. The glossary keyword
// is a singleton — only one per project — so extension terms are grouped
// under clearly labeled section headers below.


// ── Compiler Pipeline ──────────────────────────────────────

term t_spec_file "spec file" {
  definition """
    A source file with the .spec extension containing entity declarations
    in the SpecForge DSL. Parsed by the compiler into an AST.
  """
  aliases ["dot-spec file", ".spec file"]
  see_also   [t_parser, t_extension, t_spec_root, t_zero_entity_core, t_sandbox_policy, t_wasm_trap, t_fuel_metering, t_enhancement_policy, t_proof_obligation, t_sync_block, t_progressive_formality, t_rpn]
}

term t_compiler_pipeline "compiler pipeline" {
  definition """
    The sequence of stages that transforms .spec source files into a validated
    in-memory graph. Compilation is two-phase with strict ordering:

      Phase 1: Parser (structural parsing of all .spec files)
               → Extension Loader (load manifests, populate registries)

      Phase 2: Resolver (link references, build graph edges)
               → Validator (structural + declarative checks)
               → Emitter (output artifacts)

    Phase 2 MUST NOT begin until all registries (KindRegistry, FieldRegistry,
    edge type set) are fully populated from Phase 1. Within each phase, stages
    are sequential — resolution completes before validation begins.
  """
  aliases ["pipeline", "compilation pipeline"]
  see_also   [t_resolver, t_validator, t_emitter, t_spec_file]
}

term t_parser "parser" {
  definition """
    The first compiler stage. Uses a Tree-sitter grammar to transform
    .spec source text into a per-file Abstract Syntax Tree (AST).
  """
  context "SpecForge uses Tree-sitter, not a hand-written recursive descent parser."
  see_also   [t_grammar_injection]
}

term t_resolver "resolver" {
  definition """
    The compiler stage that processes use imports and links entity ID
    references to their declarations. Records resolved references as
    pending edges for the graph builder to materialize. Processes files
    in topological order (dependencies first).
  """
  aliases ["reference resolver", "import resolver"]
  see_also   [t_use_import, t_in_memory_graph, t_soft_reference]
}

term t_in_memory_graph "in-memory graph" {
  definition """
    A directed graph of nodes (entities) and edges (relationships) built from
    parsed .spec files. Serves as the compiler's database — no external database
    required. Mutable and incrementally updatable for watch mode.
  """
  aliases ["graph", "spec graph", "entity graph"]
}

term t_validator "validator" {
  definition """
    The compiler stage that enforces graph invariants: no dangling references,
    no duplicate IDs, no import cycles, orphan detection. Emits diagnostics
    at error, warning, and info levels.
  """
}

term t_emitter "emitter" {
  definition """
    An output stage that traverses the in-memory graph to produce
    artifacts: Graph Protocol JSON, markdown documentation, DOT visualization,
    traceability reports, or token-optimized agent context. Code generation
    is NOT a SpecForge concern — AI agents generate code by consuming the
    Graph Protocol.
  """
  aliases ["renderer"]
  see_also   [t_renderer, t_agent_context]
}

term t_diagnostic "diagnostic" {
  definition """
    A compiler message with a severity (error, warning, info), a validation
    code, source location, and human-readable message. Styled like rustc
    output. Diagnostic codes are partitioned between core and extensions:

    Core error codes (structural validation):
      E001 (dangling ref), E002 (duplicate ID), E003 (import cycle),
      E011 (invalid ID format), E012 (invalid field value),
      E013 (unknown provider kind), E015 (unsupported format version),
      E016 (missing file reference), E017 (enhancement conflict unresolved),
      E018 (enhancement shadows grammar-level construct),
      E020 (missing contribution export), E022 (entity kind collision
      with define block), E023 (reserved entity kind),
      E024 (unknown entity kind), E025 (missing import file),
      E026 (unresolved entity kind conflict between extensions),
      E027 (incompatible schema version),
      E028 (unknown or unsupported manifest version).

    Core warning codes (structural validation):
      W012 (orphan ref), W015 (deprecated syntax), W017 (testable entity
      without verify — extension-declared testability flag),
      W018 (missing file-reference field on extension-supported kind — pattern-driven),
      W019 (unused import), W020 (unknown field name on registered entity
      kind — field not in FieldRegistry for the entity's kind),
      W023 (deprecated field), W026 (enhancement conflict resolved by
      priority policy), W027 (entity kind conflict resolved by priority
      policy).

    Core info codes:
      I001 (unused glossary term), I003 (older format version suggestion),
      I004 (soft ref unresolved), I005 (unrecognized ref scheme),
      I006 (verify but not testable), I007 (cross-extension ref info),
      I002 (no extensions installed), I016 (schema cache not found).

    Allocation ranges (non-overlapping, codes listed per owner):
      Core (structural):    E001-E003, E011-E028,
                            W012, W015, W017-W020, W023, W026-W027,
                            I001-I007, I016

      @specforge/software:  E004, E006, E030-E035,
                            W001-W005, W007-W010, W028-W040,
                            I008-I009, I011, I014-I015
      @specforge/product:   E007-E009, E010,
                            W041-W044,
                            I010
      @specforge/governance: E005,
                            W047-W048

      Federation (@specforge/federation):    E040, I012, W050-W052, W054

      Migration (core):     W053

      Wasm (core):          E029, I013

      Surface (core):       E036 (missing surface export),
                            E037 (invalid MCP tool input_schema),
                            E038 (unknown CLI command arg type),
                            E039 (duplicate surface contribution),
                            W055 (surface declared without Wasm binary),
                            W056 (MCP tool without description),
                            W057 (CLI command without args),
                            I017 (auto-promoted tool conflicts with explicit)

      Reserved (future @specforge extensions): E040-E099,
                            W011, W013-W014, W016, W021-W022, W024-W025,
                            W045-W046, W049, W058-W099,
                            I018-I099
      Third-party extensions: E100+, W100+, I100+

    Note: Allocation is by owner, not by contiguous numeric range.
    Each code has exactly one owner.
  """
  aliases ["compiler diagnostic", "validation message"]
}

// ── Entity Model ───────────────────────────────────────────

term t_entity "entity" {
  definition """
    A declared block in a .spec file that represents a specification concept.
    Each entity has a unique ID, typed fields, and participates in the
    traceability chain via compiler-checked cross-references. Entity types
    are NOT built into the core compiler — they are defined by installable
    extensions. The core is a pure typed-graph engine.
  """
  see_also   [t_spec_coverage, t_verify_statement, t_entity_mapping, t_compiler_pipeline]
}

term t_entity_id "entity ID" {
  definition """
    A globally unique free-form identifier for an entity. Any valid identifier
    (letters, digits, underscores, 2-60 chars, starts with a letter). No
    enforced case convention — projects choose their own naming style.
  """
  aliases ["ID", "entity identifier"]
  see_also   [t_string_interning]
}

term t_use_import "use import" {
  definition """
    A file-level directive that brings symbols from another .spec file
    into scope for reference resolution. Syntax: use path/to/file.
    The .spec extension is implicit.
  """
  aliases ["import", "use directive"]
  context "Not the same as a programming language import — brings spec symbols into scope, not code."
  see_also   [t_reference_list]
}

term t_reference_list "reference list" {
  definition """
    A bracket-delimited list of entity IDs that creates compiler-checked
    cross-references. Syntax: [spec_root_singleton, multi_error_collection]. Every ID must resolve
    to a declared entity or a diagnostic is emitted.
  """
}

term t_soft_reference "soft reference" {
  definition """
    A cross-module reference that degrades gracefully when the target
    extension is not installed. Emits I004 (info) instead of E001 (error)
    when the referenced entity's extension is missing.
  """
  context "Used for cross-extension references when the target extension is not installed."
  see_also   [t_diagnostic]
}

// ── Extension Model ────────────────────────────────────────

term t_extension "extension" {
  definition """
    A Wasm extension (.wasm binary) that provides domain vocabulary to the
    compiler: entity kinds, edge types, validation rules, and testability
    flags. Loaded via the Extism runtime. The core compiler has zero built-in
    entity types — ALL domain knowledge comes from extensions. Official extensions:
    @specforge/software, @specforge/product, @specforge/governance. Domain
    extensions: @specforge/atomic-design, @specforge/compliance, @specforge/api-design.
  """
  aliases ["plugin", "domain extension"]
  context "Terraform-exact model: core has zero domain knowledge, all vocabulary from extensions. 'Extension' is the canonical term for entity IDs and code references; 'plugin' is accepted as a human-facing alias only."
  see_also   [t_provider, t_peer_dependency]
}

term t_provider "provider" {
  definition """
    A Wasm extension that registers ref schemes for external reference validation.
    Uses the specforge.http_get host function for network validation. Validates
    identifier patterns, resolves URLs, and supports multiple aliased instances.
    Example: @specforge/gh for GitHub references.
  """
  context "A contribution type within an extension — extends ref validation, not the entity model."
}

term t_renderer "renderer" {
  definition """
    A contribution type within a Wasm extension that reads the compiled graph
    via the specforge.query_graph host function and produces non-code output
    files (reports, dashboards, traceability matrices) via specforge.emit_file.
    Code generation is NOT a SpecForge concern — AI agents generate code by
    consuming the Graph Protocol. Renderers produce supplementary artifacts
    only.
  """
  context "A contribution type for non-code outputs only. Code generation is the responsibility of AI agents, not SpecForge. Renamed from 'exporter' to avoid confusion with code generation."
  see_also   [t_drift_detection]
}

// ── Compilation Concepts ───────────────────────────────────

term t_incremental_compilation "incremental compilation" {
  definition """
    The watch mode strategy: file change triggers invalidation of the
    changed file plus transitive dependents, re-parsing only invalidated
    files, rebuilding affected subgraph edges, and re-validating the
    affected subgraph. Target: <100ms file-change-to-diagnostics.
  """
  aliases ["incremental recompilation", "watch mode"]
}

term t_string_interning "string interning" {
  definition """
    A memory optimization where frequently compared strings (entity IDs,
    file paths, identifiers) are stored once and compared by pointer
    equality instead of character-by-character. SpecForge uses the
    lasso crate for this.
  """
  context "An implementation detail that affects performance, not user-visible behavior."
  see_also   [t_incremental_compilation]
}

term t_spec_root "spec root" {
  definition """
    The project configuration in specforge.json that declares project
    identity (name, version), installed extensions, provider
    configurations, personas, and surfaces. Exactly one per project.
  """
  aliases ["project root", "spec block"]
}

term t_domain_vocabulary "domain vocabulary" {
  definition """
    The set of entity kinds, edge types, and validation rules that an
    extension contributes to the compiler. Each extension defines its own
    domain-specific vocabulary — the core compiler has no vocabulary of
    its own. Extensions for different domains contribute entirely different
    entity kinds.
  """
  aliases ["vocabulary", "domain model"]
  context "The zero-entity core architecture means vocabulary is 100% extension-defined."
}

term t_zero_entity_core "zero-entity core" {
  definition """
    The architectural principle that the SpecForge compiler core has zero
    built-in entity types. The core is a pure typed-graph engine that parses
    any keyword-name-body block, resolves references, builds graphs, and
    validates constraints — but has no opinion about what entity types exist.
    Domain semantics are entirely defined by installable extensions.
  """
  aliases ["zero-core", "entity-free core"]
  context "Terraform-exact analogy: Terraform core has zero infrastructure knowledge, SpecForge core has zero domain knowledge."
  see_also   [t_domain_vocabulary]
}

// ── Graph Protocol & Agent Context ────────────────────────────

term t_graph_protocol "graph protocol" {
  definition """
    The stable JSON schema that specforge export produces — the contract
    between the compiler and any consuming AI agent. The Graph Protocol is
    SpecForge's primary product: a structured, validated, cross-referenced
    representation of human intent that any agent can consume for any task.
  """
  aliases ["Graph Protocol", "agent context protocol"]
  context "The graph schema is the standard. The compiler and DSL are implementation details."
  see_also   [t_multi_resolution_query]
}

term t_agent_context "agent context" {
  definition """
    A token-optimized export format produced by specforge export --format=context.
    Designed for direct injection into AI agent context windows. Contains entity
    IDs, contracts, relationships, and traceability chains in a compact JSON
    representation. Any AI agent (coding, PM, compliance, docs, security) can
    consume it.
  """
  aliases ["context export", "agent-context output"]
  see_also   [t_port]
}

term t_multi_resolution_query "multi-resolution query" {
  definition """
    A graph query that returns entities at a specific zoom level — from
    project-wide summaries down to individual entity details. Zoom levels
    depend on installed extensions and the entity hierarchy they define.
    Allows agents to request exactly the context slice they need via
    specforge query --scope and --hop flags. Inspired by Large Concept
    Models research on operating at the right abstraction level.
  """
  aliases ["scoped query", "resolution query"]
  see_also   [t_query_extension]
}

// ── Editor & Syntax Support ────────────────────────────────

term t_body_parser "body parser" {
  definition """
    A Wasm export provided by an extension that transforms raw entity body
    text into structured JSON fields during Phase 1.5 of compilation. Enables
    extensions to define custom syntax beyond the core key-value field format.
  """
  see_also   [t_entity]
}

term t_generic_entity_block "generic entity block" {
  definition """
    The single grammar rule that parses ALL entity blocks as
    keyword name [title] { fields } structures. There are no
    "built-in" keywords at the grammar level — every entity block
    uses this rule. Produces a clean AST node with a kind field
    containing the keyword, enabling syntax highlighting, code
    folding, and document symbols for all entity types. Only spec,
    ref, use, and define have separate dedicated grammar rules due
    to unique structural syntax (ref uses scheme:identifier format).
  """
  aliases ["generic_entity_block", "entity_block"]
  context "Part of the zero-entity core architecture. The grammar is keyword-agnostic."
  see_also   [t_graph_protocol]
}

term t_grammar_composition "grammar composition" {
  definition """
    The process of combining grammar contributions from multiple extensions
    into a coherent highlighting configuration, governed by
    GrammarConflictPolicy.
  """
  see_also   [t_grammar_contribution, t_generic_entity_block]
}

term t_grammar_contribution "grammar contribution" {
  definition """
    An extension contribution that provides a tree-sitter grammar .wasm
    binary for editor syntax highlighting of extension-defined entity kinds.
  """
}

term t_grammar_injection "grammar injection" {
  definition """
    The mechanism by which extension-provided tree-sitter grammars are loaded
    into the LSP for syntax highlighting of extension-defined entity kinds.
  """
}

term t_query_extension "query extension" {
  definition """
    A .scm query pattern shipped by an extension in its manifest to extend
    editor syntax highlighting, code folding, or indentation for
    extension-specific entity types. Composed with base query files by
    string concatenation in extension load order.
  """
  aliases ["query extension pattern", ".scm extension"]
  context "Part of Tier 2 of the 3-tier highlighting architecture. See RES-22."
  see_also   [t_semantic_token]
}

term t_semantic_token "semantic token" {
  definition """
    An LSP 3.16+ protocol mechanism for runtime syntax classification
    beyond tree-sitter static queries. The LSP server assigns token
    types (keyword, property, type, etc.) to source ranges. Used to
    classify entity keywords, enhanced fields, and cross-extension
    references that static query files cannot capture.
  """
  context "Part of Tier 3 of the 3-tier highlighting architecture. See RES-22."
  see_also   [t_glossary_entity_enhancement, t_entity_id]
}

// ── Wasm/Extism Runtime ─────────────────────────────────────

term t_wasm "Wasm" {
  definition """
    WebAssembly — a portable binary instruction format used as the
    universal extension runtime for SpecForge. Extensions compile to
    .wasm binaries that run in a sandboxed environment via the
    Extism runtime.
  """
  aliases ["WebAssembly", ".wasm"]
  see_also   [t_aot_compilation, t_grammar_composition]
}

term t_extism "Extism" {
  definition """
    A cross-language framework for building WebAssembly plugin systems
    (Extism itself refers to these as plugins). SpecForge uses Extism
    as its sole extension runtime, providing host function registration,
    linear memory management, and sandboxed execution. Statically linked
    into the specforge binary.
  """
}

term t_host_function "host function" {
  definition """
    A function provided by the SpecForge compiler that Wasm extensions can
    call during execution. Host functions are the only way extensions
    interact with the compiler and host system. Standard host functions:
    specforge.query_graph (read the compiled graph), specforge.emit_diagnostic
    (emit a compiler diagnostic), specforge.emit_file (write an output file),
    specforge.http_get (fetch a URL for provider validation).
    Additionally, specforge.add_graph_node and specforge.add_graph_edge
    allow extensions to add graph node and edge instances at runtime.
    Entity kinds and edge types are declared in extension manifests.
  """
  aliases ["host fn"]
  see_also   [t_linear_memory]
}

term t_linear_memory "linear memory" {
  definition """
    The contiguous block of memory available to a Wasm module. Each
    extension instance has its own isolated linear memory, capped at 64MB
    by default. The host cannot be accessed outside this boundary —
    attempts to do so result in a trap.
  """
}

term t_aot_compilation "AOT compilation" {
  definition """
    Ahead-of-Time compilation of .wasm binaries to native machine code.
    Cached in .specforge/cache/ using content-hash filenames. Reduces
    extension cold start to <50ms. Platform-specific — cache entries include
    the target platform in their filename.
  """
  aliases ["ahead-of-time compilation", "AOT"]
  see_also   [t_extism]
}

term t_peer_dependency "peer dependency" {
  definition """
    An extension's declared requirement that another extension must be installed
    and satisfy a semver version range. Peer dependencies determine
    topological load order and are validated at compiler startup.
    Unsatisfied peers produce a hard error.
  """
  see_also   [t_edk]
}

term t_sandbox_policy "sandbox policy" {
  definition """
    A configuration object that defines the security boundaries for a
    Wasm extension: maximum memory, execution time limit, allowed filesystem
    paths, allowed network domains, and access levels. Enforced by the
    Extism runtime and host function implementations.
  """
}

term t_edk "EDK" {
  definition """
    Extension Development Kit — the set of libraries, templates, and
    documentation for authoring SpecForge Wasm extensions. Available for
    Rust, Go, JavaScript/TypeScript, and other languages with Wasm
    compilation targets. Accessed via specforge extension init.
  """
  aliases ["Extension Development Kit"]
  see_also   [t_host_function]
}

term t_glossary_entity_enhancement "entity enhancement" {
  definition """
    An extension's ability to add fields and edges to existing entity types
    via declarations in its manifest.json. Enhanced fields participate
    in parsing, resolution, and validation like extension-defined fields.
    Conflicts between extensions are resolved via configurable enhancement
    policies.
  """
  aliases ["field enhancement", "enhancement"]
  see_also   [t_body_parser]
}

term t_wasm_trap "Wasm trap" {
  definition """
    An unrecoverable WebAssembly error such as out-of-bounds memory
    access, stack overflow, or unreachable instruction. The Extism
    runtime catches all traps and converts them to Result errors.
    Trapped extensions transition to the failed lifecycle state.
  """
  aliases ["trap", "Wasm fault"]
}

term t_fuel_metering "fuel metering" {
  definition """
    Extism/Wasmtime's instruction counting mechanism for enforcing
    execution time limits on Wasm extensions. Each Wasm instruction
    consumes fuel; when the fuel budget is exhausted, the extension
    traps. Prevents runaway extensions from blocking compilation.
  """
}

term t_content_addressed_cache "content-addressed cache" {
  definition """
    The AOT cache naming strategy where compiled artifacts are stored
    using the SHA256 hash of the source .wasm binary as the filename.
    This ensures cache hits are always valid and cache misses trigger
    recompilation. The platform triple is included in the key.
  """
  see_also   [t_wasm]
}

term t_enhancement_policy "enhancement policy" {
  definition """
    The strategy for resolving conflicts when two extensions register the
    same field name for the same entity kind. Three policies: error
    (default, hard error on conflict), priority (first extension wins,
    warning emitted), namespace (conflicting fields prefixed with
    extension name). Configured in specforge.json.
  """
  aliases ["conflict policy"]
}

// ── @specforge/software Formal Methods Terms ──────────────────

term t_refinement_chain "refinement chain" {
  definition """
    Sequence of behavior refinements from abstract specification to
    concrete implementation via refines edges. Built by the
    refinement_verify compiler pass. Cycles produce E032.
  """
  aliases ["refinement path"]
}

term t_proof_obligation "proof obligation" {
  definition """
    Machine-readable verification condition generated by formal
    analysis passes. Each obligation tracks an entity ID, kind
    (contract_preservation, invariant_preservation, refinement_correctness),
    description, and discharge status (pending, auto_proved, test_verified).
  """
  see_also   [t_refinement_chain]
}

term t_sync_block "sync block" {
  definition """
    CSP synchronization declaration on event entities specifying
    barrier behaviors and timeouts. Parsed by the process_analyze
    compiler pass for deadlock and livelock detection.
  """
}

term t_progressive_formality "progressive formality" {
  definition """
    Five-level adoption path for formal methods in specifications:
    Level 0 (prose), Level 1 (entity_graph), Level 2 (contracts),
    Level 3 (invariants), Level 4 (proofs). Each level adds
    machine-checkable rigor without requiring the next. Per RES-25.
  """
  aliases ["formality levels"]
}

// ── @specforge/governance Terms ──────────────────────────────

term t_rpn "RPN" {
  definition """
    Risk Priority Number. Calculated as severity x occurrence x detection
    in an FMEA failure_mode block. Higher RPN means higher risk priority.
    The compiler validates the arithmetic (E005).
  """
  aliases ["risk priority number"]
}

// ── @specforge/rust Terms ────────────────────────────────────

term t_specforge_test "specforge-test" {
  definition """
    A runtime Rust crate published on crates.io that provides the
    #[specforge::test("entity_id")] proc macro attribute and the
    Drop-based TestGuard for recording test results. Part of the
    Rust extension for SpecForge.
  """
  aliases ["specforge test crate"]
  see_also   [t_specforge_test_macros, t_test_guard]
}

term t_specforge_test_macros "specforge-test-macros" {
  definition """
    A proc macro Rust crate published on crates.io that contains the
    attribute macro implementation for #[specforge::test]. Depends on
    specforge-test for the runtime guard.
  """
  aliases ["specforge test macros crate"]
}

term t_test_guard "test guard" {
  definition """
    A Drop-based Rust struct (TestGuard) that records whether a test
    passed or failed. On drop, checks std::thread::panicking() — false
    means pass, true means fail. Results are written to target/specforge/
    via an atexit handler.
  """
  aliases ["TestGuard", "drop guard"]
  see_also   [t_junit_xml]
}

term t_naming_convention "naming convention" {
  definition """
    The convention for Rust test function names that enables convention-based
    entity mapping: {entity_id}__{description_slug}. The double underscore
    separates the entity ID from the test description.
  """
  see_also   [t_double_underscore_separator]
}

term t_double_underscore_separator "double underscore separator" {
  definition """
    The __ delimiter used in Rust test function names to separate the
    entity ID from the description slug. Unambiguous because entity IDs
    (snake_case) never contain double underscores. Example:
    validate_input__rejects_empty_name.
  """
}

term t_junit_xml "JUnit XML" {
  definition """
    An XML test report format produced by cargo-nextest. The primary
    machine-readable format for specforge collect rust. Contains
    testcase elements with classname, duration, and failure messages.
  """
  aliases ["JUnit report", "nextest XML"]
  see_also   [t_collect_command]
}

// ── Traceability & Coverage (core concepts) ─────────────────

term t_traceability_chain "traceability chain" {
  definition """
    The directed path through the entity graph following registered edge
    types from extensions. The chain shape depends entirely on installed
    extensions — different extension combinations produce different chains.
    Every link is compiler-checked.
  """
  aliases ["trace chain", "traceability path"]
}

term t_spec_coverage "spec coverage" {
  definition """
    The percentage of testable entities that have passing tests. Distinct
    from code coverage — spec coverage measures how many testable
    entities (as declared by extensions via the testable flag in their
    manifest) have verified test results.
  """
  aliases ["specification coverage"]
}

term t_verify_statement "verify statement" {
  definition """
    A declaration inside a testable entity block specifying how to test
    that entity. Syntax: verify <kind> "description". Verify kinds are
    extension-defined — not hardcoded. Extensions declare allowedVerifyKinds
    per entity kind in their manifest. For example, @specforge/software
    declares unit, integration, property, load, e2e as its verify kinds
    — these are NOT core defaults, they exist only when the extension is
    installed. Testable entities without verify statements trigger W004
    (from the extension's missing_field_when_flag_set validation pattern).
  """
}

term t_specforge_report_json "specforge-report.json" {
  definition """
    The standard JSON report file produced by external test runners and
    ingested by specforge collect. Contains per-entity test results
    (pass/fail/skip/duration) for any testable entity kind. SpecForge
    reads these pre-generated reports — it never executes tests itself.
  """
  aliases ["coverage report", "test report"]
}

term t_collect_command "collect command" {
  definition """
    The specforge collect subcommand that ingests test output from a
    language-specific format and emits specforge-report.json. Follows
    the Go-style verb-noun pattern: specforge collect rust.
  """
  aliases ["specforge collect"]
  see_also   [t_specforge_report_json]
}

term t_entity_mapping "entity mapping" {
  definition """
    The process of resolving which spec entity a test function corresponds
    to. Uses three-level precedence: tests field (1st) > proc macro
    attribute (2nd) > naming convention (3rd).
  """
  aliases ["test-to-entity mapping", "entity resolution"]
  see_also   [t_specforge_test, t_naming_convention]
}

// ── @specforge/software Terms ────────────────────────────────
// These terms are defined by the @specforge/software extension.
// They exist in this glossary because the glossary keyword is a singleton.

term t_port "port" {
  definition """
    An entity kind defined by @specforge/software representing an interface
    boundary between the domain and the outside world. Inbound ports define
    what the system offers; outbound ports define what the system requires.
    Ports are declarative specifications — AI agents consume port
    declarations from the Graph Protocol.
  """
  context "Extension-defined entity kind from @specforge/software. In this domain, port means a hexagonal architecture port, not a network port."
  see_also   [t_traceability_chain]
}

// ── Extension Concerns (renderer-specific) ─────────────────

term t_drift_detection "drift detection" {
  definition """
    The process of verifying that output files (from third-party renderers)
    match the current state of .spec files. Renderer extensions implement
    drift detection via checksum headers. This is an extension concern, not
    a core compiler feature.
  """
  see_also   [t_drift_checksum]
}

term t_drift_checksum "drift checksum" {
  definition """
    A SHA256 hash embedded in a @specforge-checksum header comment at
    the top of renderer output files. Used by third-party renderer
    extensions to detect when their output is stale relative to the
    current spec state. This is a renderer extension concern, not a
    core compiler feature.
  """
  aliases ["checksum header", "@specforge-checksum"]
}
