// Zero-entity core architecture types — manifest v2, registries, declarative validation
//
// Field names use snake_case per .spec DSL convention. The JSON manifest
// format uses camelCase (wasmPath, entityKinds, validationRules). The
// compiler handles serde rename during deserialization.

use "types/core"
use "types/wasm"
use "types/surface"
type ManifestV2 {
  name              string              @readonly
  version           string              @readonly
  manifest_version  integer             @readonly
  wasm_path         string
  contributes       ExtensionContributions @optional
  entity_kinds      ManifestEntityKind[] @optional
  edge_types        ManifestEdgeType[]   @optional
  validation_rules  ValidationRulePattern[] @optional
  // Extension-wide verify kinds this extension supports (e.g., ["smoke", "contract", "acceptance"])
  verify_kinds      string[]            @optional
  // Shared fields applied to ALL entity kinds in this extension (overridden by entity-kind-level fields of same name)
  fields            ManifestField[]      @optional
  // Extension-level default for incremental validation. Per-kind ManifestEntityKind.incremental overrides this value. See dispatch_incremental_validators behavior.
  incremental       boolean              @optional
  // Keywords this extension reserves from being used as entity kind names
  // (e.g., @specforge/software reserves "scenario", "given", "when", "then")
  reserved_keywords string[]             @optional
  // Wasm function name to invoke during `specforge migrate` for this extension
  migration_hook    string               @optional
  peer_dependencies PeerDependency[]     @optional
  sandbox_policy    SandboxPolicy        @optional
  query_extensions  QueryExtension[]     @optional
  host_api_version  string               @optional
  // Controls graph visibility via query_graph host function.
  // "all" (default): full graph. "own": extension + peer kinds only.
  // string[]: explicit kind list. See compute_extension_query_scope behavior.
  query_scope         string | string[]  @optional
  // Enhancement field declarations this extension adds to other extensions' entity kinds
  entity_enhancements FieldEnhancement[] @optional
  // Path to .spec template file within extension package for scaffold_starter_spec_file
  starter_template  string    @optional
  grammar_contributions GrammarContribution[] @optional
  body_parser_contributions BodyParserContribution[] @optional
  // Collector contribution declarations for test result ingestion (see register_collector_contributions)
  collector_contributions CollectorContribution[] @optional
  // Surface contributions: CLI commands, MCP tools, MCP resources (Phase 1)
  surfaces            SurfaceContributions @optional
}

// Contribution flags declaring what an extension provides.
// The compiler routes to namespaced Wasm exports based on these flags.
type ExtensionContributions {
  entities          boolean         @optional
  validators        boolean         @optional
  renderers         boolean         @optional
  providers         boolean         @optional
  collectors        boolean         @optional
  // Phase 2: extensions MAY contribute domain-specific prompts via Wasm exports (P7).
  // Dispatch mechanism deferred — boolean reserved to avoid manifest-version bump later.
  prompts           boolean         @optional
  parsers           boolean         @optional
  grammars          boolean         @optional
  body_parsers      boolean         @optional
}

type ManifestEntityKind {
  name              string              @readonly
  keyword           string              @readonly
  testable          boolean             @optional
  singleton         boolean             @optional
  supports_verify   boolean             @optional
  // Subset of extension verify_kinds allowed on this entity kind; empty = all allowed
  allowed_verify_kinds string[]         @optional
  semantic_token    string              @optional
  lsp_icon          string              @optional
  dot_shape         string              @optional
  dot_color         string              @optional
  dot_fillcolor     string              @optional
  // Fields specific to this entity kind (overrides extension-level fields of same name)
  fields            ManifestField[]     @optional
  // Whether this entity kind receives GraphDelta (true) or full Graph (false) during incremental validation
  incremental       boolean             @optional
  has_body_parser   boolean             @optional
}

type ManifestEdgeType {
  label             string              @readonly
  description       string              @optional
  source_kind       string              @optional
  target_kind       string              @optional
  // Visual style for graph rendering: "solid" | "dashed" | "dotted" (default: "solid")
  edge_style        string              @optional
  // Edge color for graph rendering (CSS/X11 color name or hex, default: "black")
  edge_color        string              @optional
  // Edge arrowhead for graph rendering: "normal" | "dot" | "diamond" | "none" (default: "normal")
  edge_arrowhead    string              @optional
}

type EntityKindConflict {
  kind_name         string              @readonly
  first_extension   string              @readonly
  second_extension  string              @readonly
  conflict_type     string              @readonly
  resolution        string              @optional
  policy_applied    string              @optional
}

type ManifestField {
  name              string              @readonly
  field_type        ManifestFieldType   @readonly
  edge              string              @optional
  target_kind       string              @optional
  file_reference    boolean             @optional
  required          boolean             @optional
}

// ManifestFieldType covers field types available in .spec DSL syntax for
// extension-declared fields. This is a superset of EnhancedFieldType (which
// is for enhancement fields only). The block_type variant corresponds to
// triple-quoted string blocks which have a dedicated grammar rule.
//
// verify is NOT a field type — it is a grammar-level construct parsed by a
// dedicated rule (parse_verify_statements). Whether an entity kind supports
// verify is declared via the supports_verify flag on ManifestEntityKind, not
// via field type registration.
type ManifestFieldType = string_type | integer_type | bool_type | enum_type
                       | string_list_type | reference_type | reference_list_type
                       | block_type

type ValidationRulePattern {
  code              string              @readonly
  severity          string              @readonly
  message_template  string
  check             ValidationPatternKind
  target_kind       string              @optional
  edge_type         string              @optional
  field             string              @optional
  constraint        FieldConstraint     @optional
}

type FieldConstraint {
  kind              string              @readonly
  pattern           string              @optional
  values            string[]            @optional
}

type ValidationPatternKind = no_incoming_edges | no_outgoing_edges
                           | missing_field_when_flag_set | field_value_constraint
                           | cycle_detection | file_exists | custom

type CustomValidationPattern {
  name              string              @readonly
  wasm_function     string              @readonly
  params            FieldMap            @optional
}

type FieldRegistryEntry {
  kind_name         string              @readonly
  field_name        string              @readonly
  field_type        ManifestFieldType   @readonly
  source_extension  string              @readonly
  edge              string              @optional
  target_kind       string              @optional
  file_reference    boolean             @optional
  required          boolean             @optional
}

type KindRegistryEntry {
  kind_name         string              @readonly
  source_extension    string              @readonly
  testable          boolean
  singleton         boolean
  supports_verify   boolean
  // Subset of extension verify_kinds allowed on this entity kind; empty = all allowed
  allowed_verify_kinds string[]         @optional
  // Orphan checking handled by extension validation_rules (e.g. no_incoming_edges pattern)
  semantic_token    string              @optional
  lsp_icon          string              @optional
  dot_shape         string              @optional
  dot_color         string              @optional
  dot_fillcolor     string              @optional
}

// SchemeRegistryEntry maps a ref scheme to the provider extension that
// handles validation for that scheme. Populated by register_provider_schemes
// from provider extension manifests. The scheme is the prefix portion of a
// ref identifier (e.g., the "gh" in "gh.issue:42").
type SchemeRegistryEntry {
  scheme            string              @readonly  @unique
  provider_alias    string              @readonly
  extension_name    string              @readonly
  supported_kinds   string[]            @optional
}

// DefineBlockConfig captures a user-defined entity type declared via a
// define block in a .spec file. Define blocks allow projects to create
// custom entity kinds without writing an extension. The custom kind is
// registered in the KindRegistry alongside extension-provided kinds.
type DefineBlockConfig {
  kind_name         string              @readonly  @unique
  id_prefix         string              @optional
  required_fields   string[]            @optional
  optional_fields   string[]            @optional
  reference_targets string[]            @optional
}

type KeywordExtensionMapping {
  keyword           string
  extension         string
  entity_kind       string
}

type EdgeRegistryEntry {
  label             string    @readonly
  source_kind       string    @optional
  target_kind       string    @optional
  source_extension  string    @readonly
  edge_style        string    @optional
  edge_color        string    @optional
  edge_arrowhead    string    @optional
}

type KeywordExtensionIndex {
  _tag              "KeywordExtensionIndex"  @literal
  entries           KeywordExtensionMapping[]
}
