// @specforge/product — Rendering, cross-extension integration, surface declarations, and migration behaviors

use "extensions/product/invariants"
use "extensions/product/types"
use "product/features"
use "types/zero-entity-core"
use "types/diagnostics"

behavior pe_declare_surface_contributions "Declare Surface Contributions" {
  category   command
  types      [ManifestV2, ProductListFilter, ProductSurfaceResponse, ProductSurfaceError, ProductSurfaceFailedPayload, ProductSurfaceOperationPayload, SurfaceResponseStatus]
  produces  [pe_cli_command_executed, pe_surface_error]
  contract   """
    The @specforge/product extension MUST declare surface contributions
    in its manifest for CLI commands, MCP tools, and MCP resources.

    CLI commands (44 total):
    - 9 entity listing commands: product:features, product:journeys,
      product:deliverables, product:milestones, product:modules,
      product:terms, product:personas, product:channels, product:releases (v1.1)
    - 22 query commands: product:milestone-completion,
      product:journey-coverage, product:feature-ordering,
      product:milestone-timeline, product:milestone-deliverables,
      product:module-features, product:deliverable-traceability,
      product:feature-deliverables, product:feature-milestones,
      product:persona-journeys, product:channel-journeys,
      product:module-deliverables, product:term-graph,
      product:deliverable-completion, product:persona-channels,
      product:journey-deliverables, product:feature-dependents,
      product:deliverable-dependents, product:deliverable-priority,
      product:persona-features, product:feature-impact,
      product:milestone-velocity
    - 2 analytics commands: product:unscheduled-features,
      product:coverage-matrix, product:critical-path
    - 8 v1.1 query commands: product:owner-workload,
      product:weighted-milestone-completion, product:release-deliverables,
      product:release-completion, product:feature-overlap,
      product:channel-features, product:deliverable-personas,
      product:release-milestones
    - 2 term analytics commands: product:term-clusters, product:term-density
    - 2 module analytics commands: product:module-depth, product:module-coupling
    - 1 channel analytics command: product:channel-coverage-matrix

    Each CLI command is auto-promoted to an MCP tool. MCP resources expose
    read-only query-port methods via specforge:// URIs. Each surface has a
    typed input/output schema defined in surfaces-cli.spec and surfaces-mcp.spec.
    List commands accept ProductListFilter and return per-kind list result
    types with pagination. Query commands accept typed inputs and return
    query payload types. MCP resources return ProductSurfaceResponse
    envelopes. All surfaces use consistent error handling via
    ProductSurfaceError with codes: ENTITY_NOT_FOUND, GRAPH_NOT_READY,
    INVALID_INPUT.
  """
  ensures    {
    cmd_milestones           "surfaces.commands contains product:milestones (cmd__product_milestones)"
    cmd_features             "surfaces.commands contains product:features (cmd__product_features)"
    cmd_journeys             "surfaces.commands contains product:journeys (cmd__product_journeys)"
    cmd_deliverables         "surfaces.commands contains product:deliverables (cmd__product_deliverables)"
    cmd_modules              "surfaces.commands contains product:modules (cmd__product_modules)"
    cmd_terms                "surfaces.commands contains product:terms (cmd__product_terms)"
    cmd_personas             "surfaces.commands contains product:personas (cmd__product_personas)"
    cmd_channels             "surfaces.commands contains product:channels (cmd__product_channels)"
    cmd_milestone_completion "surfaces.commands contains product:milestone-completion (cmd__product_milestone_completion)"
    cmd_journey_coverage     "surfaces.commands contains product:journey-coverage (cmd__product_journey_coverage)"
    cmd_feature_ordering     "surfaces.commands contains product:feature-ordering (cmd__product_feature_ordering)"
    cmd_milestone_timeline   "surfaces.commands contains product:milestone-timeline (cmd__product_milestone_timeline)"
    cmd_milestone_deliverables "surfaces.commands contains product:milestone-deliverables (cmd__product_milestone_deliverables)"
    cmd_module_features      "surfaces.commands contains product:module-features (cmd__product_module_features)"
    cmd_deliverable_traceability "surfaces.commands contains product:deliverable-traceability (cmd__product_deliverable_traceability)"
    cmd_feature_deliverables "surfaces.commands contains product:feature-deliverables (cmd__product_feature_deliverables)"
    cmd_feature_milestones   "surfaces.commands contains product:feature-milestones (cmd__product_feature_milestones)"
    cmd_persona_journeys     "surfaces.commands contains product:persona-journeys (cmd__product_persona_journeys)"
    cmd_channel_journeys     "surfaces.commands contains product:channel-journeys (cmd__product_channel_journeys)"
    cmd_module_deliverables  "surfaces.commands contains product:module-deliverables (cmd__product_module_deliverables)"
    cmd_term_graph           "surfaces.commands contains product:term-graph (cmd__product_term_graph)"
    cmd_deliverable_completion "surfaces.commands contains product:deliverable-completion (cmd__product_deliverable_completion)"
    cmd_persona_channels     "surfaces.commands contains product:persona-channels (cmd__product_persona_channels)"
    cmd_journey_deliverables "surfaces.commands contains product:journey-deliverables (cmd__product_journey_deliverables)"
    cmd_feature_dependents   "surfaces.commands contains product:feature-dependents (cmd__product_feature_dependents)"
    cmd_deliverable_dependents "surfaces.commands contains product:deliverable-dependents (cmd__product_deliverable_dependents)"
    cmd_deliverable_priority "surfaces.commands contains product:deliverable-priority (cmd__product_deliverable_priority)"
    cmd_persona_features     "surfaces.commands contains product:persona-features (cmd__product_persona_features)"
    cmd_feature_impact       "surfaces.commands contains product:feature-impact (cmd__product_feature_impact)"
    cmd_milestone_velocity   "surfaces.commands contains product:milestone-velocity (cmd__product_milestone_velocity)"
    cmd_deliverable_personas "surfaces.commands contains product:deliverable-personas (cmd__product_deliverable_personas)"
    cmd_feature_overlap      "surfaces.commands contains product:feature-overlap (cmd__product_feature_overlap)"
    cmd_channel_features     "surfaces.commands contains product:channel-features (cmd__product_channel_features)"
    cmd_term_clusters        "surfaces.commands contains product:term-clusters (cmd__product_term_clusters)"
    cmd_term_density         "surfaces.commands contains product:term-density (cmd__product_term_density)"
    cmd_module_depth         "surfaces.commands contains product:module-depth (cmd__product_module_depth)"
    cmd_module_coupling      "surfaces.commands contains product:module-coupling (cmd__product_module_coupling)"
    cmd_channel_coverage_matrix "surfaces.commands contains product:channel-coverage-matrix (cmd__product_channel_coverage_matrix)"
    cmd_releases             "surfaces.commands contains product:releases (cmd__product_releases)"
    cmd_unscheduled_features "surfaces.commands contains product:unscheduled-features (cmd__product_unscheduled_features)"
    cmd_coverage_matrix      "surfaces.commands contains product:coverage-matrix (cmd__product_coverage_matrix)"
    cmd_critical_path        "surfaces.commands contains product:critical-path (cmd__product_critical_path)"
    cmd_owner_workload       "surfaces.commands contains product:owner-workload (cmd__product_owner_workload)"
    cmd_weighted_completion  "surfaces.commands contains product:weighted-milestone-completion (cmd__product_weighted_milestone_completion)"
    cmd_release_deliverables "surfaces.commands contains product:release-deliverables (cmd__product_release_deliverables)"
    cmd_release_completion   "surfaces.commands contains product:release-completion (cmd__product_release_completion)"
    cmd_release_milestones   "surfaces.commands contains product:release-milestones (cmd__product_release_milestones)"
    mcp_auto_promotion       "each CLI command is auto-promoted to an MCP tool via specforge.product.{cmd_id}"
    res_deliverable_traceability "surfaces.resources contains specforge://product/deliverable-traceability/{deliverableId}"
    res_feature_deliverables    "surfaces.resources contains specforge://product/feature-deliverables/{featureId}"
    res_feature_milestones      "surfaces.resources contains specforge://product/feature-milestones/{featureId}"
    res_persona_journeys        "surfaces.resources contains specforge://product/persona-journeys/{personaId}"
    res_channel_journeys        "surfaces.resources contains specforge://product/channel-journeys/{channelId}"
    res_module_deliverables     "surfaces.resources contains specforge://product/module-deliverables/{moduleId}"
    res_term_graph              "surfaces.resources contains specforge://product/term-graph/{termId}"
    res_milestone_deliverables  "surfaces.resources contains specforge://product/milestone-deliverables/{milestoneId}"
    res_module_features         "surfaces.resources contains specforge://product/module-features/{moduleId}"
    res_deliverable_completion  "surfaces.resources contains specforge://product/deliverable-completion/{deliverableId}"
    res_persona_channels        "surfaces.resources contains specforge://product/persona-channels/{personaId}"
    res_journey_deliverables    "surfaces.resources contains specforge://product/journey-deliverables/{journeyId}"
    res_feature_dependents      "surfaces.resources contains specforge://product/feature-dependents/{featureId}"
    res_deliverable_dependents  "surfaces.resources contains specforge://product/deliverable-dependents/{deliverableId}"
    res_deliverable_priority    "surfaces.resources contains specforge://product/deliverable-priority/{deliverableId}"
    res_persona_features        "surfaces.resources contains specforge://product/persona-features/{personaId}"
    res_feature_impact          "surfaces.resources contains specforge://product/feature-impact/{featureId}"
    res_milestone_velocity      "surfaces.resources contains specforge://product/milestone-velocity/{milestoneId}"
    res_deliverable_personas    "surfaces.resources contains specforge://product/deliverable-personas/{deliverableId}"
    res_unscheduled_features    "surfaces.resources contains specforge://product/unscheduled-features"
    res_feature_overlap         "surfaces.resources contains specforge://product/feature-overlap"
    res_persona_coverage_matrix "surfaces.resources contains specforge://product/persona-coverage-matrix"
    res_critical_path           "surfaces.resources contains specforge://product/critical-path"
    res_channel_features        "surfaces.resources contains specforge://product/channel-features/{channelId}"
    res_term_clusters           "surfaces.resources contains specforge://product/term-clusters"
    res_term_density            "surfaces.resources contains specforge://product/term-density"
    res_module_depth            "surfaces.resources contains specforge://product/module-depth/{moduleId}"
    res_module_coupling         "surfaces.resources contains specforge://product/module-coupling"
    res_channel_coverage_matrix "surfaces.resources contains specforge://product/channel-coverage-matrix"
    res_milestone_completion    "surfaces.resources contains specforge://product/milestone-completion/{milestoneId}"
    res_journey_coverage        "surfaces.resources contains specforge://product/journey-coverage/{journeyId}"
    res_feature_ordering        "surfaces.resources contains specforge://product/feature-ordering"
    res_milestone_timeline      "surfaces.resources contains specforge://product/milestone-timeline"
    res_releases                "surfaces.resources contains specforge://product/releases"
    res_release_deliverables    "surfaces.resources contains specforge://product/releases/{id}/deliverables"
    res_release_milestones      "surfaces.resources contains specforge://product/releases/{id}/milestones"
    res_release_completion      "surfaces.resources contains specforge://product/releases/{id}/completion"
    res_owner_workload          "surfaces.resources contains specforge://product/owner-workload"
    res_weighted_completion     "surfaces.resources contains specforge://product/milestones/{id}/weighted-completion"
  }

  features [pe_surface_contributions, product_surface_access]

  verify unit "manifest declares all CLI commands with valid cmd__ Wasm export names"
  verify unit "CLI commands auto-promote to MCP tools"
  verify unit "manifest declares all MCP resources with specforge:// URIs"
  verify unit "each MCP resource has a valid mcp__ Wasm export name"
  verify unit "every ProductQueryPort method has a corresponding CLI command"
  verify unit "every ProductQueryPort method has a corresponding MCP resource"
}

behavior pe_migration_hook_absent "Migration Hook Absent in v1" {
  category   query
  types      [ManifestV2]
  contract   """
    The @specforge/product extension intentionally omits a migration_hook
    in v1. The entity model is new — there is no prior version to migrate
    from. Migration hooks will be added when the entity model evolves
    in a breaking way.
  """
  ensures    {
    no_migration_hook  "manifest migration_hook is null/absent"
    intentional        "absence is documented and intentional for v1"
  }

  features [pe_core_entity_kinds]

  verify unit "manifest has no migration_hook field"
}

behavior pe_starter_template_content "Starter Template Content" {
  category   command
  types      [ManifestV2]
  contract   """
    The @specforge/product starter template (templates/feature.spec) MUST
    contain at least one feature, one journey, and one deliverable entity
    to demonstrate the minimum viable product planning chain.
  """
  ensures    {
    file_exists      "template file at manifest starter_template path exists and is valid .spec syntax"
    has_feature      "template contains at least one feature entity"
    has_journey      "template contains at least one journey entity"
    has_deliverable  "template contains at least one deliverable entity"
  }

  features [pe_core_entity_kinds]

  verify unit "starter template file exists at declared path"
  verify unit "starter template contains feature, journey, and deliverable"
}

behavior pe_cross_extension_query_boundary "Cross-Extension Query Boundary" {
  category   query
  types      [ProductQueryError]
  ports      [GraphQueryPort]
  contract   """
    All product graph queries MUST traverse only product-owned edge types
    (the 16 declared in the manifest). Queries MUST NOT traverse edges
    owned by other extensions (e.g., Implements from @specforge/software).
    When a product entity has incoming edges from other extensions, those
    edges are invisible to product queries. This ensures product query
    results are identical regardless of which other extensions are installed.

    Enforcement mechanism: every graph traversal call MUST pass an explicit
    product-owned edge type string to GraphQueryPort.getIncomingEdges() and
    GraphQueryPort.getOutgoingEdges(). Product queries MUST NOT call these
    methods without an edgeType parameter or with a wildcard. The allowed
    edge type strings are exactly: JourneyFeature, DeliverableJourney,
    ModuleDependsOn, MilestoneFeature, DeliverableModule, ModuleFeature,
    FeatureDependsOn, JourneyPersona, JourneyChannel, MilestoneModule,
    TermSeeAlso, MilestoneDependsOn, DeliverableMilestone,
    DeliverableDependsOn, ReleaseDeliverable, ReleaseMilestone.
    Any traversal using an edge type not in this allowlist is a bug in
    the product extension.
  """
  requires   {
    graph_ready "product graph is in ready state"
  }
  ensures    {
    explicit_edge_type    "every getIncomingEdges/getOutgoingEdges call passes an explicit product edge type"
    no_wildcard_traversal "no query uses wildcard or empty edgeType parameter"
    allowlist_enforced    "only the 16 declared edge type strings are used in traversal calls"
    own_edges_only        "queries traverse only the 16 product edge types"
    ignores_foreign_edges "edges from @specforge/software or other extensions are not followed"
    results_stable        "query results identical with and without @specforge/software installed"
    no_leakage            "no foreign entity kinds appear in query results"
  }
  maintains {
    edge_type_allowlist "the set of allowed edge types is exactly the 16 declared in the manifest"
  }

  features [pe_query_dependency_analysis, pe_query_traceability, pe_query_coverage_analysis, pe_query_lifecycle_metrics]

  verify unit "milestone completion ignores Implements edges from software extension"
  verify unit "feature impact does not follow non-product edge types"
  verify unit "query results are identical with and without @specforge/software"
  verify unit "no traversal call uses empty or wildcard edgeType"
  verify integration "product queries with @specforge/software co-installed return same results as standalone"
  verify property "all traversal calls use only one of the 16 product edge type strings"
}

behavior pe_render_product_entities "Render Product Entities in Graph Protocol" {
  invariants [pe_rendering_completeness]
  category   query
  types      [
    ProductFeature,
    ProductJourney,
    ProductDeliverable,
    ProductMilestone,
    ProductModule,
    ProductTerm,
    ProductPersona,
    ProductChannel,
    ProductRenderPayload,
  ]
  produces  [pe_product_entities_rendered]
  contract   """
    Product entities MUST render as standard graph nodes in the Graph Protocol
    JSON output. The core emitter handles all entity kinds uniformly — no
    product-specific renderer is needed. Each entity appears with its kind, id,
    fields (per FieldRegistry declaration order), and edges (per EdgeTypeRegistry
    declaration order). Entities with validation errors MUST still appear in
    output with a _diagnostics array containing their diagnostic codes. All three
    export formats (context, graph, brief) MUST include product entities.
  """
  requires   {
    kinds_registered       "all 9 product entity kinds are in KindRegistry"
    fields_registered      "all product field definitions are in FieldRegistry"
    edges_registered       "all 16 product edge types are in EdgeTypeRegistry"
  }
  ensures    {
    context_format         "context export includes full entity fields and resolved edges"
    graph_format           "graph export includes entity node with edge list (no field bodies)"
    brief_format           "brief export includes entity id and kind only"
    field_order            "JSON field ordering follows FieldRegistry declaration order"
    edge_order             "edge ordering follows source entity field declaration order"
    invalid_entities_shown "entities with validation errors appear with _diagnostics array"
    no_product_renderer    "rendering uses core emitter — no extension-specific renderer"
  }

  features [pe_graph_rendering, product_graph_rendering]

  verify unit "feature entity renders with all fields in context format"
  verify unit "journey entity renders with edge list in graph format"
  verify unit "deliverable entity renders as id+kind in brief format"
  verify unit "entity with E007 diagnostic includes _diagnostics array"
  verify unit "field order matches FieldRegistry declaration order"
  verify unit "edge order matches field declaration order in source entity"
  verify unit "all 9 product entity kinds appear in export output"
  verify integration "full product graph renders in all three formats"
}

behavior pe_cross_extension_integration "Cross-Extension Integration with Peer Extensions" {
  category   query
  contract   """
    When @specforge/software is installed as a peer extension, the Implements
    edge (behavior->feature) and MilestoneBehavior entity_enhancement MUST
    integrate correctly with product entities. Product queries MUST NOT follow
    Implements edges (cross-extension isolation), but the Implements edge MUST
    be traversable by software extension queries. Entity enhancements from
    peer extensions MUST add fields to product entity kinds without modifying
    the product manifest.
  """
  requires   {
    product_registered     "all 9 product entity kinds are in KindRegistry"
  }
  ensures    {
    isolation_maintained   "product queries never follow Implements edges"
    enhancement_visible    "MilestoneBehavior fields appear on milestone entities when software is installed"
    standalone_works       "product queries work identically without peer extensions"
    implements_traversable "Implements edges are traversable by software extension queries"
  }

  features [pe_cross_extension_cooperation]

  verify integration "product queries return same results with and without @specforge/software installed"
  verify integration "milestone entity gains MilestoneBehavior fields when software extension enhances it"
  verify integration "Implements edge creates traversable link from behavior to feature"
  verify unit "pe_cross_extension_query_boundary rejects Implements edge traversal"
  verify unit "product standalone: no errors when software extension absent"
}

behavior pe_enforce_migration_strategy "Extension Version Migration" {
  invariants [pe_migration_backward_compat]
  category   command
  contract   """
    The @specforge/product extension MUST follow additive-only schema
    evolution for minor versions. New fields MUST be optional. New diagnostic
    codes MUST use reserved ranges (W047-W048, W050-W056, W058-W074). New
    edge types require a manifest version bump. Breaking changes (field
    removal, kind removal, edge type removal) MUST require a major version
    bump with a migration hook. Until v2, migration_hook remains null.
  """
  requires   {
    manifest_v1            "current manifest declares migration_hook=null"
  }
  ensures    {
    additive_minor         "minor version adds only optional fields and reserved diagnostic codes"
    reserved_codes_used    "new diagnostics consume from reserved ranges before allocating new ranges"
    major_for_breaking     "field/kind/edge removal triggers major version bump"
    migration_hook_v2      "v2 manifest declares a migration hook for v1->v2 transformation"
    backward_compat        "v1 spec files parse without error under v1.x minor bumps"
  }

  features [pe_migration_strategy]

  verify unit "adding optional field does not change manifest version"
  verify unit "adding diagnostic from reserved range does not change manifest version"
  verify unit "removing a field requires major version bump"
  verify unit "v1 spec file parses under v1.1 manifest without errors"
}

behavior pe_emit_validation_rule_details "Emit Validation Rule Details" {
  category   command
  types      [ProductValidationRuleFiredPayload, ProductValidationSummaryPayload]
  contract   """
    The @specforge/product extension SHOULD emit per-rule observability
    events during validation. For each validation rule that fires (produces
    a diagnostic), the extension emits a ProductValidationRuleFiredPayload
    on the product.validation_rule_fired channel. At the end of validation,
    it emits a ProductValidationSummaryPayload on the
    product.validation_summary channel summarizing rules_evaluated,
    rules_fired, rules_suppressed, and a by_severity breakdown. This
    enables external tooling to observe which specific rules triggered
    without parsing diagnostic output.
  """
  requires   {
    registries_populated "KindRegistry and FieldRegistry contain all 9 product entity kinds"
  }
  ensures    {
    per_rule_event       "each fired rule emits ProductValidationRuleFiredPayload"
    summary_event        "validation completion emits ProductValidationSummaryPayload"
    includes_code        "each fired payload includes diagnostic_code and severity"
    includes_entity      "entity-scoped rules include the entity_id"
    summary_counts       "summary rules_evaluated = rules_fired + rules_suppressed"
  }

  features [pe_validation_suite]

  verify unit "fired rule emits event with diagnostic code and severity"
  verify unit "suppressed rule does not emit fired event"
  verify unit "summary counts match individual fired events"
  verify unit "entity-scoped rules include entity_id in payload"
}

behavior pe_field_defaults_in_schema "Field Defaults in Graph Protocol Schema" {
  category   query
  types      [ManifestField]
  contract   """
    The @specforge/product extension MUST declare default_value on all status
    fields so that Graph Protocol JSON Schema metadata includes explicit
    defaults. This enables third-party consumers to correctly interpret absent
    fields without product-specific domain knowledge. Status field defaults:
    feature.status=proposed, milestone.status=planned, deliverable.status=draft,
    persona.status=active, channel.status=active, release.status=planned.
  """
  ensures    {
    feature_status_default      "feature.status default_value is 'proposed'"
    milestone_status_default    "milestone.status default_value is 'planned'"
    deliverable_status_default  "deliverable.status default_value is 'draft'"
    persona_status_default      "persona.status default_value is 'active'"
    channel_status_default      "channel.status default_value is 'active'"
    release_status_default      "release.status default_value is 'planned'"
    schema_emitted              "graph export JSON Schema $defs include default_value metadata"
  }

  features [pe_graph_rendering]

  verify unit "feature.status field has default_value=proposed in manifest"
  verify unit "milestone.status field has default_value=planned in manifest"
  verify unit "deliverable.status field has default_value=draft in manifest"
  verify unit "persona.status field has default_value=active in manifest"
  verify unit "channel.status field has default_value=active in manifest"
  verify unit "release.status field has default_value=planned in manifest"
  verify unit "graph export schema includes default_value metadata"
}
