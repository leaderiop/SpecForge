requirement INVARIANT-149 "Zero Domain Knowledge Core" {
  category "Invariant"
  tier     v2
  text     """
    The core compiler MUST have zero hardcoded entity types. All domain
    vocabulary — entity kinds, edge types, field definitions, validation
    rules, and testability flags — MUST come exclusively from installed
    extensions. The core MUST only understand structural parsing of
    keyword name { fields } blocks, use imports, reference lists, string
    fields, verify declarations, and define meta-blocks.
  """
}

requirement INVARIANT-120 "Registry Population Before Validation" {
  category "Invariant"
  tier     v2
  text     """
    All registries (KindRegistry, FieldRegistry, edge type set) MUST be
    fully populated from all installed extension manifests before any semantic
    validation begins. The two-phase approach MUST guarantee that Phase 1
    (structural parsing) completes for all files before Phase 2 (semantic
    validation against registries) starts.
  """
}

requirement INVARIANT-013 "Declarative Validation Determinism" {
  category "Invariant"
  tier     v2
  text     """
    Given the same set of installed extensions and the same .spec source files,
    the declarative validation engine MUST produce an identical set of
    diagnostics on every invocation. The order of diagnostic emission MUST
    be deterministic. No randomness or timing-dependent logic MUST influence
    which diagnostics are produced or their ordering.
  """
}

requirement INVARIANT-140 "Testable Entity Classification" {
  category "Invariant"
  tier     v2
  text     """
    Testability MUST be determined exclusively by the KindRegistry entries
    populated from extension manifests. An entity kind with testable=true
    MUST accept verify statements and participate in coverage calculations
    and code action suggestions. These flags MUST NOT be hardcoded — they
    come from the extension's EntityKindDescriptor declarations. The core
    compiler MUST NOT assume any entity kind is testable by default.

    P2 justification: The testable flag is structural dispatch, not domain
    knowledge. The core routes verify acceptance and coverage counting based
    on a boolean flag without knowing what "testable" means in any domain.
    This parallels Terraform providers declaring CRUD capabilities — the
    core routes based on capability flags without knowing what the resource
    is. The core never interprets why an entity is testable, only that the
    extension declared it so.
  """
}

requirement INVARIANT-007 "Compilation Pipeline Ordering" {
  category "Invariant"
  tier     v2
  text     """
    The compilation pipeline MUST execute events in strict order:
    all_files_parsed → extension_manifests_loaded → registries_populated →
    validation_complete. No phase MAY begin
    before all prior phases have completed.
  """
}
