// Zero-entity core architecture types — manifest v2, registries, declarative validation
//
// Field names use snake_case per .spec DSL convention. The JSON manifest
// format uses camelCase (wasmPath, entityKinds, validationRules). The
// compiler handles serde rename during deserialization.

use "types/core"
use "types/surface"
use "types/wasm"

type ManifestV2 {
  name                    string                  @readonly
  version                 string                  @readonly
  manifest_version        integer                 @readonly
  wasm_path               string
  contributes             ExtensionContributions  @optional
  entity_kinds            ManifestEntityKind[]    @optional
  edge_types              ManifestEdgeType[]      @optional
  validation_rules        ValidationRulePattern[] @optional
  // Extension-wide verify kinds this extension supports (e.g., ["smoke", "contract", "acceptance"])
  verify_kinds            string[]                @optional
  // Shared fields applied to ALL entity kinds in this extension (overridden by entity-kind-level fields of same name)
  fields                  ManifestField[]         @optional
  // Extension-level default for incremental validation. Per-kind ManifestEntityKind.incremental overrides this value. See dispatch_incremental_validators behavior.
  incremental             boolean                 @optional
  // Keywords this extension reserves from being used as entity kind names
  // (e.g., @specforge/software reserves "scenario", "given", "when", "then")
  reserved_keywords       string[]                @optional
  // Wasm function name to invoke during `specforge migrate` for this extension
  migration_hook          string                  @optional
  peer_dependencies       PeerDependency[]        @optional
  sandbox_policy          SandboxPolicy           @optional
  query_extensions        QueryExtension[]        @optional
  host_api_version        string                  @optional
  // Controls graph visibility via query_graph host function.
  // "all" (default): full graph. "own": extension + peer kinds only.
  // string[]: explicit kind list. See compute_extension_query_scope behavior.
  query_scope             string | string[]       @optional
  // Enhancement field declarations this extension adds to other extensions' entity kinds
  entity_enhancements     FieldEnhancement[]      @optional
  // Text of the starter .spec file scaffold_starter_spec_file writes; {project} stands for the project id
  starter_template        string                  @optional
  // Collector contribution declarations for test result ingestion (see register_collector_contributions)
  collector_contributions CollectorContribution[] @optional
  // Surface contributions: CLI commands, MCP tools, MCP resources (Phase 1)
  surfaces                SurfaceContributions    @optional
  // The colour diagrams (model, outline) draw the extension in; grey when absent
  theme_color             string                  @optional
  verify unit "ManifestV2 schema is valid"
}

// Contribution flags declaring what an extension provides.
// The compiler routes to namespaced Wasm exports based on these flags.
type ExtensionContributions {
  entities     boolean @optional
  validators   boolean @optional
  renderers    boolean @optional
  providers    boolean @optional
  collectors   boolean @optional
  // Phase 2: extensions MAY contribute domain-specific prompts via Wasm exports (P7).
  // Dispatch mechanism deferred — boolean reserved to avoid manifest-version bump later.
  prompts      boolean @optional
  parsers      boolean @optional
  grammars     boolean @optional
  body_parsers boolean @optional
  verify unit "ExtensionContributions schema is valid"
}

type ManifestEntityKind {
  name                 string          @readonly
  keyword              string          @readonly
  testable             boolean         @optional
  singleton            boolean         @optional
  supports_verify      boolean         @optional
  // Subset of extension verify_kinds allowed on this entity kind; empty = all allowed
  allowed_verify_kinds string[]        @optional
  semantic_token       string          @optional
  lsp_icon             string          @optional
  dot_shape            string          @optional
  dot_color            string          @optional
  dot_fillcolor        string          @optional
  // Fields specific to this entity kind (overrides extension-level fields of same name)
  fields               ManifestField[] @optional
  // Whether this entity kind receives GraphDelta (true) or full Graph (false) during incremental validation
  incremental          boolean         @optional
  has_body_parser      boolean         @optional
  // Reference fields that target this kind are contract obligations (A010)
  contract_target      boolean         @optional
  // Its entity ids name types: custom validators receive them as declared_types
  declares_types       boolean         @optional
  /// The one field (of those the kind declares) holding its entities'
  /// lifecycle state; the build cache records its value (ADR 0009).
  lifecycle_field      string          @optional
  verify unit "ManifestEntityKind schema is valid"
}

type ManifestEdgeType {
  label          string @readonly
  description    string @optional
  source_kind    string @optional
  target_kind    string @optional
  // Visual style for graph rendering: "solid" | "dashed" | "dotted" (default: "solid")
  edge_style     string @optional
  // Edge color for graph rendering (CSS/X11 color name or hex, default: "black")
  edge_color     string @optional
  // Edge arrowhead for graph rendering: "normal" | "dot" | "diamond" | "none" (default: "normal")
  edge_arrowhead string @optional
  verify unit "ManifestEdgeType schema is valid"
}

type EntityKindConflict {
  kind_name        string @readonly
  first_extension  string @readonly
  second_extension string @readonly
  conflict_type    string @readonly
  resolution       string @optional
  policy_applied   string @optional
  verify unit "EntityKindConflict schema is valid"
}

type ManifestField {
  name                string                 @readonly
  field_type          ManifestFieldType      @readonly
  edge                string                 @optional
  target_kind         string                 @optional
  file_reference      boolean                @optional
  required            boolean                @optional
  /// The field states what the entity promises (a behavior's contract, an
  /// invariant's guarantee), as opposed to prose; token-optimized exports keep it.
  normative           boolean                @optional
  /// Set on an entity (true, or a non-empty value), the entity owes no
  /// obligations of its own: W004, coverage and stats leave it out.
  exempts_obligations boolean                @optional
  /// The context export carries the field at the node's top level (a
  /// behavior's contract, a feature's status).
  headline            boolean                @optional
  /// The host fills the field's edges from type names the entity writes
  /// elsewhere (behavior link_derived_references).
  derived_from        DerivedReferenceSource @optional
  /// What the prove pass reads the field as: a bound it assumes or a claim
  /// that must follow from the bounds (ADR 0009). No role: not read.
  proof_role          ProofRole              @optional
  verify unit "ManifestField schema is valid"
}

// A field's role in the prove pass: a bound is assumed (bounds must be
// consistent, E046); a claim must follow from the bounds (W139 when not).
type ProofRole = bound | claim

// Where a derived reference field takes its targets from: the type names in
// the entity's type-syntax field values, or in its method signatures.
type DerivedReferenceSource = type_expressions | method_signatures

// ManifestFieldType covers field types available in .spec DSL syntax for
// extension-declared fields. This is a superset of EnhancedFieldType (which
// is for enhancement fields only). The block_type variant corresponds to
// triple-quoted string blocks which have a dedicated grammar rule.
//
// verify is NOT a field type — it is a grammar-level construct parsed by a
// dedicated rule (parse_verify_statements). Whether an entity kind supports
// verify is declared via the supports_verify flag on ManifestEntityKind, not
// via field type registration.
//
// On the wire the names drop the _type suffix (string, bool, block, ...;
// FieldType in specforge-protocol-types); the suffixed spellings and
// "boolean" are read as aliases.
type ManifestFieldType = string_type
  | integer_type
  | bool_type
  | enum_type
  | string_list_type
  | reference_type
  | reference_list_type
  | block_type

type ValidationRulePattern {
  code             string          @readonly
  severity         string          @readonly
  message_template string
  check            ValidationPatternKind
  target_kind      string          @optional
  edge_type        string          @optional
  field            string          @optional
  constraint       FieldConstraint @optional
  verify unit "ValidationRulePattern schema is valid"
}

type FieldConstraint {
  kind    ConstraintKind @readonly
  pattern string         @optional
  values  string[]       @optional
  verify unit "FieldConstraint schema is valid"
}

// How a constraint reads its pattern and values (ConstraintKind in
// specforge-protocol-types): non_empty, one_of and matches for
// field_value_constraint, when_field_equals for conditional_field_required,
// one_of for verify_kind_allowlist.
type ConstraintKind = non_empty | one_of | matches | when_field_equals

// The check kinds of the extension vocabulary (CheckKind in
// specforge-protocol-types — the SDK writes these names, the registry build
// reads them; it also reads the older SDK spellings missing_field,
// field_constraint, cycle and conditional_required).
type ValidationPatternKind = no_incoming_edges
  | no_outgoing_edges
  | no_edges
  | missing_field_when_flag_set
  | missing_required_field
  | conditional_field_required
  | field_value_constraint
  | cycle_detection
  | file_exists
  | verify_kind_allowlist
  | no_verify_statements
  | custom

type CustomValidationPattern {
  name          string   @readonly
  wasm_function string   @readonly
  params        FieldMap @optional
  verify unit "CustomValidationPattern schema is valid"
}

type FieldRegistryEntry {
  kind_name           string                 @readonly
  field_name          string                 @readonly
  field_type          ManifestFieldType      @readonly
  source_extension    string                 @readonly
  edge                string                 @optional
  target_kind         string                 @optional
  file_reference      boolean                @optional
  required            boolean                @optional
  normative           boolean                @optional
  exempts_obligations boolean                @optional
  headline            boolean                @optional
  derived_from        DerivedReferenceSource @optional
  /// The prove-pass role its manifest declares; any other value is refused.
  proof_role          ProofRole              @optional
  verify unit "FieldRegistryEntry schema is valid"
}

type KindRegistryEntry {
  kind_name            string   @readonly
  source_extension     string   @readonly
  testable             boolean
  singleton            boolean
  supports_verify      boolean
  // Subset of extension verify_kinds allowed on this entity kind; empty = all allowed
  allowed_verify_kinds string[] @optional
  // Orphan checking handled by extension validation_rules (e.g. no_incoming_edges pattern)
  semantic_token       string   @optional
  lsp_icon             string   @optional
  dot_shape            string   @optional
  dot_color            string   @optional
  dot_fillcolor        string   @optional
  contract_target      boolean  @optional
  declares_types       boolean  @optional
  /// The kind's lifecycle field; a name the kind does not declare is refused.
  lifecycle_field      string   @optional
  verify unit "KindRegistryEntry schema is valid"
}

// SchemeRegistryEntry maps a ref scheme to the provider extension that
// handles validation for that scheme. Populated by register_provider_schemes
// from provider extension manifests. The scheme is the prefix portion of a
// ref identifier (e.g., the "gh" in "gh.issue:42").
type SchemeRegistryEntry {
  scheme          string   @readonly @unique
  provider_alias  string   @readonly
  extension_name  string   @readonly
  supported_kinds string[] @optional
  verify unit "SchemeRegistryEntry schema is valid"
}

type KeywordExtensionMapping {
  keyword     string
  extension   string
  entity_kind string
  verify unit "KeywordExtensionMapping schema is valid"
}

type EdgeRegistryEntry {
  label            string @readonly
  source_kind      string @optional
  target_kind      string @optional
  source_extension string @readonly
  edge_style       string @optional
  edge_color       string @optional
  edge_arrowhead   string @optional
  verify unit "EdgeRegistryEntry schema is valid"
}

type KeywordExtensionIndex {
  _tag    "KeywordExtensionIndex" @literal
  entries KeywordExtensionMapping[]
  verify unit "KeywordExtensionIndex schema is valid"
}

type CompilerPassDeclaration {
  name          string @readonly
  run_after     string @optional
  wasm_function string @readonly
  description   string @optional
  verify unit "CompilerPassDeclaration schema is valid"
}

type FeatureFlagDeclaration {
  name          string @readonly
  default_value boolean
  description   string @optional
  verify unit "FeatureFlagDeclaration schema is valid"
}
