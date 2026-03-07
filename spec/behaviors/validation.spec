// Validation behaviors — core graph validation rules

use invariants/core
use invariants/validation
use invariants/zero-entity-core
use types/core
use types/graph
use types/diagnostics
use types/errors
use types/zero-entity-core
use ports/outbound
use events/compilation

behavior detect_dangling_references "Detect Dangling References" {
  invariants [reference_resolution_completeness]
  types      [Diagnostic, EntityId, ValidationCode, Severity, ValidationError, Graph, Edge]
  consumes  [graph_built]
  // Diagnostics from this validator flow into the declarative_validation_executed
  // aggregate and ultimately to validation_complete. No separate event produced.

  contract """
    This behavior is a post-resolution integrity assertion. User-facing
    E001 diagnostics are emitted by link_entity_references during resolution.
    This behavior delegates to link_entity_references for E001 emission.
    The validator MUST NOT re-emit E001 for references already flagged
    during resolution. The validator's role is to verify that every
    reference list entry has a corresponding graph edge — if not, it
    indicates a resolver bug, not a user error.
  """

  verify unit "reference without corresponding graph edge indicates resolver bug"
  verify unit "reference with corresponding graph edge passes"
  verify unit "empty graph with zero edges produces no dangling reference diagnostic"

}

behavior detect_duplicate_entity_ids "Detect Duplicate Entity IDs" {
  invariants [string_interning_consistency, entity_id_uniqueness]
  types      [Diagnostic, DuplicateIdError]
  consumes  [all_files_parsed]
  // Diagnostics from this validator flow into the declarative_validation_executed
  // aggregate and ultimately to validation_complete. No separate event produced.

  contract """
    The validator MUST detect entity IDs declared more than once across
    all .spec files. Duplicate IDs MUST produce an E002 diagnostic that
    names both declaration sites (file, line, column).
  """

  verify unit "duplicate ID in same file produces E002"
  verify unit "duplicate ID across files produces E002"
  verify unit "E002 includes both source locations"

}


// ── Domain-Specific Validation ──────────────────────────────
// Domain-specific validations (orphan entity checks, unused reference
// warnings, unverified entity warnings, trigger consistency checks, etc.)
// are declared as ValidationRulePatterns in extension manifests and are
// defined in their owning extension directories:
//   - spec/extensions/software/validation-rules.spec
//   - spec/extensions/product/validation-rules.spec
//   - spec/extensions/governance/validation-rules.spec
// The core compiler executes these patterns generically via the declarative
// validation engine (see behaviors/zero-entity-core.spec).
// Diagnostic codes (W001, W003, W004, W007, E006, etc.) are defined by
// their owning extensions, not by the core compiler.

// ── Structural Validation (core — domain-agnostic) ───────────
// DiagnosticBag accumulator pattern: core structural validators do not emit
// individual diagnostic events. Instead, each validator appends Diagnostic
// values into a shared DiagnosticBag. After all structural validators have
// run, the bag is drained into the declarative_validation_executed aggregate
// and forwarded to validation_complete. This avoids O(n) event fan-out for
// large graphs and lets the pipeline batch diagnostics for deterministic
// ordering (see diagnostic_determinism invariant).
// Core structural validations operate on graph topology and field presence
// WITHOUT knowledge of entity semantics. They check: dangling references,
// duplicate IDs, import cycles, orphan structural nodes (W012 for any
// grammar-level structural kind — ref, spec — with zero incoming edges),
// and file-reference existence. Extension-defined entity kinds opt into
// generic orphan detection via extension-defined `no_incoming_edges`
// ValidationRulePatterns. Domain-specific orphan rules (e.g., W001 orphan
// entity) are extension-defined ValidationRulePatterns.

behavior detect_orphan_refs "Detect Orphan Structural Nodes" {
  invariants [diagnostic_determinism]
  types      [Diagnostic, EntityId, Graph]
  consumes  [graph_built]
  // Diagnostics from this validator flow into the declarative_validation_executed
  // aggregate and ultimately to validation_complete. No separate event produced.

  // H1: ref and spec are grammar-level structural constructs — parsed by the
  // core grammar (like `use` and `define`), NOT extension-defined entity kinds.
  // Because they are structural, their orphan detection belongs in core, not in
  // extension manifests. Extension-defined entity kinds that want orphan
  // detection declare a `no_incoming_edges` ValidationRulePattern in their
  // extension manifest, which the declarative validation engine handles
  // separately.

  contract """
    The core validator MUST detect orphan nodes of ANY structural node
    type (ref, spec) that have zero incoming edges in the compiled
    graph. Orphan structural nodes MUST produce a W012 warning
    identifying the unreferenced node.

    This is a generic structural check applied uniformly to all
    grammar-level structural kinds — it does not encode domain
    knowledge about any specific kind. The set of structural kinds is
    defined by the grammar (currently ref and spec), not by
    extensions.

    Extension-defined entity kinds that want generic orphan detection
    declare a `no_incoming_edges` ValidationRulePattern in their
    extension manifest, which the declarative validation engine
    handles separately.
  """

  verify unit "unreferenced ref produces W012"
  verify unit "referenced ref suppresses W012"
  verify unit "unreferenced structural node of any grammar-level kind produces W012"
  verify unit "structural node with at least one incoming edge suppresses W012"

}

// Core structural validation: checks file existence for ANY field declared as
// a file reference in extension metadata. This is purely structural and
// domain-agnostic — the core checks that referenced files exist, just like it
// checks that referenced entity IDs exist (E001). Gherkin .feature files are
// the current use case, but this mechanism applies to any file-reference field
// registered by any extension (e.g., a future extension could declare an
// `openapi` field as a file reference).
// W018 (missing gherkin on supported kind) is an extension-level concern:
// it is declared as a missing_field_when_flag_set ValidationRulePattern by
// @specforge/software, not hardcoded in core. The core only handles E016.
behavior validate_file_reference_paths "Validate File Reference Paths" {
  // reference_resolution_completeness applies here because file paths are a form
  // of reference that must resolve: just as entity-ID references must resolve to
  // graph nodes, file-path references must resolve to existing filesystem entries.
  // Both are "references" in the graph-integrity sense — unresolved file paths
  // violate the same completeness guarantee as dangling entity references.
  invariants [reference_resolution_completeness]
  types      [Diagnostic]
  ports      [FileSystem]
  consumes  [graph_built]
  // Diagnostics from this validator flow into the declarative_validation_executed
  // aggregate and ultimately to validation_complete. No separate event produced.

  contract """
    This behavior performs generic file-path validation for any entity
    field whose extension metadata declares it as a file reference.
    The validation is domain-agnostic: the core does not interpret the
    semantics of the referenced file — it only checks existence.
    Gherkin .feature file references (from @specforge/software) are the
    current use case, but any extension can register file-reference
    fields (e.g., openapi, protobuf, schema) and they will be
    validated by this same mechanism.

    File existence (E016): every file path in a file-reference field
    MUST reference an existing file relative to the spec root.
    Non-existent files MUST produce an E016 diagnostic.

    Field-presence warnings (e.g., W018 for missing gherkin on a kind
    with supportsGherkin=true) are NOT handled here — they are declared
    as missing_field_when_flag_set ValidationRulePatterns in the owning
    extension manifest and executed by the declarative validation engine.
  """

  verify unit "non-existent file reference produces E016"
  verify unit "existing file reference passes silently"

}

// validate_extension_testability moved to behaviors/zero-entity-validation.spec
// where its feature owner (zero-entity core) lives.
