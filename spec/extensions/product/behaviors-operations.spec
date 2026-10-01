// @specforge/product — Rendering, cross-extension integration, surface declarations, and migration behaviors

use "extensions/product/invariants"
use "extensions/product/types"
use "product/features"
use "types/diagnostics"
use "types/zero-entity-core"

behavior pe_declare_surface_contributions "Declare Surface Contributions" {
  category command
  types    [
    ManifestV2,
    ProductListFilter,
    ProductSurfaceResponse,
    ProductSurfaceError,
    ProductSurfaceFailedPayload,
    ProductSurfaceOperationPayload,
    SurfaceResponseStatus,
  ]
  produces [pe_cli_command_executed, pe_surface_error]
  contract """
    The @specforge/product extension declares no surface contributions: its
    manifest's surfaces list is empty and its module exports no cmd__ or
    mcp__ functions. The specforge product subcommands (features, journeys,
    deliverables, milestones, modules, terms, personas, channels, releases,
    milestone-completion, journey-coverage, feature-impact,
    feature-dependents, persona-features, channel-features, bulk-status,
    health) are built into the CLI.
  """
  ensures {
    no_surfaces "manifest surfaces is empty"
  }
  features [pe_surface_contributions, product_surface_access]
  verify unit "manifest surfaces is empty"
}

behavior pe_migration_hook_absent "Migration Hook Absent in v1" {
  category query
  types    [ManifestV2]
  contract """
    The @specforge/product extension intentionally omits a migration_hook
    in v1. The entity model is new — there is no prior version to migrate
    from. Migration hooks will be added when the entity model evolves
    in a breaking way.
  """
  ensures {
    no_migration_hook "manifest migration_hook is null/absent"
    intentional       "absence is documented and intentional for v1"
  }
  features [pe_core_entity_kinds]
  verify unit "manifest has no migration_hook field"
}

behavior pe_starter_template_content "Starter Template Content" {
  category command
  types    [ManifestV2]
  contract """
    The @specforge/product starter template (extensions/product/src/starter.spec) MUST
    contain at least one feature, one journey, and one deliverable entity
    to demonstrate the minimum viable product planning chain.
  """
  ensures {
    file_exists     "template file at manifest starter_template path exists and is valid .spec syntax"
    has_feature     "template contains at least one feature entity"
    has_journey     "template contains at least one journey entity"
    has_deliverable "template contains at least one deliverable entity"
  }
  features [pe_core_entity_kinds]
  verify unit "starter template file exists at declared path"
  verify unit "starter template contains feature, journey, and deliverable"
}

behavior pe_cross_extension_query_boundary "Cross-Extension Query Boundary" {
  category query
  types    [ProductQueryError]
  ports    [GraphQueryPort]
  contract """
    All product graph queries MUST traverse only product-owned edge types
    (the 20 declared in the manifest). Queries MUST NOT traverse edges
    owned by other extensions (e.g., BehaviorImplementsFeature from @specforge/software).
    When a product entity has incoming edges from other extensions, those
    edges are invisible to product queries. This ensures product query
    results are identical regardless of which other extensions are installed.

    Enforcement mechanism: every graph traversal call MUST pass an explicit
    product-owned edge type string to GraphQueryPort.getIncomingEdges() and
    GraphQueryPort.getOutgoingEdges(). Product queries MUST NOT call these
    methods without an edgeType parameter or with a wildcard. The allowed
    edge type strings are exactly: FeatureDependsOn, FeatureRelatesTo,
    JourneyExercisesFeature, JourneyTargetsPersona, JourneyUsesChannel,
    DeliverableSupportsJourney, DeliverableContainsModule,
    DeliverableTrackedByMilestone, DeliverableDependsOn,
    MilestoneDeliversFeature, MilestoneScopesModule, MilestoneDependsOn,
    ModuleContainsFeature, ModuleDependsOn, TermReferencesRelatedTerm,
    TermBelongsToModule, ReleaseIncludesDeliverable, ReleaseCompletesMilestone,
    ReleaseDependsOn, PersonaPrioritizesFeature.
    Any traversal using an edge type not in this allowlist is a bug in
    the product extension.
  """
  requires {
    graph_ready "product graph is in ready state"
  }
  ensures {
    explicit_edge_type    "every getIncomingEdges/getOutgoingEdges call passes an explicit product edge type"
    no_wildcard_traversal "no query uses wildcard or empty edgeType parameter"
    allowlist_enforced    "only the 20 declared edge type strings are used in traversal calls"
    own_edges_only        "queries traverse only the 20 product edge types"
    ignores_foreign_edges "edges from @specforge/software or other extensions are not followed"
    results_stable        "query results identical with and without @specforge/software installed"
    no_leakage            "no foreign entity kinds appear in query results"
  }
  maintains {
    edge_type_allowlist "the set of allowed edge types is exactly the 20 declared in the manifest"
  }
  features [
    pe_query_dependency_analysis,
    pe_query_traceability,
    pe_query_coverage_analysis,
    pe_query_lifecycle_metrics,
  ]
  verify unit "milestone completion ignores BehaviorImplementsFeature edges from software extension"
  verify unit "feature impact does not follow non-product edge types"
  verify unit "query results are identical with and without @specforge/software"
  verify unit "no traversal call uses empty or wildcard edgeType"
  verify integration "product queries with @specforge/software co-installed return same results as standalone"
  verify property "all traversal calls use only one of the 20 product edge type strings"
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
  produces   [pe_product_entities_rendered]
  contract   """
    Product entities MUST render as standard graph nodes in the Graph Protocol
    JSON output. The core emitter handles all entity kinds uniformly — no
    product-specific renderer is needed. Each entity appears with its kind, id,
    fields (per FieldRegistry declaration order), and edges (per EdgeTypeRegistry
    declaration order). Entities with validation errors MUST still appear in
    output with a _diagnostics array containing their diagnostic codes. All three
    export formats (context, graph, brief) MUST include product entities.
  """
  requires {
    kinds_registered  "all 9 product entity kinds are in KindRegistry"
    fields_registered "all product field definitions are in FieldRegistry"
    edges_registered  "all 20 product edge types are in EdgeTypeRegistry"
  }
  ensures {
    context_format         "context export includes full entity fields and resolved edges"
    graph_format           "graph export includes entity node with edge list (no field bodies)"
    brief_format           "brief export includes entity id and kind only"
    field_order            "JSON field ordering follows FieldRegistry declaration order"
    edge_order             "edge ordering follows source entity field declaration order"
    invalid_entities_shown "entities with validation errors appear with _diagnostics array"
    no_product_renderer    "rendering uses core emitter — no extension-specific renderer"
  }
  features   [pe_graph_rendering, product_graph_rendering]
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
  category query
  contract """
    When @specforge/software is installed as a peer extension, the BehaviorImplementsFeature
    edge (behavior->feature) and the entity_enhancement that adds a behaviors
    field to milestone (MilestoneIncludesBehavior edges) MUST
    integrate correctly with product entities. Product queries MUST NOT follow
    BehaviorImplementsFeature edges (cross-extension isolation), but the BehaviorImplementsFeature edge MUST
    be traversable by software extension queries. Entity enhancements from
    peer extensions MUST add fields to product entity kinds without modifying
    the product manifest.
  """
  requires {
    product_registered "all 9 product entity kinds are in KindRegistry"
  }
  ensures {
    isolation_maintained   "product queries never follow BehaviorImplementsFeature edges"
    enhancement_visible    "the behaviors field appears on milestone entities when software is installed"
    standalone_works       "product queries work identically without peer extensions"
    implements_traversable "BehaviorImplementsFeature edges are traversable by software extension queries"
  }
  features [pe_cross_extension_cooperation]
  verify integration "product queries return same results with and without @specforge/software installed"
  verify integration "milestone entity gains a behaviors field when software extension enhances it"
  verify integration "BehaviorImplementsFeature edge creates traversable link from behavior to feature"
  verify unit "pe_cross_extension_query_boundary rejects BehaviorImplementsFeature edge traversal"
  verify unit "product standalone: no errors when software extension absent"
}

behavior pe_enforce_migration_strategy "Extension Version Migration" {
  invariants [pe_migration_backward_compat]
  category   command
  contract   """
    The @specforge/product extension MUST follow additive-only schema
    evolution for minor versions. New fields MUST be optional. New diagnostic
    codes are allocated outside the third-party range (900-998) and added to
    the diagnostic catalog. New edge types require a manifest version bump. Breaking changes (field
    removal, kind removal, edge type removal) MUST require a major version
    bump with a migration hook. Until v2, migration_hook remains null.
  """
  requires {
    manifest_v1 "current manifest declares migration_hook=null"
  }
  ensures {
    additive_minor     "minor version adds only optional fields and new diagnostic codes"
    catalogued_codes   "new diagnostics get a diagnostic catalog entry outside the third-party 900-998 range"
    major_for_breaking "field/kind/edge removal triggers major version bump"
    migration_hook_v2  "v2 manifest declares a migration hook for v1->v2 transformation"
    backward_compat    "v1 spec files parse without error under v1.x minor bumps"
  }
  features   [pe_migration_strategy]
  verify unit "adding optional field does not change manifest version"
  verify unit "adding a diagnostic does not change manifest version"
  verify unit "removing a field requires major version bump"
  verify unit "v1 spec file parses under v1.1 manifest without errors"
}

behavior pe_field_defaults_in_schema "Field Defaults in Graph Protocol Schema" {
  category query
  types    [ManifestField]
  contract """
    The @specforge/product extension declares no default_value on any field,
    status fields included, so Graph Protocol JSON Schema metadata carries
    no defaults for product fields. An absent status is interpreted by each
    consumer.
  """
  ensures {
    no_status_defaults "every product field's default_value is null"
  }
  features [pe_graph_rendering]
  verify unit "no product field declares a default_value"
}
