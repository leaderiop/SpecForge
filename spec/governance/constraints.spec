requirement PERFORMANCE-02 "Incremental Compilation Latency" {
  category "Performance"
  tier     v2
  text     """
    Incremental recompilation after a file change must produce diagnostics within 100ms for projects up to 500 spec files.

    file_change_to_diagnostics < 100ms
    with up to 500 .spec files in the project
  """
}

requirement PERFORMANCE-03 "Memory Usage" {
  category "Performance"
  tier     v2
  text     """
    Peak memory consumption must stay under 50MB for projects with up to 500 spec files and 2000 entities.

    peak_memory < 50MB
    for a project with 500 .spec files and approximately 2000 entities
  """
}

requirement COMPATIBILITY-02 "Cross-Platform Compatibility" {
  category "Compatibility"
  tier     v2
  text     """
    CLI and LSP binaries must build and run on Linux, macOS, and Windows with no platform-specific file path handling.

    The CLI and LSP binaries MUST build and run on:
    Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64).
    No platform-specific file path handling.
  """
}

requirement USABILITY-02 "Diagnostic Quality" {
  category "Usability"
  tier     v2
  text     """
    Every error diagnostic must include file path, line/column numbers, a context snippet, and actionable suggestions when a close match exists.

    Every error diagnostic MUST include: file path, line number,
    column number, context snippet, and at least one actionable
    suggestion when a close match exists.
  """
}

requirement PORTABILITY-03 "Zero Runtime Dependencies" {
  category "Portability"
  tier     v2
  text     """
    CLI and LSP binaries must be statically linked with zero runtime dependencies beyond the OS, requiring no Node.js, Python, or JVM.

    The specforge-cli and specforge-lsp binaries MUST be statically
    linked with zero runtime dependencies beyond the OS. No Node.js,
    Python, or JVM required at runtime. The wasmtime runtime is statically
    linked into the binary.
  """
}

requirement COMPATIBILITY-01 "Backward Compatibility" {
  category "Compatibility"
  tier     v2
  text     """
    The compiler should parse spec files from the previous format version without errors, with deprecation warnings acceptable.

    The compiler SHOULD parse .spec files from the previous format version
    without errors. Deprecation warnings are acceptable.
  """
}

requirement USABILITY-01 "CLI Scaffolding Robustness" {
  category "Usability"
  tier     v2
  text     """
    Init and add commands must produce valid specforge.json and starter spec files without overwriting existing files or diverging between interactive and non-interactive modes.

    init/add commands produce valid, parseable specforge.json and a
    starter .spec file; existing files never overwritten without
    consent; non-interactive mode produces identical output to
    interactive mode for the same inputs
  """
}

requirement RELIABILITY-05 "Parser Correctness" {
  category "Reliability"
  tier     v2
  text     """
    The parser must produce correct ASTs for all valid inputs and recover from all syntax errors without crashing.

    parser produces correct AST for all valid inputs and recovers
    from all syntax errors without crashing
  """
}

requirement USABILITY-03 "Editor Integration Quality" {
  category "Usability"
  tier     v2
  text     """
    Tree-sitter query files must load without errors in editors, and extension entities must receive syntax highlighting via the generic entity_block rule.

    highlights.scm, folds.scm, indents.scm load without errors
    in Tree-sitter editors; extension entities receive syntax
    highlighting via generic entity_block rule and extension
    query extensions
  """
}

requirement RELIABILITY-30 "Reference Resolution Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Every valid reference must resolve exactly once and every invalid reference must produce exactly one diagnostic.

    every valid reference resolves exactly once; every invalid
    reference produces exactly one diagnostic
  """
}

requirement RELIABILITY-34 "Validation Pass Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Each validation code must fire if and only if its specified condition holds, with zero false positives and zero false negatives.

    each validation code fires if and only if the specified condition
    holds; zero false positives, zero false negatives
  """
}

requirement RELIABILITY-02 "Extension Validation Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Extension validation rules must run only when the owning extension is installed, with no false positives when the extension is absent.

    extension-specific validation rules run only when the owning extension is
    installed; no false positives when extension absent; the core declarative
    validation engine executes ValidationRulePattern entries from extension
    manifests without knowledge of domain-specific entity types
  """
}

requirement RELIABILITY-04 "Output Format Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Every emitter must produce syntactically valid output in its target format and produce deterministic results across runs.

    every emitter produces syntactically valid output in its format
    (JSON, DOT); deterministic across runs
  """
}

requirement USABILITY-04 "LSP Responsiveness" {
  category "Usability"
  tier     v2
  text     """
    All LSP features including go-to-definition, hover, and completion must respond within 50ms on a 500-file project.

    go-to-def, hover, completion respond within 50ms
    on a 500-file project
  """
}

requirement RELIABILITY-33 "Traceability Completeness" {
  category "Reliability"
  tier     v2
  text     """
    Traceability analysis must detect all broken links in the chain with zero false negatives, including missing tests, non-existent paths, and absent report results.

    traceability_false_negatives <= 0
    specforge analyze coverage MUST detect all broken links in the traceability
    chain with zero false negatives: testable entities with verify
    declarations but no recorded test results, and recorded results
    that name unknown entities.
  """
}

requirement RELIABILITY-01 "Extension System Integrity" {
  category "Reliability"
  tier     v2
  text     """
    Extension install and remove operations must never corrupt specforge.json, and the Wasm sandbox must contain all extension execution with traps producing diagnostics rather than crashes.

    extension_integrity_violations <= 0
    extension install/remove never corrupts specforge.json; Wasm sandbox
    contains all extension execution; Wasm traps produce diagnostics, not
    crashes; provider registration survives reload; timeout enforcement
    prevents runaway extensions
  """
}

requirement PERFORMANCE-16 "Wasm Compile Cache Budget" {
  category "Performance"
  tier     v2
  text     """
    Wasm extensions must load from the compile cache in under 50ms, measured in process as the load of a compile-cache hit plus the handshake and describe calls that declare the extension, and first compilation must complete in under 500ms for typical components.

    cached_extension_load < 50ms
    Each Wasm extension MUST load in under 50ms on a compile-cache hit.
    cached_extension_load is read in process: the runtime's load of a
    compile-cache hit plus the handshake and describe calls that declare
    the extension, which is what the host pays for it (ADR 0373 D3).
    First compilation (cache cold) SHOULD complete in under 500ms for a
    typical component (<1MB .wasm).
    Measured 2026-11-17 on the vendored @gsd/core (5,470,859 B, the
    largest builtin): 32 to 33ms in process (median of 8 runs, three
    times; the ignored test
    gsd_core_loads_from_the_compile_cache_in_under_fifty_ms). The CLI
    differential of a check loading it, 51ms wall and 47ms CPU (median
    of 20), adds the host's registry build over its declaration and the
    runtime's 10ms epoch-ticker join, so wall time moves in 10ms steps;
    its cold compile is 285ms wall.
  """
}

requirement PERFORMANCE-17 "Wasm Memory Limit" {
  category "Performance"
  tier     v2
  text     """
    Each Wasm extension instance is limited to the linear memory its handshake declares (max_memory_mb), held to the host ceiling of 512 MB, which is also the limit of an extension declaring none; a growth past it traps the call.

    extension_memory <= 512MB
    Each Wasm extension instance MUST be limited to its declared
    max_memory_mb of linear memory and tables, all its memories counted
    together, at most 512 MB; an extension declaring
    no limit MUST be limited to 512 MB. A growth past the limit MUST trap
    the call (memory_limit_exceeded, reported as E028). No limit is shared
    across extensions: one extension's memory never fails another
    (extension_isolation, ADR 0037).
  """
}

requirement SECURITY-01 "Wasm Sandbox Enforcement" {
  category "Security"
  tier     v2
  text     """
    Extensions must not access the host filesystem, network, environment variables, arguments, stdin or process control: the host grants a component no WASI capability, whatever its declaration asks for, with zero sandbox escapes in adversarial testing.

    sandbox_escapes <= 0
    Zero sandbox escapes in adversarial testing. A component's WASI
    context MUST preopen no directory and pass no environment, arguments
    or stdin, and MUST refuse TCP, UDP and name lookup. No declaration
    (sandbox_policy, a surface's sandbox override) grants a capability
    (ADR 0037); effects only through the four granted ports (ADR 0335 D4).
  """
}

requirement PORTABILITY-01 "Wasm Binary Size Limit" {
  category "Portability"
  tier     v2
  text     """
    Extension Wasm binaries should be under 5MB, with a warning diagnostic for binaries exceeding 10MB to ensure reasonable download and compilation times. The one exception is the vendored builtin @gsd/core, held to its own allowance by the blob-size check.

    extension_wasm_size < 5MB
    Each extension .wasm binary SHOULD be under 5MB. Binaries exceeding 10MB
    MUST produce a warning diagnostic at install time. This ensures
    reasonable download times and first-compilation latency.
    One exception: the vendored builtin @gsd/core (5,683,413 B, about
    half of it its prompt and agent assets) is held to its own
    allowance, 5,700,000 B, which the blob-size check enforces
    (gate/src/blob_size.rs, ADR 0366 D3).
  """
}

requirement PERFORMANCE-01 "Extension Count Limit" {
  category "Performance"
  tier     v2
  text     """
    The system should support up to 20 extensions without degradation, warn at 21-50, and refuse more than 50 with a hard error.

    loaded_extensions <= 50
    The system SHOULD support up to 20 extensions without performance
    degradation. 21-50 extensions MUST produce a warning diagnostic.
    More than 50 extensions MUST be refused with a hard error.
  """
}

requirement PORTABILITY-02 "Wasm Compile Cache Size Limit" {
  category "Portability"
  tier     v2
  text     """
    The wasmtime compile cache directory must stay under 500MB total; the engine's built-in size-based cleanup removes least-recently-used entries beyond the soft limit.

    compile_cache_size < 500MB
    The compile cache directory SHOULD stay under 500MB total. The
    runtime configures wasmtime's cache with a 500MB files-total-size
    soft limit; when exceeded, the engine's cleanup removes
    least-recently-used entries on subsequent runs. No info diagnostic
    is required — cleanup is engine-internal.
  """
}

requirement RELIABILITY-03 "Formatting Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Formatter output must be idempotent, preserve all comments, produce an identical compiler graph, and complete within 50ms per file.

    formatter output is idempotent (format(format(x)) == format(x));
    all comments preserved; formatted files produce identical compiler graph;
    single file formatted in under 50ms
  """
}

requirement INTEROPERABILITY-01 "Schema Publication Accessibility" {
  category "Interoperability"
  tier     v2
  text     """
    The Graph Protocol JSON Schema must be publicly accessible without authentication at a URL that remains stable across patch versions.

    The Graph Protocol schema MUST be publicly accessible as a standalone
    JSON Schema document. The published schema MUST be downloadable without
    authentication. The schema URL MUST be stable across patch versions.
  """
}
