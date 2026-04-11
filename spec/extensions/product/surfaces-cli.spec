// CLI surface contributions — list commands, query commands, and planning insights
//
// This file specifies CLI command surface contributions for the @specforge/product
// extension. Every CLI command is auto-promoted to an MCP tool with the same JSON
// Schema. See surfaces-mcp.spec for MCP resources and surfaces-shared.spec for
// cross-cutting conventions.

use "extensions/product/types"
use "extensions/product/behaviors-registration"
use "extensions/product/behaviors-queries"
use "extensions/product/behaviors-operations"
use "extensions/product/behaviors-v1-1"
use "extensions/product/features"

// ════════════════════════════════════════════════════════════════
// CLI List Commands (8 + 1 v1.1) — entity listing with filter/pagination
// ════════════════════════════════════════════════════════════════

behavior surface_list_features "Surface: List Features" {
  category   command
  types      [ProductListFilter, FeatureListResult, FeatureListEntry, ProductSurfaceError]
  contract   """
    The product:features CLI command MUST list all feature entities in
    the product graph, returning a paginated FeatureListResult. Accepts
    optional --status, --priority, --tags, --limit (default 100),
    --offset (default 0), --sort-by (default "id"), --sort-order
    (default "asc"), and --format (default "json") flags.
    Wasm export: cmd__product_features.
    MCP tool: specforge.product.features.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list       "stdout is a valid FeatureListResult JSON object"
    pagination_correct "total reflects unfiltered count; has_more is true iff offset + limit < total"
    status_filter      "when --status is set, only features with matching FeatureStatus are returned"
    priority_filter    "when --priority is set, only features with matching Priority are returned"
    tags_filter        "when --tags is set, only features whose tags intersect the filter set are returned"
    sort_applied       "entries are sorted by sort_by field in sort_order direction"
    limit_respected    "entries.length <= limit"
    empty_result       "project with no features returns empty list with total=0"
    table_format       "when --format=table, stdout is human-readable table with columns: id, title, status, priority"
    brief_format       "when --format=brief, stdout is newline-delimited entity IDs only"
    exit_zero          "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list features returns paginated FeatureListResult"
  verify unit "status filter reduces result set"
  verify unit "priority filter reduces result set"
  verify unit "tags filter intersects correctly"
  verify unit "pagination offset and limit are respected"
  verify unit "empty project returns total=0 and empty list"
  verify unit "table format produces human-readable output"
}

behavior surface_list_journeys "Surface: List Journeys" {
  category   command
  types      [ProductListFilter, JourneyListResult, JourneyListEntry, ProductSurfaceError]
  contract   """
    The product:journeys CLI command MUST list all journey entities in
    the product graph, returning a paginated JourneyListResult. Accepts
    the standard list flags (--status, --priority, --tags, --limit,
    --offset, --sort-by, --sort-order, --format) plus --persona to
    filter by persona ID.
    Wasm export: cmd__product_journeys.
    MCP tool: specforge.product.journeys.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list       "stdout is a valid JourneyListResult JSON object"
    persona_filter     "when --persona is set, only journeys referencing that persona (via JourneyPersona edge) are returned"
    pagination_correct "total reflects unfiltered count; has_more is true iff offset + limit < total"
    channel_count      "each entry's channel_count reflects the number of JourneyChannel edges"
    feature_count      "each entry's feature_count reflects the number of JourneyFeature edges"
    exit_zero          "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list journeys returns paginated JourneyListResult"
  verify unit "persona filter reduces result set to matching journeys"
  verify unit "channel_count and feature_count are accurate per entry"
}

behavior surface_list_deliverables "Surface: List Deliverables" {
  category   command
  types      [ProductListFilter, DeliverableListResult, DeliverableListEntry, ProductSurfaceError]
  contract   """
    The product:deliverables CLI command MUST list all deliverable entities
    in the product graph, returning a paginated DeliverableListResult.
    Accepts the standard list flags plus --artifact-type to filter by
    ArtifactType.
    Wasm export: cmd__product_deliverables.
    MCP tool: specforge.product.deliverables.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list         "stdout is a valid DeliverableListResult JSON object"
    artifact_type_filter "when --artifact-type is set, only deliverables with matching ArtifactType are returned"
    status_filter        "when --status is set, only deliverables with matching DeliverableStatus are returned"
    journey_count        "each entry's journey_count reflects the number of DeliverableJourney edges"
    module_count         "each entry's module_count reflects the number of DeliverableModule edges"
    exit_zero            "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list deliverables returns paginated DeliverableListResult"
  verify unit "artifact-type filter reduces result set"
  verify unit "journey_count and module_count are accurate per entry"
}

behavior surface_list_milestones "Surface: List Milestones" {
  category   command
  types      [ProductListFilter, MilestoneListResult, MilestoneListEntry, ProductSurfaceError]
  contract   """
    The product:milestones CLI command MUST list all milestone entities
    in the product graph, returning a paginated MilestoneListResult.
    Accepts the standard list flags.
    Wasm export: cmd__product_milestones.
    MCP tool: specforge.product.milestones.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list       "stdout is a valid MilestoneListResult JSON object"
    status_filter      "when --status is set, only milestones with matching MilestoneStatus are returned"
    priority_filter    "when --priority is set, only milestones with matching Priority are returned"
    feature_count      "each entry's feature_count reflects the number of MilestoneFeature edges"
    exit_zero          "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list milestones returns paginated MilestoneListResult"
  verify unit "status filter reduces result set"
  verify unit "feature_count is accurate per entry"
}

behavior surface_list_modules "Surface: List Modules" {
  category   command
  types      [ProductListFilter, ModuleListResult, ModuleListEntry, ProductSurfaceError]
  contract   """
    The product:modules CLI command MUST list all module entities in the
    product graph, returning a paginated ModuleListResult. Accepts the
    standard list flags plus --family to filter by module family.
    Wasm export: cmd__product_modules.
    MCP tool: specforge.product.modules.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list    "stdout is a valid ModuleListResult JSON object"
    family_filter   "when --family is set, only modules with matching family are returned"
    feature_count   "each entry's feature_count reflects the number of ModuleFeature edges"
    depends_on      "each entry's depends_on lists outgoing ModuleDependsOn target IDs"
    exit_zero       "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list modules returns paginated ModuleListResult"
  verify unit "family filter reduces result set"
  verify unit "feature_count and depends_on are accurate per entry"
}

behavior surface_list_terms "Surface: List Terms" {
  category   command
  types      [ProductListFilter, TermListResult, TermListEntry, ProductSurfaceError]
  contract   """
    The product:terms CLI command MUST list all term entities in the
    product graph, returning a paginated TermListResult. Accepts the
    standard list flags.
    Wasm export: cmd__product_terms.
    MCP tool: specforge.product.terms.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list   "stdout is a valid TermListResult JSON object"
    alias_count    "each entry's alias_count reflects the length of the aliases field"
    definition     "each entry includes the full definition string"
    exit_zero      "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list terms returns paginated TermListResult"
  verify unit "alias_count is accurate per entry"
}

behavior surface_list_personas "Surface: List Personas" {
  category   command
  types      [ProductListFilter, PersonaListResult, PersonaListEntry, ProductSurfaceError]
  contract   """
    The product:personas CLI command MUST list all persona entities in
    the product graph, returning a paginated PersonaListResult. Accepts
    the standard list flags plus --technical-level to filter by
    TechnicalLevel.
    Wasm export: cmd__product_personas.
    MCP tool: specforge.product.personas.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list          "stdout is a valid PersonaListResult JSON object"
    technical_level_filter "when --technical-level is set, only personas with matching TechnicalLevel are returned"
    journey_count          "each entry's journey_count reflects the number of reverse JourneyPersona edges"
    exit_zero              "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list personas returns paginated PersonaListResult"
  verify unit "technical-level filter reduces result set"
  verify unit "journey_count is accurate per entry"
}

behavior surface_list_channels "Surface: List Channels" {
  category   command
  types      [ProductListFilter, ChannelListResult, ChannelListEntry, ProductSurfaceError]
  contract   """
    The product:channels CLI command MUST list all channel entities in
    the product graph, returning a paginated ChannelListResult. Accepts
    the standard list flags plus --interaction-model to filter by
    InteractionModel.
    Wasm export: cmd__product_channels.
    MCP tool: specforge.product.channels.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    returns_list              "stdout is a valid ChannelListResult JSON object"
    interaction_model_filter  "when --interaction-model is set, only channels with matching InteractionModel are returned"
    journey_count             "each entry's journey_count reflects the number of reverse JourneyChannel edges"
    exit_zero                 "exit code 0 on success"
  }

  features [pe_surface_contributions]

  verify unit "list channels returns paginated ChannelListResult"
  verify unit "interaction-model filter reduces result set"
  verify unit "journey_count is accurate per entry"
}


// ════════════════════════════════════════════════════════════════
// CLI Query Commands (6) — graph queries with typed I/O
// ════════════════════════════════════════════════════════════════

behavior surface_milestone_completion "Surface: Milestone Completion" {
  category   command
  types      [MilestoneCompletionInput, MilestoneCompletionPayload, ProductSurfaceError]
  contract   """
    The product:milestone-completion CLI command MUST accept a positional
    milestone_id argument and return a MilestoneCompletionPayload JSON
    object on stdout. Delegates to pe_query_milestone_completion.
    Wasm export: cmd__product_milestone_completion.
    MCP tool: specforge.product.milestone_completion.
    MCP tool input schema:
      { "milestone_id": { "type": "string", "description": "Entity ID of the milestone" } }
    Exit code 0 on success, 1 on entity-not-found or graph-not-ready.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    delegates           "delegates to pe_query_milestone_completion with the provided milestone_id"
    json_output         "stdout is a valid MilestoneCompletionPayload JSON: milestone_id, total_features, done_count, completion_ratio, done_features"
    not_found_error     "missing milestone_id returns ProductSurfaceError with suggestion and exit code 1"
    table_format        "when --format=table, stdout is: milestone_id, done/total, ratio%"
    exit_zero_success   "exit code 0 when query succeeds"
    exit_one_error      "exit code 1 when entity not found or graph not ready"
  }

  features [pe_surface_contributions]

  verify unit "milestone-completion returns MilestoneCompletionPayload JSON"
  verify unit "missing milestone ID returns error with suggestion"
  verify unit "table format shows ratio as percentage"
  verify unit "exit code 0 on success, 1 on error"
}

behavior surface_journey_coverage "Surface: Journey Coverage" {
  category   command
  types      [JourneyCoverageInput, JourneyCoveragePayload, ProductSurfaceError]
  contract   """
    The product:journey-coverage CLI command MUST accept a positional
    journey_id argument and return a JourneyCoveragePayload JSON object
    on stdout. Delegates to pe_query_journey_coverage.
    Wasm export: cmd__product_journey_coverage.
    MCP tool: specforge.product.journey_coverage.
    MCP tool input schema:
      { "journey_id": { "type": "string", "description": "Entity ID of the journey" } }
    Exit code 0 on success, 1 on entity-not-found or graph-not-ready.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    delegates           "delegates to pe_query_journey_coverage with the provided journey_id"
    json_output         "stdout is a valid JourneyCoveragePayload JSON: journey_id, total_features, covered_count, uncovered_features"
    not_found_error     "missing journey_id returns ProductSurfaceError with suggestion and exit code 1"
    table_format        "when --format=table, stdout is: journey_id, covered/total, uncovered list"
    exit_zero_success   "exit code 0 when query succeeds"
    exit_one_error      "exit code 1 when entity not found or graph not ready"
  }

  features [pe_surface_contributions]

  verify unit "journey-coverage returns JourneyCoveragePayload JSON"
  verify unit "missing journey ID returns error with suggestion"
  verify unit "exit code 0 on success, 1 on error"
}

behavior surface_feature_ordering "Surface: Feature Ordering" {
  category   command
  types      [FeatureOrderingPayload, ProductSurfaceError]
  contract   """
    The product:feature-ordering CLI command takes no positional arguments
    and returns a FeatureOrderingPayload JSON object on stdout. Delegates
    to pe_query_feature_ordering. This is a global query.
    Wasm export: cmd__product_feature_ordering.
    MCP tool: specforge.product.feature_ordering.
    MCP tool input schema: {} (no parameters).
    Exit code 0 on success, 1 on graph-not-ready.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    delegates           "delegates to pe_query_feature_ordering"
    json_output         "stdout is a valid FeatureOrderingPayload JSON: sorted_features, has_cycles, cycle_members"
    no_args             "command takes no positional entity ID argument"
    table_format        "when --format=table, stdout is numbered feature list with cycle members flagged"
    exit_zero_success   "exit code 0 when query succeeds (even if cycles exist)"
    exit_one_error      "exit code 1 only when graph not ready"
  }

  features [pe_surface_contributions]

  verify unit "feature-ordering returns FeatureOrderingPayload JSON"
  verify unit "cycles present in output does not cause exit code 1"
  verify unit "empty feature graph returns empty sorted list"
}

behavior surface_milestone_timeline "Surface: Milestone Timeline" {
  category   command
  types      [MilestoneTimelineInput, MilestoneTimelinePayload, MilestoneTimelineEntry, ProductSurfaceError]
  contract   """
    The product:milestone-timeline CLI command takes an optional --as-of
    flag (ISO 8601 date string, defaults to the build timestamp) and
    returns a MilestoneTimelinePayload JSON object on stdout. Delegates
    to pe_query_milestone_timeline. Overdue detection is query-time
    only — it does NOT emit I058 diagnostics during specforge check.
    Wasm export: cmd__product_milestone_timeline.
    MCP tool: specforge.product.milestone_timeline.
    MCP tool input schema:
      { "as_of_date": { "type": "string", "format": "date", "description": "Date for overdue calculation (default: build timestamp)" } }
    Exit code 0 on success, 1 on graph-not-ready.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    delegates           "delegates to pe_query_milestone_timeline with optional as_of_date"
    json_output         "stdout is a valid MilestoneTimelinePayload JSON: milestones[], overdue_count"
    entry_fields        "each MilestoneTimelineEntry has: milestone_id, target_date, status, is_overdue, priority"
    date_default        "when --as-of is omitted, build timestamp is used (deterministic)"
    no_validation_side_effect "does NOT emit I058 diagnostics — query-time only"
    table_format        "when --format=table, stdout is chronological table with overdue markers"
    exit_zero_success   "exit code 0 when query succeeds"
  }

  features [pe_surface_contributions]

  verify unit "milestone-timeline returns MilestoneTimelinePayload JSON"
  verify unit "as-of flag overrides current date for overdue calculation"
  verify unit "table format marks overdue milestones"
}

behavior surface_milestone_deliverables "Surface: Milestone Deliverables" {
  category   command
  types      [MilestoneDeliverablesInput, MilestoneDeliverablePayload, ProductSurfaceError]
  contract   """
    The product:milestone-deliverables CLI command MUST accept a positional
    milestone_id argument and return a MilestoneDeliverablePayload JSON
    object on stdout. Delegates to pe_query_milestone_deliverables.
    Wasm export: cmd__product_milestone_deliverables.
    MCP tool: specforge.product.milestone_deliverables.
    MCP tool input schema:
      { "milestone_id": { "type": "string", "description": "Entity ID of the milestone" } }
    Exit code 0 on success, 1 on entity-not-found or graph-not-ready.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    delegates           "delegates to pe_query_milestone_deliverables with the provided milestone_id"
    json_output         "stdout is a valid MilestoneDeliverablePayload JSON: milestone_id, deliverables[], count"
    not_found_error     "missing milestone_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_zero_success   "exit code 0 when query succeeds"
    exit_one_error      "exit code 1 when entity not found or graph not ready"
  }

  features [pe_surface_contributions]

  verify unit "milestone-deliverables returns MilestoneDeliverablePayload JSON"
  verify unit "missing milestone ID returns error with suggestion"
  verify unit "exit code 0 on success, 1 on error"
}

behavior surface_module_features "Surface: Module Features" {
  category   command
  types      [ModuleFeaturesInput, ModuleFeaturePayload, ProductSurfaceError]
  contract   """
    The product:module-features CLI command MUST accept a positional
    module_id argument and return a ModuleFeaturePayload JSON object on
    stdout. Delegates to pe_query_module_features.
    Wasm export: cmd__product_module_features.
    MCP tool: specforge.product.module_features.
    MCP tool input schema:
      { "module_id": { "type": "string", "description": "Entity ID of the module" } }
    Exit code 0 on success, 1 on entity-not-found or graph-not-ready.
  """
  requires {
    graph_ready "product graph is built and in ready state"
  }
  ensures  {
    delegates           "delegates to pe_query_module_features with the provided module_id"
    json_output         "stdout is a valid ModuleFeaturePayload JSON: module_id, features[], count"
    not_found_error     "missing module_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_zero_success   "exit code 0 when query succeeds"
    exit_one_error      "exit code 1 when entity not found or graph not ready"
  }

  features [pe_surface_contributions]

  verify unit "module-features returns ModuleFeaturePayload JSON"
  verify unit "missing module ID returns error with suggestion"
  verify unit "exit code 0 on success, 1 on error"
}


// ════════════════════════════════════════════════════════════════
// CLI Query Commands (16) — parity with MCP-only queries
//
// These commands were previously MCP-resource-only. Adding CLI
// equivalents ensures full CLI/MCP surface parity: every
// ProductQueryPort method is accessible from both surfaces.
// ════════════════════════════════════════════════════════════════

behavior surface_deliverable_traceability "Surface: Deliverable Traceability" {
  category   command
  types      [DeliverableTraceabilityPayload, ProductSurfaceError]
  contract   """
    The product:deliverable-traceability CLI command MUST accept a positional
    deliverable_id argument and return a DeliverableTraceabilityPayload.
    Delegates to pe_query_deliverable_traceability.
    Wasm export: cmd__product_deliverable_traceability.
    MCP tool: specforge.product.deliverable_traceability.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_deliverable_traceability"
    json_output     "stdout is a valid DeliverableTraceabilityPayload JSON"
    not_found_error "missing deliverable_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "deliverable-traceability returns DeliverableTraceabilityPayload JSON"
  verify unit "missing deliverable ID returns error with suggestion"
}

behavior surface_feature_deliverables "Surface: Feature Deliverables" {
  category   command
  types      [FeatureDeliverablePayload, ProductSurfaceError]
  contract   """
    The product:feature-deliverables CLI command MUST accept a positional
    feature_id argument and return a FeatureDeliverablePayload.
    Delegates to pe_query_feature_deliverables.
    Wasm export: cmd__product_feature_deliverables.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_feature_deliverables"
    json_output     "stdout is a valid FeatureDeliverablePayload JSON"
    not_found_error "missing feature_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "feature-deliverables returns FeatureDeliverablePayload JSON"
  verify unit "missing feature ID returns error with suggestion"
}

behavior surface_feature_milestones "Surface: Feature Milestones" {
  category   command
  types      [FeatureMilestonePayload, ProductSurfaceError]
  contract   """
    The product:feature-milestones CLI command MUST accept a positional
    feature_id argument and return a FeatureMilestonePayload.
    Delegates to pe_query_feature_milestones.
    Wasm export: cmd__product_feature_milestones.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_feature_milestones"
    json_output     "stdout is a valid FeatureMilestonePayload JSON"
    not_found_error "missing feature_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "feature-milestones returns FeatureMilestonePayload JSON"
  verify unit "missing feature ID returns error with suggestion"
}

behavior surface_persona_journeys "Surface: Persona Journeys" {
  category   command
  types      [PersonaJourneyPayload, ProductSurfaceError]
  contract   """
    The product:persona-journeys CLI command MUST accept a positional
    persona_id argument and return a PersonaJourneyPayload.
    Delegates to pe_query_persona_journeys.
    Wasm export: cmd__product_persona_journeys.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_persona_journeys"
    json_output     "stdout is a valid PersonaJourneyPayload JSON"
    not_found_error "missing persona_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "persona-journeys returns PersonaJourneyPayload JSON"
  verify unit "missing persona ID returns error with suggestion"
}

behavior surface_channel_journeys "Surface: Channel Journeys" {
  category   command
  types      [ChannelJourneyPayload, ProductSurfaceError]
  contract   """
    The product:channel-journeys CLI command MUST accept a positional
    channel_id argument and return a ChannelJourneyPayload.
    Delegates to pe_query_channel_journeys.
    Wasm export: cmd__product_channel_journeys.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_channel_journeys"
    json_output     "stdout is a valid ChannelJourneyPayload JSON"
    not_found_error "missing channel_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "channel-journeys returns ChannelJourneyPayload JSON"
  verify unit "missing channel ID returns error with suggestion"
}

behavior surface_module_deliverables "Surface: Module Deliverables" {
  category   command
  types      [ModuleDeliverablePayload, ProductSurfaceError]
  contract   """
    The product:module-deliverables CLI command MUST accept a positional
    module_id argument and return a ModuleDeliverablePayload.
    Delegates to pe_query_module_deliverables.
    Wasm export: cmd__product_module_deliverables.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_module_deliverables"
    json_output     "stdout is a valid ModuleDeliverablePayload JSON"
    not_found_error "missing module_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "module-deliverables returns ModuleDeliverablePayload JSON"
  verify unit "missing module ID returns error with suggestion"
}

behavior surface_term_graph "Surface: Term Graph" {
  category   command
  types      [TermGraphPayload, ProductSurfaceError]
  contract   """
    The product:term-graph CLI command MUST accept a positional term_id
    argument and optional --max-hops flag (default 1, max 5). Returns a
    TermGraphPayload. Delegates to pe_query_term_graph.
    Wasm export: cmd__product_term_graph.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_term_graph with term_id and optional maxHops"
    json_output     "stdout is a valid TermGraphPayload JSON"
    max_hops_cap    "maxHops > 5 is clamped to 5 without error"
    not_found_error "missing term_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "term-graph returns TermGraphPayload JSON"
  verify unit "max-hops flag is respected and capped at 5"
  verify unit "missing term ID returns error with suggestion"
}

behavior surface_deliverable_completion "Surface: Deliverable Completion" {
  category   command
  types      [DeliverableCompletionPayload, ProductSurfaceError]
  contract   """
    The product:deliverable-completion CLI command MUST accept a positional
    deliverable_id argument and return a DeliverableCompletionPayload.
    Delegates to pe_query_deliverable_completion.
    Wasm export: cmd__product_deliverable_completion.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_deliverable_completion"
    json_output     "stdout is a valid DeliverableCompletionPayload JSON"
    not_found_error "missing deliverable_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "deliverable-completion returns DeliverableCompletionPayload JSON"
  verify unit "missing deliverable ID returns error with suggestion"
}

behavior surface_persona_channels "Surface: Persona Channels" {
  category   command
  types      [PersonaChannelPayload, ProductSurfaceError]
  contract   """
    The product:persona-channels CLI command MUST accept a positional
    persona_id argument and return a PersonaChannelPayload.
    Delegates to pe_query_persona_channels.
    Wasm export: cmd__product_persona_channels.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_persona_channels"
    json_output     "stdout is a valid PersonaChannelPayload JSON"
    not_found_error "missing persona_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "persona-channels returns PersonaChannelPayload JSON"
  verify unit "missing persona ID returns error with suggestion"
}

behavior surface_journey_deliverables "Surface: Journey Deliverables" {
  category   command
  types      [JourneyDeliverablePayload, ProductSurfaceError]
  contract   """
    The product:journey-deliverables CLI command MUST accept a positional
    journey_id argument and return a JourneyDeliverablePayload.
    Delegates to pe_query_journey_deliverables.
    Wasm export: cmd__product_journey_deliverables.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_journey_deliverables"
    json_output     "stdout is a valid JourneyDeliverablePayload JSON"
    not_found_error "missing journey_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "journey-deliverables returns JourneyDeliverablePayload JSON"
  verify unit "missing journey ID returns error with suggestion"
}

behavior surface_feature_dependents "Surface: Feature Dependents" {
  category   command
  types      [FeatureDependentPayload, ProductSurfaceError]
  contract   """
    The product:feature-dependents CLI command MUST accept a positional
    feature_id argument and return a FeatureDependentPayload.
    Delegates to pe_query_feature_dependents.
    Wasm export: cmd__product_feature_dependents.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_feature_dependents"
    json_output     "stdout is a valid FeatureDependentPayload JSON"
    not_found_error "missing feature_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "feature-dependents returns FeatureDependentPayload JSON"
  verify unit "missing feature ID returns error with suggestion"
}

behavior surface_deliverable_dependents "Surface: Deliverable Dependents" {
  category   command
  types      [DeliverableDependentPayload, ProductSurfaceError]
  contract   """
    The product:deliverable-dependents CLI command MUST accept a positional
    deliverable_id argument and return a DeliverableDependentPayload.
    Delegates to pe_query_deliverable_dependents.
    Wasm export: cmd__product_deliverable_dependents.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_deliverable_dependents"
    json_output     "stdout is a valid DeliverableDependentPayload JSON"
    not_found_error "missing deliverable_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "deliverable-dependents returns DeliverableDependentPayload JSON"
  verify unit "missing deliverable ID returns error with suggestion"
}

behavior surface_deliverable_priority "Surface: Deliverable Priority" {
  category   command
  types      [DeliverablePriorityPayload, ProductSurfaceError]
  contract   """
    The product:deliverable-priority CLI command MUST accept a positional
    deliverable_id argument and return a DeliverablePriorityPayload.
    Delegates to pe_query_deliverable_priority.
    Wasm export: cmd__product_deliverable_priority.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_deliverable_priority"
    json_output     "stdout is a valid DeliverablePriorityPayload JSON"
    null_priority   "deliverable with no milestones/journeys returns priority=null"
    not_found_error "missing deliverable_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "deliverable-priority returns DeliverablePriorityPayload JSON"
  verify unit "missing deliverable ID returns error with suggestion"
}

behavior surface_persona_features "Surface: Persona Features" {
  category   command
  types      [PersonaFeaturePayload, ProductSurfaceError]
  contract   """
    The product:persona-features CLI command MUST accept a positional
    persona_id argument and return a PersonaFeaturePayload via multi-hop
    persona->journey->feature traversal. Delegates to pe_query_persona_features.
    Wasm export: cmd__product_persona_features.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_persona_features"
    json_output     "stdout is a valid PersonaFeaturePayload JSON"
    not_found_error "missing persona_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "persona-features returns PersonaFeaturePayload JSON"
  verify unit "missing persona ID returns error with suggestion"
}

behavior surface_feature_impact "Surface: Feature Impact" {
  category   command
  types      [FeatureImpactPayload, ProductSurfaceError]
  contract   """
    The product:feature-impact CLI command MUST accept a positional
    feature_id argument and return a FeatureImpactPayload with transitive
    impact analysis. Delegates to pe_query_feature_impact.
    Wasm export: cmd__product_feature_impact.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_feature_impact"
    json_output     "stdout is a valid FeatureImpactPayload JSON"
    not_found_error "missing feature_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "feature-impact returns FeatureImpactPayload JSON"
  verify unit "missing feature ID returns error with suggestion"
}

behavior surface_milestone_velocity "Surface: Milestone Velocity" {
  category   command
  types      [MilestoneVelocityPayload, ProductSurfaceError]
  contract   """
    The product:milestone-velocity CLI command MUST accept a positional
    milestone_id argument and return a MilestoneVelocityPayload.
    Delegates to pe_query_milestone_velocity.
    Wasm export: cmd__product_milestone_velocity.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_milestone_velocity"
    json_output     "stdout is a valid MilestoneVelocityPayload JSON"
    not_found_error "missing milestone_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "milestone-velocity returns MilestoneVelocityPayload JSON"
  verify unit "missing milestone ID returns error with suggestion"
}

behavior surface_deliverable_personas "Surface: Deliverable Personas" {
  category   command
  types      [DeliverablePersonaPayload, ProductSurfaceError]
  contract   """
    The product:deliverable-personas CLI command MUST accept a positional
    deliverable_id argument and return a DeliverablePersonaPayload.
    Delegates to pe_query_deliverable_personas.
    Wasm export: cmd__product_deliverable_personas.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_deliverable_personas"
    json_output     "stdout is a valid DeliverablePersonaPayload JSON"
    not_found_error "missing deliverable_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "deliverable-personas returns DeliverablePersonaPayload JSON"
  verify unit "missing deliverable ID returns error with suggestion"
}

behavior surface_feature_overlap "Surface: Feature Overlap" {
  category   command
  types      [FeatureOverlapPayload, ProductSurfaceError]
  contract   """
    The product:feature-overlap CLI command MUST return features shared
    across 2+ deliverables. No positional arguments. Delegates to
    pe_query_feature_overlap.
    Wasm export: cmd__product_feature_overlap.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_feature_overlap"
    json_output     "stdout is a valid FeatureOverlapPayload JSON"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "feature-overlap returns FeatureOverlapPayload JSON"
  verify unit "no overlapping features returns empty list"
}

behavior surface_channel_features "Surface: Channel Features" {
  category   command
  types      [ChannelFeaturePayload, ProductSurfaceError]
  contract   """
    The product:channel-features CLI command MUST accept a positional
    channel_id argument and return a ChannelFeaturePayload via multi-hop
    channel->journey->feature traversal. Delegates to pe_query_channel_features.
    Wasm export: cmd__product_channel_features.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_channel_features"
    json_output     "stdout is a valid ChannelFeaturePayload JSON"
    not_found_error "missing channel_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "channel-features returns ChannelFeaturePayload JSON"
  verify unit "missing channel ID returns error with suggestion"
}

behavior surface_release_milestones "Surface: Release Milestones" {
  category   command
  types      [ReleaseMilestonePayload, ProductSurfaceError]
  contract   """
    The product:release-milestones CLI command MUST accept a positional
    release_id argument and return a ReleaseMilestonePayload.
    Delegates to pe_query_release_milestones.
    Wasm export: cmd__product_release_milestones.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_release_milestones"
    json_output     "stdout is a valid ReleaseMilestonePayload JSON"
    not_found_error "missing release_id returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }
  features [pe_surface_contributions]
  verify unit "release-milestones returns ReleaseMilestonePayload JSON"
  verify unit "missing release ID returns error with suggestion"
}

// ════════════════════════════════════════════════════════════════
// CLI Command — Unscheduled Features
// ════════════════════════════════════════════════════════════════

behavior surface_unscheduled_features "Surface: Unscheduled Features" {
  category   command
  types      [UnscheduledFeaturesPayload, ProductSurfaceError]
  contract   """
    The product:unscheduled-features CLI command MUST return all features
    not scheduled in any milestone. Accepts --format flag (json/table/brief).
    Table format shows feature ID and status in aligned columns. Brief
    format lists feature IDs one per line. Auto-promoted to MCP tool.
  """
  ensures  {
    json_output     "json format returns full UnscheduledFeaturesPayload"
    table_output    "table format shows feature ID and status"
    brief_output    "brief format lists feature IDs"
    exit_code       "exit 0 on success, exit 1 on error"
  }

  features [pe_surface_contributions]

  verify unit "product:unscheduled-features returns UnscheduledFeaturesPayload"
  verify unit "no unscheduled features returns empty list"
}

// ════════════════════════════════════════════════════════════════
// CLI Command — Coverage Matrix
// ════════════════════════════════════════════════════════════════

behavior surface_coverage_matrix "Surface: Coverage Matrix" {
  category   command
  types      [PersonaCoverageMatrixPayload, PersonaCoverageEntry, ProductSurfaceError]
  contract   """
    The product:coverage-matrix CLI command MUST return the persona
    coverage matrix showing feature reachability per persona. Accepts
    --format flag (json/table/brief). Table format shows persona ID,
    reachable count, unreachable count, and coverage ratio in aligned
    columns. Brief format outputs overall_coverage as a percentage.
    Auto-promoted to MCP tool.
  """
  ensures  {
    json_output     "json format returns full PersonaCoverageMatrixPayload"
    table_output    "table format shows per-persona coverage"
    brief_output    "brief format outputs overall coverage percentage"
    exit_code       "exit 0 on success, exit 1 on error"
  }

  features [pe_surface_contributions]

  verify unit "product:coverage-matrix returns PersonaCoverageMatrixPayload"
  verify unit "no personas returns empty matrix"
}

// ════════════════════════════════════════════════════════════════
// CLI Command — Critical Path
// ════════════════════════════════════════════════════════════════

behavior surface_critical_path "Surface: Critical Path" {
  category   command
  types      [CriticalPathPayload, CriticalPathNode, ProductSurfaceError]
  contract   """
    The product:critical-path CLI command MUST compute and return the
    critical path through the milestone dependency graph. Accepts
    --format flag (json/table/brief). Table format shows milestone
    IDs, target dates, status, and slack in aligned columns. Brief
    format outputs the path as a chain of milestone IDs.
    Auto-promoted to MCP tool.
  """
  ensures  {
    json_output     "json format returns full CriticalPathPayload"
    table_output    "table format shows milestones with dates and slack"
    brief_output    "brief format shows milestone chain"
    exit_code       "exit 0 on success, exit 1 on error"
    cycle_safe      "returns empty path with message if milestone cycles exist"
  }

  features [pe_surface_contributions]

  verify unit "product:critical-path returns CriticalPathPayload"
  verify unit "empty graph returns empty path"
  verify unit "cycles return empty path with diagnostic message"
}

// ---------------------------------------------------------------------------
// v1.1 CLI surfaces — release, ownership, effort
// ---------------------------------------------------------------------------

behavior surface_list_releases "List Releases CLI" {
  category   command
  types      [ReleaseListResult, ReleaseListEntry, ProductListFilter]
  contract   """
    The CLI command specforge list-releases MUST list all release entities
    with pagination, filtering, and sorting. Auto-promoted to MCP tool.
  """
  ensures  {
    pagination  "offset/limit/has_more computed correctly"
    filtering   "status and tags filters applied before pagination"
  }
  features [pe_surface_contributions]

  verify unit "list-releases returns all releases with default pagination"
  verify unit "list-releases --status=released filters correctly"
  verify unit "list-releases --format=json returns valid JSON"
}

behavior surface_release_deliverables "Release Deliverables CLI" {
  category   command
  types      [ReleaseDeliverablePayload]
  contract   """
    The CLI command specforge release-deliverables <id> MUST return
    deliverables grouped under the specified release.
  """
  features [pe_surface_contributions]

  verify unit "release-deliverables returns correct deliverable list"
  verify unit "release-deliverables with unknown ID returns ENTITY_NOT_FOUND"
}

behavior surface_release_completion "Release Completion CLI" {
  category   command
  types      [ReleaseCompletionPayload]
  contract   """
    The CLI command specforge release-completion <id> MUST return the
    aggregate completion status of a release.
  """
  features [pe_surface_contributions]

  verify unit "release-completion returns correct shipped/total ratio"
}

behavior surface_owner_workload "Owner Workload CLI" {
  category   command
  types      [OwnerWorkloadPayload, OwnerWorkloadEntry]
  contract   """
    The CLI command specforge owner-workload MUST return aggregate
    ownership statistics across features, milestones, deliverables,
    and releases, grouped by owner.
  """
  features [pe_surface_contributions]

  verify unit "owner-workload returns grouped ownership statistics"
  verify unit "owner-workload reports unowned entities"
}

behavior surface_weighted_milestone_completion "Weighted Milestone Completion CLI" {
  category   command
  types      [WeightedMilestoneCompletionPayload]
  contract   """
    The CLI command specforge weighted-milestone-completion <id> MUST
    return the effort-weighted completion for the specified milestone.
  """
  features [pe_surface_contributions]

  verify unit "weighted-milestone-completion returns effort breakdown"
  verify unit "weighted-milestone-completion with unknown ID returns ENTITY_NOT_FOUND"
}

// ════════════════════════════════════════════════════════════════
// CLI Commands — Term Analytics
// ════════════════════════════════════════════════════════════════

behavior surface_term_clusters "Surface: Term Clusters" {
  category   command
  types      [TermClusterPayload, TermCluster, ProductSurfaceError]
  contract   """
    The product:term-clusters CLI command MUST return connected components
    in the TermSeeAlso subgraph. Accepts --format flag (json/table/brief).
    Table format shows cluster ID, term count, and term IDs in aligned
    columns. Brief format outputs cluster_count and isolated_count.
    Auto-promoted to MCP tool.
    Wasm export: cmd__product_term_clusters.
    MCP tool: specforge.product.term-clusters.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_term_clusters"
    json_output     "json format returns full TermClusterPayload"
    table_output    "table format shows per-cluster term lists"
    brief_output    "brief format outputs cluster_count and isolated_count"
    exit_code       "exit 0 on success, exit 1 on error"
  }

  features [pe_surface_contributions]

  verify unit "product:term-clusters returns TermClusterPayload"
  verify unit "no terms returns zero clusters and zero isolated"
}

behavior surface_term_density "Surface: Term Density" {
  category   command
  types      [TermDensityPayload, ProductSurfaceError]
  contract   """
    The product:term-density CLI command MUST return connectivity statistics
    for the TermSeeAlso subgraph. Accepts --format flag (json/table/brief).
    Table format shows total terms, edges, avg connections, hub count,
    and isolated count. Brief format outputs avg_connections and hub count.
    Auto-promoted to MCP tool.
    Wasm export: cmd__product_term_density.
    MCP tool: specforge.product.term-density.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_term_density"
    json_output     "json format returns full TermDensityPayload"
    table_output    "table format shows connectivity statistics"
    brief_output    "brief format outputs avg_connections and hub count"
    exit_code       "exit 0 on success, exit 1 on error"
  }

  features [pe_surface_contributions]

  verify unit "product:term-density returns TermDensityPayload"
  verify unit "empty graph returns zero stats"
}

// ════════════════════════════════════════════════════════════════
// CLI Commands — Module Analytics
// ════════════════════════════════════════════════════════════════

behavior surface_module_dependency_depth "Surface: Module Dependency Depth" {
  category   command
  types      [ModuleDependencyDepthPayload, ProductSurfaceError]
  contract   """
    The product:module-depth <moduleId> CLI command MUST return the longest
    dependency chain from a module. Accepts --format flag (json/table/brief).
    Table format shows depth and chain modules. Brief format outputs depth
    as a single integer. Auto-promoted to MCP tool.
    Wasm export: cmd__product_module_depth.
    MCP tool: specforge.product.module-depth.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_module_dependency_depth"
    json_output     "json format returns full ModuleDependencyDepthPayload"
    table_output    "table format shows depth and chain"
    brief_output    "brief format outputs depth as integer"
    not_found_error "missing moduleId returns ProductSurfaceError with suggestion and exit code 1"
    exit_code       "exit 0 on success, exit 1 on error"
  }

  features [pe_surface_contributions]

  verify unit "product:module-depth returns ModuleDependencyDepthPayload"
  verify unit "missing module returns error with suggestion"
}

behavior surface_module_coupling "Surface: Module Coupling" {
  category   command
  types      [ModuleCouplingPayload, ModuleCouplingEntry, ProductSurfaceError]
  contract   """
    The product:module-coupling CLI command MUST return fan-in/fan-out
    coupling metrics for all modules. Accepts --format flag (json/table/brief).
    Table format shows module ID, fan_in, fan_out, and coupling in aligned
    columns, sorted by coupling descending. Brief format outputs the
    most_coupled_id. Auto-promoted to MCP tool.
    Wasm export: cmd__product_module_coupling.
    MCP tool: specforge.product.module-coupling.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_module_coupling"
    json_output     "json format returns full ModuleCouplingPayload"
    table_output    "table format shows per-module coupling metrics"
    brief_output    "brief format outputs most_coupled_id"
    exit_code       "exit 0 on success, exit 1 on error"
  }

  features [pe_surface_contributions]

  verify unit "product:module-coupling returns ModuleCouplingPayload"
  verify unit "empty graph returns empty modules array"
}

// ════════════════════════════════════════════════════════════════
// CLI Command — Channel Coverage Matrix
// ════════════════════════════════════════════════════════════════

behavior surface_channel_coverage_matrix "Surface: Channel Coverage Matrix" {
  category   command
  types      [ChannelCoverageMatrixPayload, ChannelCoverageEntry, ProductSurfaceError]
  contract   """
    The product:channel-coverage-matrix CLI command MUST return the channel
    coverage matrix showing feature reachability per channel. Accepts
    --format flag (json/table/brief). Table format shows channel ID,
    reachable count, unreachable count, and coverage ratio in aligned
    columns. Brief format outputs overall_coverage as a percentage.
    Symmetric counterpart to product:coverage-matrix (persona).
    Auto-promoted to MCP tool.
    Wasm export: cmd__product_channel_coverage_matrix.
    MCP tool: specforge.product.channel-coverage-matrix.
  """
  requires { graph_ready "product graph is built and in ready state" }
  ensures  {
    delegates       "delegates to pe_query_channel_coverage_matrix"
    json_output     "json format returns full ChannelCoverageMatrixPayload"
    table_output    "table format shows per-channel coverage"
    brief_output    "brief format outputs overall coverage percentage"
    exit_code       "exit 0 on success, exit 1 on error"
  }

  features [pe_surface_contributions]

  verify unit "product:channel-coverage-matrix returns ChannelCoverageMatrixPayload"
  verify unit "no channels returns empty matrix"
}
