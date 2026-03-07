// Zero-entity core — manifest V2, dynamic registries, grammar, bootstrap, visualization, consistency

use invariants/zero-entity-core
use invariants/core
use types/zero-entity-core
use types/core
use types/config
use types/diagnostics
use types/errors
use types/wasm
use ports/outbound
use events/compilation

// -- Extension Manifest V2 ---------------------------------------------------

behavior validate_manifest_v2_schema "Validate Manifest V2 Schema" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ExtensionError]

  contract """
    The compiler MUST validate a manifest against the v2 schema. This
    behavior is a pure schema check — it does NOT handle v1 detection or
    migration. Required fields (name, version, manifest_version, wasm_path)
    MUST be present. The manifest_version MUST be 2. Unknown top-level
    fields MUST produce a warning. Malformed JSON MUST produce a hard error.
    This behavior is called by validate_extension_manifest (behaviors/wasm.spec)
    after initial manifest parsing. Schema validation MUST complete for all
    manifests before registry population begins.
  """

  verify unit "valid v2 manifest passes schema validation"
  verify unit "missing required field produces hard error"
  verify unit "manifestVersion != 2 produces hard error"
  verify unit "unknown top-level field produces warning"

}

behavior register_entity_kinds_from_manifest "Register Entity Kinds From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestEntityKind, KindRegistryEntry]

  contract """
    For each entityKinds entry in a extension manifest, the compiler MUST
    register the kind in the KindRegistry with full metadata: testable flag,
    singleton flag, supportsVerify flag, supportsGherkin flag, semantic
    token classification, LSP icon for outline, and DOT shape for
    visualization. The source extension name MUST be recorded for diagnostics
    and doctor output.
  """

  verify unit "entity kind registered with testable flag"
  verify unit "entity kind registered with singleton flag"
  verify unit "entity kind registered with LSP metadata"
  verify unit "source extension recorded in registry entry"
  // Testability registration (formerly register_testability_from_manifest)
  verify unit "testable=true entity participates in coverage"
  verify unit "testable=false entity excluded from coverage"
  verify unit "no default testability assumed by core"
  // Gherkin/scenario support registration (formerly register_scenario_support_from_manifest)
  verify unit "supportsGherkin=true entity allows gherkin blocks"
  verify unit "supportsGherkin=false entity warns on gherkin blocks"
  verify unit "no default gherkin support assumed by core"

}

behavior register_edge_types_from_manifest "Register Edge Types From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestEdgeType]

  contract """
    For each edgeTypes entry in a extension manifest, the compiler MUST
    register the edge label in the edge type set. The source and target
    kind constraints MUST be recorded for graph validation. Field-to-edge
    mappings from ManifestField entries MUST create corresponding edge
    type registrations.

    Duplicate edge labels across extensions MUST produce a W-level warning
    including both extension names. Resolution is deterministic:
    first-registered wins, where registration order follows topological
    sort of peer dependencies. Extensions without dependency relationships
    are ordered alphabetically by name. The winning registration's
    source_kind and target_kind constraints are used for graph validation;
    the duplicate's constraints are discarded.
  """

  verify unit "edge type registered with label and description"
  verify unit "source/target kind constraints recorded"
  verify unit "duplicate edge label across extensions produces W-level warning"
  verify unit "first-registered edge type wins on collision (topological order)"
  verify unit "field-to-edge mapping creates edge type"

}

behavior register_validation_rules_from_manifest "Register Validation Rules From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation, declarative_validation_determinism]
  types      [ManifestV2, ValidationRulePattern]

  contract """
    For each validationRules entry in a extension manifest, the compiler
    MUST parse and register the declarative rule. Registration is a two-step
    process:

    Step 1 (during registration): Rules are parsed and stored with their
    raw target_kind and edge_type strings. No cross-reference validation
    occurs at this point because peer-dependency extensions may not have
    registered their kinds yet.

    Step 2 (post-registration, after registries_populated): The compiler
    MUST validate all registered rules for internal consistency —
    target_kind references MUST match registered entity kinds, edge_type
    references MUST match registered edge types. Invalid references MUST
    produce a warning, not a hard error, to allow partial loading. This
    deferred validation ensures that cross-extension references (e.g., a
    governance rule targeting a software entity kind) resolve correctly
    regardless of extension load order.
  """

  verify unit "validation rule registered from manifest"
  verify unit "target_kind validation deferred to post-registration phase"
  verify unit "target_kind reference validated against KindRegistry after registries_populated"
  verify unit "edge_type reference validated against edge type set after registries_populated"
  verify unit "invalid reference produces warning not error"

}

behavior register_verify_kinds_from_manifest "Register Verify Kinds From Manifest" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestEntityKind]

  contract """
    When a manifest entityKind has an allowedVerifyKinds field, the compiler
    MUST register those verify kinds for entities of that kind. Verify kind
    names MUST NOT be hardcoded (no built-in verify kind names — all come from extension manifests).
    Extensions MAY define arbitrary verify kinds (e.g., "contract", "smoke",
    "chaos"). Unknown verify kinds in .spec files MUST produce a warning
    referencing the entity's extension.
  """

  verify unit "custom verify kinds registered from manifest"
  verify unit "no hardcoded verify kinds in core"
  verify unit "unknown verify kind in .spec produces W-level diagnostic in Phase 2"

}

// -- Dynamic Entity Registration ---------------------------------------------

behavior boot_empty_kind_registry "Boot Empty Kind Registry" {
  invariants [zero_domain_knowledge_core]
  types      [KindRegistryEntry]

  contract """
    When the compiler initializes, KindRegistry::new() MUST return an
    empty registry with zero entity kind entries. Only the structural
    keywords spec, ref, use, and define MUST be recognized by the
    parser (these have dedicated grammar rules). No extension-defined
    entity keywords MUST exist until extensions populate the registry.
  """

  verify unit "KindRegistry::new() has zero entries"
  verify unit "parser recognizes spec keyword without extensions"
  verify unit "parser recognizes ref keyword without extensions"
  verify unit "parser recognizes use keyword without extensions"
  verify unit "parser recognizes define keyword without extensions"

}

behavior boot_empty_field_registry "Boot Empty Field Registry" {
  invariants [zero_domain_knowledge_core]
  types      [FieldRegistryEntry]

  contract """
    When the compiler initializes, FieldRegistry::new() MUST return an
    empty registry with zero field definitions. No extension-defined field
    names MUST be recognized until extensions populate the registry. The
    entity title (the string after the entity keyword and ID) is a
    grammar-level positional element parsed by the generic_entity_block
    rule — it is NOT a FieldRegistry entry and does not participate in
    field validation.
  """

  verify unit "FieldRegistry::new() has zero entries"
  verify unit "no field names recognized before extension loading"
  verify unit "entity title parsed by grammar, not FieldRegistry"

}

behavior boot_empty_edge_registry "Boot Empty Edge Registry" {
  invariants [zero_domain_knowledge_core]
  types      [ManifestEdgeType]

  contract """
    When the compiler initializes, the edge type set MUST start empty with
    zero registered edge labels. No extension-defined edge types MUST exist
    until extensions populate the set. This parallels boot_empty_kind_registry
    and boot_empty_field_registry — all three registries begin empty and are
    populated exclusively from extension manifests.
  """

  verify unit "edge type set starts with zero entries"
  verify unit "no edge labels recognized before extension loading"

}

behavior custom_entity_types_via_define "Custom Entity Types via Define" {
  invariants [reference_resolution_completeness, entity_id_uniqueness, zero_domain_knowledge_core]
  types      [CompilerConfig, DefineBlockConfig, KindRegistryEntry]
  consumes   [registries_populated]
  produces   [custom_entity_type_defined, define_blocks_registered]

  contract """
    When a define block exists in a .spec file, the compiler MUST
    register the custom entity type as a DefineBlockConfig with its
    id_prefix, required fields, optional fields, and reference targets.
    The custom kind MUST be added to the KindRegistry so that subsequent
    entity blocks using that keyword are recognized. Custom entities MUST
    participate in reference resolution and orphan detection like
    extension-defined entities.

    Define blocks are "project-scoped domain knowledge" — they allow
    projects to extend the entity vocabulary without writing an extension.
    Define blocks MUST be processed in Phase 2, after extension loading
    and registry population (registries_populated event). This ordering
    ensures that define blocks MAY reference extension-defined kinds in
    their reference_targets (e.g., a custom "risk" kind targeting
    "behavior" from @specforge/software). After processing, the compiler
    MUST emit a custom_entity_type_defined event for each define block.
    Define-block kinds are registered with source_extension set to
    "<project>" to distinguish them from extension-provided kinds.
  """

  verify unit "custom entity type is registered in KindRegistry"
  verify unit "custom entity participates in reference resolution"
  verify unit "custom entity has orphan detection"
  verify unit "define block creates DefineBlockConfig with correct fields"
  verify unit "define blocks processed after registries_populated"
  verify unit "define block can reference extension-defined kinds"
  verify unit "define-block kind has source_extension '<project>'"
  verify unit "custom_entity_type_defined event emitted per define block"

}

behavior populate_kind_registry_from_extensions "Populate Kind Registry From Extensions" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestEntityKind, KindRegistryEntry]
  produces   [registries_populated]

  contract """
    After loading all extension manifests, the compiler MUST populate the
    KindRegistry by iterating extensions in topological order (peer deps
    first). For each extension, each entityKinds entry MUST be registered
    as a KindRegistryEntry. After population, the parser MUST recognize
    all registered keywords for subsequent files. Population MUST complete
    before any semantic validation begins.
  """

  verify unit "extensions iterated in topological order"
  verify unit "all entityKinds entries registered"
  verify unit "registered keywords available to parser"
  verify unit "population completes before validation"
  verify integration "two extensions register kinds without collision"

}

behavior populate_field_registry_from_extensions "Populate Field Registry From Extensions" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestField, FieldRegistryEntry]

  contract """
    After populating the KindRegistry, the compiler MUST populate the
    FieldRegistry from all extension manifests. Each ManifestEntityKind's
    fields array MUST be registered as valid field definitions for that
    entity kind. Field types (string, string[], reference, reference[],
    block) MUST be validated against ManifestFieldType variants. Invalid
    field types MUST produce a warning. Note: verify and gherkin are NOT
    field types — they are grammar-level constructs governed by
    supports_verify and supports_gherkin flags on ManifestEntityKind.
  """

  verify unit "fields registered per entity kind"
  verify unit "field types validated against known types"
  verify unit "invalid field type produces warning"

}

behavior populate_edge_registry_from_extensions "Populate Edge Registry From Extensions" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestEdgeType, ManifestField]

  contract """
    The compiler MUST build the complete edge type set from two sources:
    explicit edgeTypes declarations in manifests, and implicit edge types
    derived from ManifestField entries with an edge property. Both sources
    MUST be merged into a single edge type set before graph construction.
    Duplicate edge labels MUST be warned about but not rejected.
  """

  verify unit "explicit edgeTypes merged into edge set"
  verify unit "implicit edges from field mappings merged"
  verify unit "duplicate edge labels produce warning"

}

behavior validate_registered_entity_fields "Validate Registered Entity Fields" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestField, ManifestEntityKind, ManifestEdgeType, FieldRegistryEntry, KindRegistryEntry]

  contract """
    After all registries are populated, the compiler MUST cross-validate
    the registered data using only structural checks — no domain-specific
    logic. target_kind references in fields MUST resolve to registered
    entity kinds. Edge labels in field-to-edge mappings MUST resolve to
    registered edge types. Field type declarations MUST be internally
    consistent. Validation failures MUST produce warnings to allow partial
    loading, not hard errors.
  """

  verify unit "target_kind reference resolves to registered kind"
  verify unit "edge label resolves to registered edge type"
  verify unit "unresolved target_kind produces warning"
  verify unit "unresolved edge label produces warning"
  verify unit "cross-validation uses no domain-specific logic"

}

// -- Grammar Consolidation ---------------------------------------------------

behavior collapse_grammar_to_generic_entity_block "Collapse Grammar to Generic Entity Block" {
  invariants [zero_domain_knowledge_core]
  types      [SpecFile, Entity]

  contract """
    The tree-sitter grammar MUST have exactly ONE generic entity_block rule
    that parses any keyword id [title] { fields } structure. All per-keyword
    block rules that existed in the pre-zero-entity grammar MUST be collapsed
    into this single generic rule. spec_block, ref_block, use_import, and
    define_block MUST remain as separate grammar rules because they have unique
    structural syntax (ref has scheme:identifier ID format, spec has singleton
    semantics). All keyword validation MUST happen in the semantic phase, not
    the grammar.
  """

  verify unit "grammar has single generic entity_block rule"
  verify unit "no per-keyword block rules remain in grammar"
  verify unit "spec_block remains as separate grammar rule"
  verify unit "ref_block remains as separate grammar rule"
  verify unit "use_import remains as separate grammar rule"
  verify unit "define_block remains as separate grammar rule"

}

// -- Zero-Entity Bootstrap ---------------------------------------------------

behavior two_phase_parse_structural "Two-Phase Parse: Structural" {
  invariants [registry_population_before_validation, zero_domain_knowledge_core]
  types      [SpecFile, Entity]

  contract """
    In Phase 1 of the two-phase compilation, the parser MUST perform
    purely structural parsing of all .spec files. Every keyword name { }
    block MUST be parsed into a generic entity node regardless of whether
    the keyword is registered. No keyword validation MUST occur in Phase 1.
    The output MUST be a list of SpecFile ASTs with generic entity blocks.
  """

  verify unit "unknown keyword parsed into generic entity node"
  verify unit "no keyword validation in Phase 1"
  verify unit "all .spec files parsed before Phase 2"
  verify unit "parse errors collected without aborting"

}

behavior two_phase_validate_semantic "Two-Phase Validate: Semantic" {
  invariants [registry_population_before_validation, zero_domain_knowledge_core]
  types      [KindRegistryEntry, FieldRegistryEntry]
  consumes   [registries_populated, define_blocks_registered]
  // barrier: MUST wait for ALL consumed events (registries_populated AND define_blocks_registered) before executing

  contract """
    This behavior orchestrates the two-phase pipeline by invoking
    resolution and validation behaviors in sequence. It does not
    duplicate their logic.
    Phase 2 of the two-phase compilation includes resolution and semantic
    validation. First, the resolver MUST link references and build graph
    edges using the populated registries. Then, the validator MUST check
    all entity keywords against the populated KindRegistry. Unknown
    keywords MUST produce E024 diagnostics. Known keywords MUST have
    their fields validated against the FieldRegistry via
    detect_unknown_entity_fields. Phase 2 MUST NOT begin until all
    extension manifests are loaded and all registries are fully populated.
  """

  verify unit "known keyword passes semantic validation"
  verify unit "unknown keyword produces E024"
  verify unit "field validation uses FieldRegistry"
  verify unit "Phase 2 starts only after registries populated"

}

// The KeywordExtensionIndex is a static JSON file mapping known entity keywords
// to their providing extension names. It is generated from the extension registry
// at release time and shipped as a bundled data file. It does NOT require network
// access at runtime.
behavior suggest_missing_extensions "Suggest Missing Extensions" {
  invariants [zero_domain_knowledge_core]
  types      [KindRegistryEntry, UnknownKindError, KeywordExtensionIndex]

  contract """
    When an E024 (unknown entity kind) diagnostic is emitted, the help
    text MUST suggest which extension provides the unknown keyword, if known.
    The suggestion MUST use a data-driven keyword-to-extension index shipped
    as a bundled data file (not hardcoded in compiler source). The index maps
    common keywords to their providing extensions. If no mapping exists for
    the unknown keyword, the help text MUST suggest running specforge search.
  """

  verify unit "E024 for keyword in index suggests the providing extension"
  verify unit "E024 for keyword not in index suggests specforge search"
  verify unit "keyword-to-extension index is loaded from bundled data file"

}

behavior detect_unknown_entity_kinds "Detect Unknown Entity Kinds" {
  invariants [zero_domain_knowledge_core]
  types      [KindRegistryEntry, UnknownKindError]

  contract """
    After Phase 1 parsing and registry population, the compiler MUST scan
    all parsed entity blocks and check each keyword against the KindRegistry.
    Every keyword not present in the registry MUST produce an E024 diagnostic
    with the entity's source span. The diagnostic MUST include the unrecognized
    keyword and the file location.
  """

  verify unit "unregistered keyword produces E024"
  verify unit "E024 includes keyword name and source span"
  verify unit "registered keyword does not produce E024"
  verify unit "define-block keywords not checked against KindRegistry"

}

behavior graceful_degradation_without_extensions "Graceful Degradation Without Extensions" {
  invariants [zero_domain_knowledge_core]
  types      [SpecFile, KindRegistryEntry]
  consumes   [registries_populated]

  contract """
    When no extensions are installed, the compiler MUST still function. It
    MUST emit an I002 info diagnostic indicating that no extensions are
    configured. It MUST parse all .spec files structurally. The graph MUST
    be built with generic entity nodes but no kind-specific validation.
    The LSP MUST provide basic features (syntax highlighting, folding)
    without extension-driven completions.

    Reference edges are structural — they are implicit edges created by the
    parser when one entity's reference list mentions another entity's ID.
    These edges do not require registered edge types in the EdgeRegistry.
    The EdgeRegistry governs extension-defined semantic edge types only.

    In structural-only mode, the kind field of exported entity nodes MUST
    contain the raw keyword string as parsed from the .spec file.
  """

  verify unit "no extensions installed emits I002 info"
  verify unit "structural parsing works without extensions"
  verify unit "graph built with generic nodes"
  verify unit "LSP provides basic features without extensions"
  verify unit "specforge export produces valid JSON from structural-only graph"
  verify unit "generic entity nodes appear as nodes in exported graph"
  verify unit "references between generic entities produce edges"
  verify integration "specforge check with zero extensions exits cleanly with I002"

}

// -- Extension-Driven Visualization ------------------------------------------

behavior render_extension_defined_dot_shapes "Render Extension-Defined DOT Shapes" {
  invariants [zero_domain_knowledge_core]
  types      [KindRegistryEntry]

  contract """
    When rendering DOT visualization, the graph renderer MUST query the
    KindRegistry for each entity's dot_shape field. Entity nodes MUST use
    the extension-defined DOT shape (box, ellipse, diamond, hexagon, etc.).
    If no dot_shape is specified in the manifest, the default shape MUST
    be "box". The shape MUST appear in the DOT node attribute list.
  """

  verify unit "entity uses extension-defined dot_shape"
  verify unit "default shape is box when dot_shape not specified"
  verify unit "dot_shape appears in DOT node attributes"

}

behavior render_extension_defined_edge_styles "Render Extension-Defined Edge Styles" {
  invariants [zero_domain_knowledge_core]
  types      [ManifestEdgeType]

  contract """
    When rendering graph visualization, the compiler MUST query the edge
    type registry for style metadata on each edge. Edge styles MUST be
    one of: solid, dashed, or dotted. If no edge_style is defined for an
    edge type in the extension manifest, the compiler MUST default to a
    solid line. The style MUST appear in the DOT edge attribute list as
    the style property.
  """

  verify unit "edge uses extension-defined edge_style"
  verify unit "default style is solid when edge_style not specified"
  verify unit "edge_style appears in DOT edge attributes"

}

// -- Extension Manifest Consistency ------------------------------------------

behavior validate_extension_manifest_consistency "Validate Extension Manifest Consistency" {
  invariants [zero_domain_knowledge_core, registry_population_before_validation]
  types      [ManifestV2, ManifestEntityKind, ManifestEdgeType, ManifestField]

  contract """
    When an extension is loaded, the compiler MUST validate that the manifest
    is self-consistent: all target_kind references in fields MUST reference
    entity kinds declared in the same manifest or in a peer dependency.
    All edge labels in field-to-edge mappings MUST have corresponding
    edgeType declarations. This validation is domain-agnostic — it checks
    structural consistency of the manifest without knowledge of what the
    entity kinds or edge types represent.

    Self-contradictory references within the same manifest (target_kind or
    edge_type that references a name not declared in the manifest itself
    and not in any peer dependency) MUST produce an E-level error, not a
    warning. These are authoring errors in the extension manifest that
    indicate broken internal contracts. Cross-extension references to kinds
    from non-peer extensions produce W-level warnings (the kind may exist
    but the dependency is undeclared).
  """

  verify unit "target_kind referencing own manifest kind passes"
  verify unit "target_kind referencing peer dependency kind passes"
  verify unit "self-contradictory target_kind produces E-level error"
  verify unit "target_kind referencing non-peer extension kind produces W-level warning"
  verify unit "self-contradictory edge label produces E-level error"

}
