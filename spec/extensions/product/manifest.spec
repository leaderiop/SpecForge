// @specforge/product extension manifest declaration

use "extensions/product/types"
use "types/zero-entity-core"

behavior pe_declare_manifest "Declare @specforge/product Manifest" {
  category command
  types    [ManifestV2, ManifestEntityKind, ManifestEdgeType]
  contract """
    The @specforge/product extension MUST declare a v2 manifest with name
    "@specforge/product", manifestVersion 2. The manifest MUST declare
    exactly 9 entity kinds (journey, deliverable, milestone, module,
    term, feature, persona, channel, release), 16 edge types (JourneyFeature,
    DeliverableJourney, ModuleDependsOn, MilestoneFeature,
    DeliverableModule, ModuleFeature, FeatureDependsOn,
    JourneyPersona, JourneyChannel, MilestoneModule, TermSeeAlso,
    MilestoneDependsOn, DeliverableMilestone, DeliverableDependsOn,
    ReleaseDeliverable, ReleaseMilestone),
    and all associated validation rules. Diagnostic codes: E007-E009,
    E015-E016, W041-W046, W049, W057, W075-W095, I010, I046-I097.
  """
  requires {
    valid_manifest_version   "manifestVersion == 2"
    valid_extension_name     "name == '@specforge/product'"
  }
  ensures  {
    nine_entity_kinds        "entityKinds.length == 9"
    sixteen_edge_types       "edgeTypes.length == 16"
    all_kinds_named          "entityKinds contains journey, deliverable, milestone, module, term, feature, persona, channel, release"
    all_edges_named          "edgeTypes contains JourneyFeature, DeliverableJourney, ModuleDependsOn, MilestoneFeature, DeliverableModule, ModuleFeature, FeatureDependsOn, JourneyPersona, JourneyChannel, MilestoneModule, TermSeeAlso, MilestoneDependsOn, DeliverableMilestone, DeliverableDependsOn, ReleaseDeliverable, ReleaseMilestone"
    contributes_entities     "contributes.entities is true"
    contributes_validators   "contributes.validators is true"
    contributes_no_renderers "contributes.renderers is false — product provides no rendering"
    contributes_no_providers "contributes.providers is false — product has no external data providers"
    contributes_no_collectors "contributes.collectors is false — product does not collect test results"
    contributes_no_prompts   "contributes.prompts is false — product has no agent prompts"
    contributes_no_parsers   "contributes.parsers is false — product uses default parser"
    contributes_no_grammars  "contributes.grammars is false — product uses default grammar"
    contributes_no_body_parsers "contributes.body_parsers is false — product uses default body parsing"
    no_entity_enhancements   "entity_enhancements is empty — product DECLARES no enhancements on other extensions' entity kinds. However, product IS the target of enhancements from peer extensions (e.g., @specforge/software adds MilestoneBehavior fields to product's milestone kind via its own entity_enhancements). The directionality is: software enhances product, not the reverse."
    acceptance_verify_kind   "verify_kinds declares ['acceptance'] — feature, deliverable, and milestone support verify acceptance annotations linking to external acceptance test files"
    no_query_extensions      "query_extensions is empty — product uses standard graph traversal APIs (getIncomingEdges, getOutgoingEdges, getNodesByKind, detectCycles) and declares no custom query operators"
    no_peer_deps             "peer_dependencies is empty — product is standalone and requires no other extensions. Peer extensions like @specforge/software declare product as THEIR peer_dependency to contribute entity_enhancements (e.g., MilestoneBehavior on milestone) and cross-extension edges (e.g., Implements: behavior→feature)."
    no_migration_hook        "migration_hook is null — intentionally absent in v1 (no prior version)"
    no_passes                "passes is empty — product declares no custom compiler passes"
    no_feature_flags         "feature_flags is empty — product declares no feature flags"
    incremental_default      "incremental is true — product supports incremental compilation"
    sandbox_restricted       "sandbox_policy declares network_access=false, file_system_access=read-only, max_memory_mb=256, max_execution_ms=5000"
    sandbox_no_network       "sandbox_policy.network_access is false — product queries are pure graph traversals"
    host_api_declared        "host_api_version is 1.0.0"
    starter_tmpl_declared    "starter_template is templates/feature.spec"
    surfaces_declared        "surfaces declares CLI commands and MCP resources; every ProductQueryPort method has a corresponding CLI command and MCP resource"
    ext_short_declared       "ext_short is 'product' for MCP tool naming (specforge.product.{cmd_id})"
    no_reserved_keywords     "reserved_keywords is empty — product has no keywords to reserve"
    query_scope_own          "query_scope is 'own' — product queries only its own and peer entity kinds"
    wasm_path_declared       "wasm_path points to the product extension Wasm binary"
    fields_declared          "fields declares shared fields (tags: string[] @optional) applied to all 9 entity kinds"
    no_grammar_contributions "grammar_contributions is empty — product uses default grammar"
    no_body_parser_contributions "body_parser_contributions is empty — product uses default body parsing"
    no_collector_contributions "collector_contributions is empty — product does not collect test results"
  }

  features [pe_core_entity_kinds]

  verify unit "manifest name is @specforge/product"
  verify unit "manifest declares exactly 9 entity kinds"
  verify unit "manifest declares exactly 16 edge types"
  verify unit "manifest version is 2"
  verify unit "contributes declares entities=true and validators=true"
  verify unit "contributes false flags: renderers, providers, collectors, prompts, parsers, grammars, body_parsers"
  verify unit "entity_enhancements is empty"
  verify unit "verify_kinds declares acceptance"
  verify unit "query_extensions is empty"
  verify unit "peer_dependencies is empty"
  verify unit "migration_hook is null"
  verify unit "passes is empty"
  verify unit "feature_flags is empty"
  verify unit "incremental is true"
  verify unit "sandbox_policy declares no network access and read-only filesystem"
  verify unit "host_api_version is 1.0.0"
  verify unit "starter_template is templates/feature.spec"
  verify unit "every ProductQueryPort method has a CLI command and MCP resource"
  verify unit "ext_short is product"
  verify unit "reserved_keywords is empty"
  verify unit "query_scope is own"
  verify unit "wasm_path points to valid binary"
  verify unit "fields declares shared tags field"
  verify unit "grammar_contributions is empty"
  verify unit "body_parser_contributions is empty"
  verify unit "collector_contributions is empty"
}

invariant pe_manifest_nine_entity_kinds "Nine Entity Kinds" {
  guarantee   """
    The @specforge/product manifest MUST declare exactly 9 entity kinds:
    journey, deliverable, milestone, module, term, feature, persona,
    channel, release. Feature is a domain-neutral product concept. Persona
    and channel are first-class entity kinds. Release coordinates
    multi-deliverable shipping.
  """
  risk        high

  verify property "manifest entityKinds array has exactly 9 entries"
}

invariant pe_manifest_sixteen_edge_types "Sixteen Edge Types" {
  guarantee   """
    The @specforge/product manifest MUST declare exactly 16 edge types:
    JourneyFeature, DeliverableJourney, ModuleDependsOn,
    MilestoneFeature, DeliverableModule, ModuleFeature,
    FeatureDependsOn, JourneyPersona, JourneyChannel, MilestoneModule,
    TermSeeAlso, MilestoneDependsOn, DeliverableMilestone,
    DeliverableDependsOn, ReleaseDeliverable, ReleaseMilestone.
    These edges model relationships between the 9 entity kinds.
  """
  risk        medium

  verify property "manifest edgeTypes array has exactly 16 entries"
}
