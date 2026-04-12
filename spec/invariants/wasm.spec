// Wasm runtime invariants

invariant wasm_sandbox_integrity "Wasm Sandbox Integrity" {
  guarantee """
    Wasm extensions MUST NOT escape the Wasm sandbox. An extension MUST NOT
    access the host filesystem, network, or memory outside its linear
    memory region unless explicitly permitted by the sandbox policy.
    Any sandbox violation MUST trap the extension and emit a diagnostic.
  """
  risk high

  verify property "no extension can read or write outside its sandbox boundaries"
  verify unit "sandbox violation traps the extension and emits a diagnostic"

}

invariant extension_load_order_determinism "Extension Load Order Determinism" {
  guarantee """
    Given the same set of installed extensions, the compiler MUST produce
    the same topological load order on every invocation. The ordering
    MUST be deterministic and reproducible across platforms.
  """
  risk medium

  verify property "same extension set produces identical load order across 100 runs"
  verify unit "load order is deterministic across different platforms"

}

invariant peer_dependency_satisfaction "Peer Dependency Satisfaction" {
  guarantee """
    If an extension declares peer dependencies, the compiler MUST verify that
    all declared peers are installed and satisfy the declared semver ranges.
    Unsatisfied peer dependencies MUST produce an error diagnostic (E-level), not
    a silent degradation.
  """
  risk high

  verify unit "satisfied peer dependencies pass validation"
  verify unit "unsatisfied peer dependency produces an error diagnostic"
  verify unit "peer with wrong version range produces an error diagnostic"

}

// -- Cache & Isolation Invariants ---------------------------------------------

invariant aot_cache_integrity "AOT Cache Integrity" {
  guarantee """
    AOT artifacts MUST be verified on load by re-hashing the source .wasm
    binary and comparing against the cache key. Corrupted, truncated, or
    platform-mismatched cache entries MUST be automatically evicted and
    recompiled. The cache MUST NOT serve stale or invalid artifacts.
  """
  risk medium

  verify property "corrupted AOT artifact is detected and recompiled"
  verify unit "platform-mismatched cache entry is evicted"

}

invariant extension_isolation "Extension Isolation" {
  guarantee """
    An extension failure MUST NOT affect other extensions or the host compiler.
    After an extension traps or fails during any lifecycle phase, the remaining
    extensions MUST continue execution normally. The failed extension MUST be
    excluded from subsequent phases in the current compilation.
  """
  risk high

  verify property "extension trap does not affect other extensions"
  verify unit "failed extension excluded from subsequent phases"

}

invariant host_function_type_safety "Host Function Type Safety" {
  guarantee """
    Data exchanged between the host and extensions via host functions MUST
    conform to declared schemas. Malformed input from an extension MUST
    produce an ExtensionError diagnostic, not undefined behavior. The host
    MUST validate all extension-provided data before processing.
  """
  risk high

  verify unit "malformed extension input produces ExtensionError"
  verify unit "valid extension input is processed correctly"

}

// -- Entity Kind Invariants ---------------------------------------------------

invariant entity_kind_uniqueness "Entity Kind Uniqueness" {
  guarantee """
    No two extensions MAY register the same entity kind name. Three
    distinct collision codes apply: collisions with define block kinds
    MUST produce E022, collisions with structural keywords (spec, use,
    define) MUST produce E023, and collisions between two extension-
    registered kinds MUST produce E026. All collisions are detected at
    extension load time. The compiler never arbitrates conflicts —
    extension authors resolve collisions via renames or peer dependencies.
  """
  risk high

  verify property "no two extensions can silently register the same entity kind"
  verify unit "built-in keyword rejection is unconditional"

}

// -- Entity Enhancement Invariants --------------------------------------------

invariant enhancement_field_uniqueness "Enhancement Field Uniqueness" {
  guarantee """
    No two extensions MAY register the same field name for the same entity
    kind. When a conflict is detected, the compiler MUST resolve it
    according to the configured enhancement_policy or produce a hard
    error. The resolution MUST be deterministic and explicit.
  """
  risk medium

  verify property "no two extensions can silently claim the same field"
  verify unit "conflict resolution is deterministic across runs"

}

invariant enhancement_builtin_precedence "Enhancement Built-in Precedence" {
  guarantee """
    Core grammar-level constructs MUST always take precedence over extension
    enhancements. An extension MUST NOT register an enhancement field whose
    name collides with a grammar-level construct: the entity title (the
    string after keyword and ID) or verify statements. These constructs are parsed by dedicated grammar
    rules and exist independently of the FieldRegistry. Attempts to
    shadow them MUST produce E018 regardless of enhancement_policy
    configuration.
  """
  risk high

  verify unit "enhancement shadowing grammar-level construct produces E018"
  verify unit "E018 not configurable via enhancement_policy"

}

// -- Collector Invariants ---------------------------------------------------

invariant collector_output_conformance "Collector Output Conformance" {
  guarantee """
    Collector output MUST conform to the specforge-report/v1 schema. Every
    CollectorReport MUST include a valid schema field, entries array, and
    stats object. Entity IDs referenced in collector entries MUST be validated
    against the graph — unknown entity IDs MUST produce a W029 warning, not
    a hard error, to allow partial coverage ingestion.
  """
  risk medium

  verify unit "valid collector output passes schema validation"
  verify unit "unknown entity ID in collector entry produces W029"
  verify unit "missing required fields produce hard error"

}

// -- Registry Invariants ----------------------------------------------------

invariant registry_integrity "Registry Integrity" {
  guarantee """
    Downloaded extension binaries from a registry MUST be verified against
    their declared SHA256 hash before installation. Hash mismatches MUST
    produce a hard error diagnostic and abort installation. The trust level
    of the source MUST be recorded in specforge.lock.
  """
  risk high

  verify unit "SHA256 match passes verification"
  verify unit "SHA256 mismatch produces hard error and aborts"
  verify unit "trust level recorded in lock file"

}

invariant extension_operation_atomicity "Extension Operation Atomicity" {
  guarantee """
    Extension install, uninstall, and update operations MUST be atomic.
    On failure, all changes MUST be rolled back — no partial installs,
    no orphaned files, no inconsistent lock state.
  """
  risk high

  verify unit "failed install rolls back to previous state"
  verify unit "interrupted upgrade preserves original extension"
  verify integration "concurrent install and uninstall are serialized"
}

invariant credential_secrecy "Registry Credential Secrecy" {
  guarantee """
    Raw authentication tokens MUST never be logged, stored in
    specforge.json, or included in diagnostic output. Only token
    presence/absence and validity status may be reported.
  """
  risk high

  verify unit "registry token is not included in log output"
  verify unit "diagnostic messages report credential presence not value"
  verify property "no log line contains raw token string"
}

invariant renderer_output_restriction "Renderer Output Restriction" {
  guarantee """
    Extension renderers MUST only emit spec-layer diagnostic artifacts:
    coverage reports, traceability matrices, validation dashboards, graph
    visualizations. Renderers MUST NOT produce application source code,
    configuration files, executable artifacts, or any output consumed by
    end users or deployed to production. SpecForge provides context;
    agents produce code.
  """
  risk high
  verify unit "emit_file rejects blacklisted code extensions"
  verify unit "renderer output restricted to allowed_output_extensions"
  verify property "no renderer can bypass emit_file extension whitelist"
}

// -- Extension-Defined Grammar Invariants ------------------------------------

invariant grammar_composition_determinism "Grammar Composition Determinism" {
  guarantee """
    Given the same set of installed extensions and the same
    GrammarConflictPolicy, grammar composition MUST produce identical
    results. Extension load order MUST NOT affect which grammar is
    selected for a given entity kind.
  """

  risk critical

  verify property "same extensions + same policy = same grammar mapping"
  verify unit "extension load order does not affect grammar selection"

}

invariant grammar_injection_isolation "Grammar Injection Isolation" {
  guarantee """
    One extension's grammar MUST NOT affect the parsing behavior of
    another extension's entity kinds. Grammar contributions are scoped
    to their declared entity_kinds only. A malformed or crashing grammar
    MUST NOT prevent other grammars from loading or functioning.
  """

  risk high

  verify property "grammar scoped to declared entity_kinds only"
  verify unit "malformed grammar does not affect other extensions"

}

invariant body_parser_output_conformance "Body Parser Output Conformance" {
  guarantee """
    Body parser Wasm exports MUST return JSON that conforms to the
    declared output schema (if present) or to the FieldMap structure
    expected by Phase 2 validation. Non-conforming output MUST be
    rejected with a BodyParserError, and the system MUST fall back
    to treating the body as a raw string field.
  """

  risk high

  verify property "parser output always conforms to declared schema"
  verify unit "non-conforming output produces BodyParserError"
  verify mutation "removing output validation allows invalid fields through"

}

invariant surface_schema_validity "Surface Schema Validity" {
  guarantee """
    Extension-contributed MCP tool input schemas and CLI command argument
    types MUST conform to JSON Schema draft 2020-12 and declared type
    constraints. Invalid schemas MUST produce E037. Unknown argument types
    MUST produce E038.
  """

  risk medium

  verify unit "valid MCP tool schema passes validation"
  verify unit "invalid MCP tool schema produces E037"
  verify unit "known command arg type passes validation"
  verify unit "unknown command arg type produces E038"

}
