// @specforge/product extension manifest declaration

use "extensions/product/types"
use "types/zero-entity-core"

behavior pe_declare_manifest "Declare @specforge/product Manifest" {
  category command
  types    [ExtensionDeclaration, EntityKindDescriptor, EdgeTypeDescriptor]
  contract """
    The @specforge/product extension MUST declare itself with name
    "@specforge/product". Its declaration MUST declare
    exactly 9 entity kinds (journey, deliverable, milestone, module,
    term, feature, persona, channel, release), 20 edge types
    (FeatureDependsOn, FeatureRelatesTo, JourneyExercisesFeature,
    JourneyTargetsPersona, JourneyUsesChannel, DeliverableSupportsJourney,
    DeliverableContainsModule, DeliverableTrackedByMilestone,
    DeliverableDependsOn, MilestoneDeliversFeature, MilestoneScopesModule,
    MilestoneDependsOn, ModuleContainsFeature, ModuleDependsOn,
    TermReferencesRelatedTerm, TermBelongsToModule,
    ReleaseIncludesDeliverable, ReleaseCompletesMilestone, ReleaseDependsOn,
    PersonaPrioritizesFeature), and 61 validation rules. Diagnostic codes:
    E007, E015, E052, W041-W046, W049, W057, W077-W080, W083-W085, W092,
    W093, W095, I010, I046-I048, I050, I053-I055, I057, I059-I062,
    I066-I070, I080-I083, I086, I087, I089. Feature, milestone,
    deliverable, persona, channel and release MUST declare status as their
    lifecycle field, the state the build cache records for the transition
    rules (W087-W091, W094; ADR 0009).
  """
  requires {
    supported_protocol   "the handshake's protocol major version is the host's"
    valid_extension_name "name == '@specforge/product'"
  }
  ensures {
    nine_entity_kinds            "entityKinds.length == 9"
    twenty_edge_types            "edgeTypes.length == 20"
    all_kinds_named              "entityKinds contains journey, deliverable, milestone, module, term, feature, persona, channel, release"
    all_edges_named              "edgeTypes contains FeatureDependsOn, FeatureRelatesTo, JourneyExercisesFeature, JourneyTargetsPersona, JourneyUsesChannel, DeliverableSupportsJourney, DeliverableContainsModule, DeliverableTrackedByMilestone, DeliverableDependsOn, MilestoneDeliversFeature, MilestoneScopesModule, MilestoneDependsOn, ModuleContainsFeature, ModuleDependsOn, TermReferencesRelatedTerm, TermBelongsToModule, ReleaseIncludesDeliverable, ReleaseCompletesMilestone, ReleaseDependsOn, PersonaPrioritizesFeature"
    contributes_entities         "contributes.entities is true"
    contributes_validators       "contributes.validators is true"
    contributes_no_renderers     "contributes.renderers is false — product provides no rendering"
    contributes_no_providers     "contributes.providers is false — product has no external data providers"
    contributes_no_collectors    "contributes.collectors is false — product does not collect test results"
    contributes_no_prompts       "contributes.prompts is false — product has no agent prompts"
    contributes_no_parsers       "contributes.parsers is false — product uses default parser"
    contributes_no_grammars      "contributes.grammars is false — product uses default grammar"
    contributes_no_body_parsers  "contributes.body_parsers is false — product uses default body parsing"
    no_entity_enhancements       "entity_enhancements is empty — product DECLARES no enhancements on other extensions' entity kinds. However, product IS the target of enhancements from peer extensions (e.g., @specforge/software adds a behaviors field (MilestoneIncludesBehavior edges) to product's milestone kind via its own entity_enhancements). The directionality is: software enhances product, not the reverse."
    no_verify_kinds              "verify_kinds is empty and every product kind has supportsVerify=false — product entities declare no verify statements"
    no_peer_deps                 "peer_dependencies is empty — product is standalone and requires no other extensions. Peer extensions like @specforge/software declare product as THEIR peer_dependency to contribute entity_enhancements (e.g., a behaviors field on milestone) and cross-extension edges (e.g., BehaviorImplementsFeature: behavior→feature)."
    no_migration_hook            "migration_hook is null — intentionally absent in v1 (no prior version)"
    no_passes                    "passes is empty — product declares no custom compiler passes"
    no_feature_flags             "feature_flags is empty — product declares no feature flags"
    no_sandbox_policy            "sandbox_policy is null — product declares no sandbox policy"
    starter_tmpl_declared        "starter_template is the product starter in src/starter.spec"
    surfaces_declared            "surfaces declares the 40 specforge product CLI commands (cmd__product_* exports, auto-promoted to MCP tools) and no explicit MCP tools or resources"
    ext_short_declared           "ext_short is 'product' for MCP tool naming (specforge.product.{cmd_id})"
    lifecycle_fields_declared    "feature, milestone, deliverable, persona, channel and release declare lifecycle_field status"
    fields_declared              "fields declares shared fields (tags: string[] @optional) applied to all 9 entity kinds"
    no_grammar_contributions     "grammar_contributions is empty — product uses default grammar"
    no_body_parser_contributions "body_parser_contributions is empty — product uses default body parsing"
    no_collector_contributions   "collector_contributions is empty — product does not collect test results"
  }
  features [pe_core_entity_kinds]
  verify unit "manifest name is @specforge/product"
  verify unit "manifest declares exactly 9 entity kinds"
  verify unit "manifest declares exactly 20 edge types"
  verify unit "the handshake's protocol major is the host's"
  verify unit "contributes declares entities=true and validators=true"
  verify unit "contributes false flags: renderers, providers, collectors, prompts, parsers, grammars, body_parsers"
  verify unit "entity_enhancements is empty"
  verify unit "no product kind declares verify kinds or supportsVerify"
  verify unit "peer_dependencies is empty"
  verify unit "migration_hook is null"
  verify unit "passes is empty"
  verify unit "feature_flags is empty"
  verify unit "sandbox_policy is null"
  verify unit "starter_template is the product starter in src/starter.spec"
  verify unit "surfaces declares the product commands"
  verify unit "ext_short is product"
  verify unit "fields declares shared tags field"
  verify unit "the six lifecycle kinds declare status as their lifecycle field"
  verify unit "grammar_contributions is empty"
  verify unit "body_parser_contributions is empty"
  verify unit "collector_contributions is empty"
}

invariant pe_manifest_nine_entity_kinds "Nine Entity Kinds" {
  guarantee """
    The @specforge/product manifest MUST declare exactly 9 entity kinds:
    journey, deliverable, milestone, module, term, feature, persona,
    channel, release. Feature is a domain-neutral product concept. Persona
    and channel are first-class entity kinds. Release coordinates
    multi-deliverable shipping.
  """
  risk      high
  verify property "manifest entityKinds array has exactly 9 entries"
}

invariant pe_manifest_twenty_edge_types "Twenty Edge Types" {
  guarantee """
    The @specforge/product manifest MUST declare exactly 20 edge types:
    FeatureDependsOn, FeatureRelatesTo, JourneyExercisesFeature,
    JourneyTargetsPersona, JourneyUsesChannel, DeliverableSupportsJourney,
    DeliverableContainsModule, DeliverableTrackedByMilestone,
    DeliverableDependsOn, MilestoneDeliversFeature, MilestoneScopesModule,
    MilestoneDependsOn, ModuleContainsFeature, ModuleDependsOn,
    TermReferencesRelatedTerm, TermBelongsToModule,
    ReleaseIncludesDeliverable, ReleaseCompletesMilestone, ReleaseDependsOn,
    PersonaPrioritizesFeature. These edges model relationships between the
    9 entity kinds.
  """
  risk      medium
  verify property "manifest edgeTypes array has exactly 20 entries"
}
