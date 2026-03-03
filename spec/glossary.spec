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
      output. Error codes: E001-E003, E005-E012 (E004 reserved). Warning
      codes: W001-W012. Info codes: I001, I003-I005 (I002 reserved).
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
      A globally unique identifier following the pattern PREFIX-INFIX-NUMBER
      (e.g., BEH-SF-001). The infix is declared in the spec root and scopes
      all IDs to the project.
    """
    aliases ["ID", "entity identifier"]
  }

  term "infix" {
    definition """
      A 2-4 uppercase letter code declared in the spec root block that scopes
      all entity IDs to a project. For SpecForge, the infix is SF.
    """
    context "Not a general programming term — specific to SpecForge ID patterns."
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
      An extension that adds entity types, edge types, and validation rules
      to the compiler. Installed via plugins list in the spec root.
      Official plugins: @specforge/product and @specforge/governance.
    """
    context "Extends the entity model, not the output format. Compare with provider and generator."
  }

  term "provider" {
    definition """
      An extension that registers ref schemes for external reference validation.
      Validates identifier patterns, resolves URLs, and supports multiple
      aliased instances. Example: @specforge/gh for GitHub references.
    """
    context "Extends ref validation, not the entity model. Compare with plugin and generator."
  }

  term "generator" {
    definition """
      An extension that reads the compiled graph and produces output files.
      Uses the subprocess I/O protocol (stdin JSON graph, stdout files,
      stderr diagnostics). Example: @specforge/gen-typescript.
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
      The singleton spec block in specforge.spec that declares project
      identity (name, infix, version), installed plugins, provider
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
}
