// Zero-entity core architecture invariants

invariant zero_domain_knowledge_core "Zero Domain Knowledge Core" {
  guarantee """
    The core compiler MUST have zero hardcoded entity types. All domain
    vocabulary — entity kinds, edge types, field definitions, validation
    rules, and testability flags — MUST come exclusively from installed
    extensions. The core MUST only understand structural parsing of
    keyword name { fields } blocks, use imports, reference lists, string
    fields, verify declarations, and define meta-blocks.
  """
  risk high

  verify property "core with zero extensions installed has zero entity kinds in KindRegistry"
  verify unit "compiling a .spec file with no extensions produces only structural parse, no kind validation"

}

invariant registry_population_before_validation "Registry Population Before Validation" {
  guarantee """
    All registries (KindRegistry, FieldRegistry, edge type set) MUST be
    fully populated from all installed extension manifests before any semantic
    validation begins. The two-phase approach MUST guarantee that Phase 1
    (structural parsing) completes for all files before Phase 2 (semantic
    validation against registries) starts.
  """
  risk high

  verify property "no validation diagnostic references a kind that was registered after validation started"
  verify unit "adding an extension that defines kind X makes X available in the validation phase"

}

invariant declarative_validation_determinism "Declarative Validation Determinism" {
  guarantee """
    Given the same set of installed extensions and the same .spec source files,
    the declarative validation engine MUST produce an identical set of
    diagnostics on every invocation. The order of diagnostic emission MUST
    be deterministic. No randomness or timing-dependent logic MUST influence
    which diagnostics are produced or their ordering.
  """
  risk medium

  verify property "same extensions and sources produce identical diagnostics across 100 runs"
  verify unit "diagnostic ordering is deterministic regardless of extension load order"

}

invariant testable_entity_classification "Testable Entity Classification" {
  guarantee """
    Testability MUST be determined exclusively by the KindRegistry entries
    populated from extension manifests. An entity kind with testable=true
    MUST accept verify statements and participate in coverage calculations
    and code action suggestions. These flags MUST NOT be hardcoded — they
    come from the extension's ManifestEntityKind declarations. The core
    compiler MUST NOT assume any entity kind is testable by default.

    P2 justification: The testable flag is structural dispatch, not domain
    knowledge. The core routes verify acceptance and coverage counting based
    on a boolean flag without knowing what "testable" means in any domain.
    This parallels Terraform providers declaring CRUD capabilities — the
    core routes based on capability flags without knowing what the resource
    is. The core never interprets why an entity is testable, only that the
    extension declared it so.
  """


  risk medium

  verify unit "entity kind with testable=true in manifest accepts verify statements"
  verify unit "testable=true entity counts toward coverage"
  verify unit "testable=false entity excluded from coverage"
  verify unit "no default testability assumed by core"

}

invariant define_extension_kind_uniqueness "Define-Extension Kind Uniqueness" {
  guarantee """
    A define block MUST NOT register a kind name that is already registered
    by an installed extension. If a define block declares a kind name that
    collides with an extension-provided kind, the compiler MUST emit an
    E-level diagnostic identifying both the define block and the owning
    extension. Extension-registered kinds always take precedence over
    define blocks — define blocks are project-local overrides for kinds
    NOT provided by any extension.
  """
  risk medium

  verify unit "define block with kind name matching an extension kind produces E-level diagnostic"
  verify unit "define block with unique kind name succeeds"

}

invariant compilation_pipeline_ordering "Compilation Pipeline Ordering" {
  guarantee """
    The compilation pipeline MUST execute events in strict order:
    all_files_parsed → extension_manifests_loaded → registries_populated →
    define_blocks_registered → validation_complete. No phase MAY begin
    before all prior phases have completed.
  """
  risk critical

  verify property "pipeline events fire in declared order"

}
