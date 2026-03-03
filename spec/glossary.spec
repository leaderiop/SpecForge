// SpecForge Glossary — Ubiquitous language for the project

glossary {

  // ── Compiler Pipeline ──────────────────────────────────────

  term "spec file" {
    definition """
      A source file with the .spec extension containing entity declarations
      in the SpecForge DSL. Parsed by the compiler into an AST.
    """
    aliases ["dot-spec file", ".spec file"]
  }

  term "compiler pipeline" {
    definition """
      The sequence of stages that transforms .spec source files into a validated
      in-memory graph: Parser -> Resolver -> Graph Builder -> Validator -> Emitter.
    """
    aliases ["pipeline", "compilation pipeline"]
  }

  term "parser" {
    definition """
      The first compiler stage. Uses a Tree-sitter grammar to transform
      .spec source text into a per-file Abstract Syntax Tree (AST).
    """
    context "SpecForge uses Tree-sitter, not a hand-written recursive descent parser."
  }

  term "resolver" {
    definition """
      The compiler stage that processes use imports, links entity ID references
      to their declarations, and builds the in-memory graph. Processes files
      in topological order (dependencies first).
    """
    aliases ["reference resolver", "import resolver"]
  }

  term "in-memory graph" {
    definition """
      A directed graph of nodes (entities) and edges (relationships) built from
      parsed .spec files. Serves as the compiler's database — no external database
      required. Mutable and incrementally updatable for watch mode.
    """
    aliases ["graph", "spec graph", "entity graph"]
  }

  term "validator" {
    definition """
      The compiler stage that enforces graph invariants: no dangling references,
      no duplicate IDs, no import cycles, orphan detection. Emits diagnostics
      at error, warning, and info levels.
    """
  }

  term "emitter" {
    definition """
      An output generator that traverses the in-memory graph to produce
      artifacts: markdown documentation, JSON graph, DOT visualization,
      traceability reports, or generated code stubs.
    """
    aliases ["renderer", "output generator"]
  }

  term "diagnostic" {
    definition """
      A compiler message with a severity (error, warning, info), a validation
      code, source location, and human-readable message. Styled like rustc
      output. Error codes: E001-E012, E015-E018, E020, E022-E023.
      Warning codes: W001-W012, W015-W019, W023, W027-W028.
      Info codes: I001, I003-I006.
    """
    aliases ["compiler diagnostic", "validation message"]
  }

  // ── Entity Model ───────────────────────────────────────────

  term "entity" {
    definition """
      A declared block in a .spec file that represents a specification concept.
      Each entity has a unique ID, typed fields, and participates in the
      traceability chain via compiler-checked cross-references.
    """
  }

  term "entity ID" {
    definition """
      A globally unique free-form identifier for an entity. Any valid identifier
      (letters, digits, underscores, 2-60 chars, starts with a letter). No
      enforced case convention — projects choose their own naming style.
    """
    aliases ["ID", "entity identifier"]
  }

  term "traceability chain" {
    definition """
      The directed path through the entity graph from high-level artifacts
      down to guarantees: deliverable -> capability -> feature -> behavior
      -> invariant. Every link is compiler-checked.
    """
    aliases ["trace chain", "traceability path"]
  }

  term "use import" {
    definition """
      A file-level directive that brings symbols from another .spec file
      into scope for reference resolution. Syntax: use path/to/file.
      The .spec extension is implicit.
    """
    aliases ["import", "use directive"]
    context "Not the same as a programming language import — brings spec symbols into scope, not code."
  }

  term "reference list" {
    definition """
      A bracket-delimited list of entity IDs that creates compiler-checked
      cross-references. Syntax: [spec_root_singleton, multi_error_collection]. Every ID must resolve
      to a declared entity or a diagnostic is emitted.
    """
  }

  term "soft reference" {
    definition """
      A cross-module reference that degrades gracefully when the target
      module is not installed. Emits I004 (info) instead of E001 (error)
      when the referenced entity's plugin is missing.
    """
    context "Used for core -> plugin and plugin -> plugin references."
  }

  // ── Extension Model ────────────────────────────────────────

  term "plugin" {
    definition """
      A Wasm extension (.wasm binary) that adds entity types, edge types, and
      validation rules to the compiler. Loaded via the Extism runtime. Communicates
      with the compiler via host functions (specforge.register_entity,
      specforge.emit_diagnostic, etc.). Official plugins: @specforge/product
      and @specforge/governance.
    """
    context "Extends the entity model, not the output format. Compare with provider and generator."
  }

  term "provider" {
    definition """
      A Wasm extension that registers ref schemes for external reference validation.
      Uses the specforge.http_get host function for network validation. Validates
      identifier patterns, resolves URLs, and supports multiple aliased instances.
      Example: @specforge/gh for GitHub references.
    """
    context "Extends ref validation, not the entity model. Compare with plugin and generator."
  }

  term "generator" {
    definition """
      A Wasm extension that reads the compiled graph via the specforge.query_graph
      host function and produces output files via specforge.emit_file. Diagnostics
      are emitted via specforge.emit_diagnostic. Any language with a Wasm
      compilation target can implement a generator. Example: @specforge/gen-typescript.
    """
    aliases ["code generator", "output plugin"]
    context "Extends output formats, not the entity model. Compare with plugin and provider."
  }

  // ── Compilation Concepts ───────────────────────────────────

  term "incremental compilation" {
    definition """
      The watch mode strategy: file change triggers invalidation of the
      changed file plus transitive dependents, re-parsing only invalidated
      files, rebuilding affected subgraph edges, and re-validating the
      affected subgraph. Target: <100ms file-change-to-diagnostics.
    """
    aliases ["incremental recompilation", "watch mode"]
  }

  term "string interning" {
    definition """
      A memory optimization where frequently compared strings (entity IDs,
      file paths, identifiers) are stored once and compared by pointer
      equality instead of character-by-character. SpecForge uses the
      lasso crate for this.
    """
    context "An implementation detail that affects performance, not user-visible behavior."
  }

  term "RPN" {
    definition """
      Risk Priority Number. Calculated as severity x occurrence x detection
      in an FMEA failure_mode block. Higher RPN means higher risk priority.
      The compiler validates the arithmetic (E005).
    """
    aliases ["risk priority number"]
  }

  term "spec root" {
    definition """
      The project configuration in specforge.json that declares project
      identity (name, version), installed plugins, provider
      configurations, personas, and surfaces. Exactly one per project.
    """
    aliases ["project root", "spec block"]
  }

  // ── Code Generation ────────────────────────────────────────

  term "port" {
    definition """
      An interface boundary between the domain and the outside world,
      declared as a port block. Inbound ports define what the system offers;
      outbound ports define what the system requires. Ports generate
      language-specific interfaces.
    """
    context "In SpecForge, port means a hexagonal architecture port, not a network port."
  }

  term "drift detection" {
    definition """
      The process of verifying that generated code files match the current
      state of .spec files. specforge gen --check exits with code 1 if
      regenerating would produce different output.
    """
  }

  term "specforge-report.json" {
    definition """
      The standard JSON report file emitted by test runner plugins. Contains
      per-behavior test results (pass/fail/skip/duration) and per-invariant
      violation test results. Consumed by specforge coverage.
    """
    aliases ["coverage report", "test report"]
  }

  term "verify statement" {
    definition """
      A declaration inside a behavior block specifying how to test that
      behavior. Syntax: verify unit|integration|property|load|e2e "description".
      Behaviors without verify statements trigger W004.
    """
  }

  // ── Rust Plugin ───────────────────────────────────────────

  term "specforge-test" {
    definition """
      A runtime Rust crate published on crates.io that provides the
      #[specforge::test("entity_id")] proc macro attribute and the
      Drop-based TestGuard for recording test results.
    """
    aliases ["specforge test crate"]
  }

  term "specforge-test-macros" {
    definition """
      A proc macro Rust crate published on crates.io that contains the
      attribute macro implementation for #[specforge::test]. Depends on
      specforge-test for the runtime guard.
    """
    aliases ["specforge test macros crate"]
  }

  term "test guard" {
    definition """
      A Drop-based Rust struct (TestGuard) that records whether a test
      passed or failed. On drop, checks std::thread::panicking() — false
      means pass, true means fail. Results are written to target/specforge/
      via an atexit handler.
    """
    aliases ["TestGuard", "drop guard"]
  }

  term "entity mapping" {
    definition """
      The process of resolving which spec entity a test function corresponds
      to. Uses three-level precedence: tests field (1st) > proc macro
      attribute (2nd) > naming convention (3rd).
    """
    aliases ["test-to-entity mapping", "entity resolution"]
  }

  term "spec coverage" {
    definition """
      The percentage of testable entities that have passing tests. Distinct
      from code coverage — spec coverage measures how many spec-declared
      behaviors, invariants, events, constraints, and capabilities have
      verified test results.
    """
    aliases ["specification coverage"]
  }

  term "collect command" {
    definition """
      The specforge collect subcommand that ingests test output from a
      language-specific format and emits specforge-report.json. Follows
      the Go-style verb-noun pattern: specforge collect rust.
    """
    aliases ["specforge collect"]
  }

  term "drift checksum" {
    definition """
      A SHA256 hash embedded in a @specforge-checksum header comment at
      the top of generated files. Used by specforge gen --check to detect
      when generated code is stale relative to the current spec state.
    """
    aliases ["checksum header", "@specforge-checksum"]
  }

  term "naming convention" {
    definition """
      The convention for Rust test function names that enables convention-based
      entity mapping: {entity_id}__{description_slug}. The double underscore
      separates the entity ID from the test description.
    """
  }

  term "double underscore separator" {
    definition """
      The __ delimiter used in Rust test function names to separate the
      entity ID from the description slug. Unambiguous because entity IDs
      (snake_case) never contain double underscores. Example:
      validate_input__rejects_empty_name.
    """
  }

  term "JUnit XML" {
    definition """
      An XML test report format produced by cargo-nextest. The primary
      machine-readable format for specforge collect rust. Contains
      testcase elements with classname, duration, and failure messages.
    """
    aliases ["JUnit report", "nextest XML"]
  }

  // ── Editor & Syntax Support ────────────────────────────────

  term "generic entity block" {
    definition """
      A grammar fallback rule that matches identifier-name-body patterns
      not matching any built-in entity keyword. Produces a clean AST node
      with a kind field containing the unknown keyword, enabling syntax
      highlighting, code folding, and document symbols for plugin and
      custom entity types without grammar modifications.
    """
    aliases ["generic_entity_block"]
    context "Part of Tier 1 of the 3-tier highlighting architecture. See RES-22."
  }

  term "query extension" {
    definition """
      A .scm query pattern shipped by a plugin in its manifest to extend
      editor syntax highlighting, code folding, or indentation for
      plugin-specific entity types. Composed with base query files by
      string concatenation in plugin load order.
    """
    aliases ["query extension pattern", ".scm extension"]
    context "Part of Tier 2 of the 3-tier highlighting architecture. See RES-22."
  }

  term "semantic token" {
    definition """
      An LSP 3.16+ protocol mechanism for runtime syntax classification
      beyond tree-sitter static queries. The LSP server assigns token
      types (keyword, property, type, etc.) to source ranges. Used to
      classify custom entity keywords, enhanced fields, and cross-plugin
      references that static query files cannot capture.
    """
    context "Part of Tier 3 of the 3-tier highlighting architecture. See RES-22."
  }

  // ── Wasm/Extism Runtime ─────────────────────────────────────

  term "Wasm" {
    definition """
      WebAssembly — a portable binary instruction format used as the
      universal plugin runtime for SpecForge. Plugins, providers, and
      generators compile to .wasm binaries that run in a sandboxed
      environment via the Extism runtime.
    """
    aliases ["WebAssembly", ".wasm"]
  }

  term "Extism" {
    definition """
      A cross-language framework for building WebAssembly plugin systems.
      SpecForge uses Extism as its sole plugin runtime, providing host
      function registration, linear memory management, and sandboxed
      execution. Statically linked into the specforge binary.
    """
  }

  term "host function" {
    definition """
      A function provided by the SpecForge compiler that Wasm plugins can
      call during execution. Host functions are the only way plugins
      interact with the compiler and host system. Standard host functions:
      specforge.query_graph, specforge.emit_diagnostic, specforge.register_entity,
      specforge.register_edge, specforge.emit_file, specforge.http_get.
    """
    aliases ["host fn"]
  }

  term "linear memory" {
    definition """
      The contiguous block of memory available to a Wasm module. Each
      plugin instance has its own isolated linear memory, capped at 64MB
      by default. The host cannot be accessed outside this boundary —
      attempts to do so result in a trap.
    """
  }

  term "AOT compilation" {
    definition """
      Ahead-of-Time compilation of .wasm binaries to native machine code.
      Cached in .specforge/cache/ using content-hash filenames. Reduces
      plugin cold start to <50ms. Platform-specific — cache entries include
      the target platform in their filename.
    """
    aliases ["ahead-of-time compilation", "AOT"]
  }

  term "peer dependency" {
    definition """
      A plugin's declared requirement that another plugin must be installed
      and satisfy a semver version range. Peer dependencies determine
      topological load order and are validated at compiler startup.
      Unsatisfied peers produce a hard error.
    """
  }

  term "sandbox policy" {
    definition """
      A configuration object that defines the security boundaries for a
      Wasm plugin: maximum memory, execution time limit, allowed filesystem
      paths, allowed network domains, and access levels. Enforced by the
      Extism runtime and host function implementations.
    """
  }

  term "PDK" {
    definition """
      Plugin Development Kit — the set of libraries, templates, and
      documentation for authoring SpecForge Wasm plugins. Available for
      Rust, Go, JavaScript/TypeScript, and other languages with Wasm
      compilation targets. Accessed via specforge plugin init.
    """
    aliases ["Plugin Development Kit"]
  }

  term "entity enhancement" {
    definition """
      A plugin's ability to add fields and edges to existing entity types
      via declarations in its manifest.json. Enhanced fields participate
      in parsing, resolution, and validation like built-in fields. Conflicts
      between plugins are resolved via configurable enhancement policies.
    """
    aliases ["field enhancement", "enhancement"]
  }

  term "Wasm trap" {
    definition """
      An unrecoverable WebAssembly error such as out-of-bounds memory
      access, stack overflow, or unreachable instruction. The Extism
      runtime catches all traps and converts them to Result errors.
      Trapped plugins transition to the failed lifecycle state.
    """
    aliases ["trap", "Wasm fault"]
  }

  term "fuel metering" {
    definition """
      Extism/Wasmtime's instruction counting mechanism for enforcing
      execution time limits on Wasm plugins. Each Wasm instruction
      consumes fuel; when the fuel budget is exhausted, the plugin
      traps. Prevents runaway plugins from blocking compilation.
    """
  }

  term "content-addressed cache" {
    definition """
      The AOT cache naming strategy where compiled artifacts are stored
      using the SHA256 hash of the source .wasm binary as the filename.
      This ensures cache hits are always valid and cache misses trigger
      recompilation. The platform triple is included in the key.
    """
  }

  term "enhancement policy" {
    definition """
      The strategy for resolving conflicts when two plugins register the
      same field name for the same entity kind. Three policies: error
      (default, hard error on conflict), priority (first plugin wins,
      warning emitted), namespace (conflicting fields prefixed with
      plugin name). Configured in specforge.json.
    """
    aliases ["conflict policy"]
  }
}
