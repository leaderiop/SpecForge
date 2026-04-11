// MCP surface contributions — read-only query resources via specforge:// URIs
//
// This file specifies MCP resource surface contributions for the @specforge/product
// extension. All MCP resources return a ProductSurfaceResponse envelope:
//   { "status": "ok"|"error", "data": <payload>, "error": <error>,
//     "_resource": "<uri>", "_timestamp": "<ISO 8601>" }
//
// URI template parameters map directly to query-port method arguments.
// Resources are read-only (no side effects, no diagnostics emitted).
// See surfaces-cli.spec for CLI commands and surfaces-shared.spec for
// cross-cutting conventions.

use "extensions/product/types"
use "extensions/product/behaviors-registration"
use "extensions/product/behaviors-queries"
use "extensions/product/behaviors-operations"
use "extensions/product/behaviors-v1-1"
use "extensions/product/features"

// ════════════════════════════════════════════════════════════════
// MCP Resources — original read-only query endpoints
// ════════════════════════════════════════════════════════════════

behavior resource_deliverable_traceability "Resource: Deliverable Traceability" {
  category   query
  types      [ProductSurfaceResponse, DeliverableTraceabilityPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/deliverable-traceability/{deliverableId}
    MUST return a ProductSurfaceResponse with data containing a
    DeliverableTraceabilityPayload. Delegates to pe_query_deliverable_traceability.
    Wasm export: mcp__product_deliverable_traceability.
    URI template: specforge://product/deliverable-traceability/{deliverableId}
    Response schema:
      data: { deliverable_id, transitive_features[], journey_path_count, module_path_count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok and _resource=URI"
    data_typed         "data field contains a DeliverableTraceabilityPayload"
    not_found_envelope "non-existent deliverableId returns status=error with ProductSurfaceError"
    timestamp_present  "_timestamp is a valid ISO 8601 datetime"
  }

  features [pe_surface_contributions]

  verify unit "resource returns ProductSurfaceResponse with DeliverableTraceabilityPayload"
  verify unit "non-existent deliverable returns error envelope"
}

behavior resource_feature_deliverables "Resource: Feature Deliverables" {
  category   query
  types      [ProductSurfaceResponse, FeatureDeliverablePayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/feature-deliverables/{featureId}
    MUST return a ProductSurfaceResponse with data containing a
    FeatureDeliverablePayload. Delegates to pe_query_feature_deliverables.
    Wasm export: mcp__product_feature_deliverables.
    URI template: specforge://product/feature-deliverables/{featureId}
    Response schema:
      data: { feature_id, deliverables[], via_journey_count, via_module_count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a FeatureDeliverablePayload"
    not_found_envelope "non-existent featureId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns FeatureDeliverablePayload in envelope"
  verify unit "non-existent feature returns error envelope"
}

behavior resource_feature_milestones "Resource: Feature Milestones" {
  category   query
  types      [ProductSurfaceResponse, FeatureMilestonePayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/feature-milestones/{featureId}
    MUST return a ProductSurfaceResponse with data containing a
    FeatureMilestonePayload. Delegates to pe_query_feature_milestones.
    Wasm export: mcp__product_feature_milestones.
    URI template: specforge://product/feature-milestones/{featureId}
    Response schema:
      data: { feature_id, milestones[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a FeatureMilestonePayload"
    not_found_envelope "non-existent featureId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns FeatureMilestonePayload in envelope"
  verify unit "non-existent feature returns error envelope"
}

behavior resource_persona_journeys "Resource: Persona Journeys" {
  category   query
  types      [ProductSurfaceResponse, PersonaJourneyPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/persona-journeys/{personaId}
    MUST return a ProductSurfaceResponse with data containing a
    PersonaJourneyPayload. Delegates to pe_query_persona_journeys.
    Wasm export: mcp__product_persona_journeys.
    URI template: specforge://product/persona-journeys/{personaId}
    Response schema:
      data: { persona_id, journeys[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a PersonaJourneyPayload"
    not_found_envelope "non-existent personaId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns PersonaJourneyPayload in envelope"
  verify unit "non-existent persona returns error envelope"
}

behavior resource_channel_journeys "Resource: Channel Journeys" {
  category   query
  types      [ProductSurfaceResponse, ChannelJourneyPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/channel-journeys/{channelId}
    MUST return a ProductSurfaceResponse with data containing a
    ChannelJourneyPayload. Delegates to pe_query_channel_journeys.
    Wasm export: mcp__product_channel_journeys.
    URI template: specforge://product/channel-journeys/{channelId}
    Response schema:
      data: { channel_id, journeys[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a ChannelJourneyPayload"
    not_found_envelope "non-existent channelId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns ChannelJourneyPayload in envelope"
  verify unit "non-existent channel returns error envelope"
}

behavior resource_module_deliverables "Resource: Module Deliverables" {
  category   query
  types      [ProductSurfaceResponse, ModuleDeliverablePayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/module-deliverables/{moduleId}
    MUST return a ProductSurfaceResponse with data containing a
    ModuleDeliverablePayload. Delegates to pe_query_module_deliverables.
    Wasm export: mcp__product_module_deliverables.
    URI template: specforge://product/module-deliverables/{moduleId}
    Response schema:
      data: { module_id, deliverables[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a ModuleDeliverablePayload"
    not_found_envelope "non-existent moduleId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns ModuleDeliverablePayload in envelope"
  verify unit "non-existent module returns error envelope"
}

behavior resource_milestone_deliverables "Resource: Milestone Deliverables" {
  category   query
  types      [ProductSurfaceResponse, MilestoneDeliverablePayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/milestone-deliverables/{milestoneId}
    MUST return a ProductSurfaceResponse with data containing a
    MilestoneDeliverablePayload. Delegates to pe_query_milestone_deliverables.
    Wasm export: mcp__product_milestone_deliverables.
    URI template: specforge://product/milestone-deliverables/{milestoneId}
    Response schema:
      data: { milestone_id, deliverables[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a MilestoneDeliverablePayload"
    not_found_envelope "non-existent milestoneId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns MilestoneDeliverablePayload in envelope"
  verify unit "non-existent milestone returns error envelope"
}

behavior resource_module_features "Resource: Module Features" {
  category   query
  types      [ProductSurfaceResponse, ModuleFeaturePayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/module-features/{moduleId}
    MUST return a ProductSurfaceResponse with data containing a
    ModuleFeaturePayload. Delegates to pe_query_module_features.
    Wasm export: mcp__product_module_features.
    URI template: specforge://product/module-features/{moduleId}
    Response schema:
      data: { module_id, features[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a ModuleFeaturePayload"
    not_found_envelope "non-existent moduleId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns ModuleFeaturePayload in envelope"
  verify unit "non-existent module returns error envelope"
}

behavior resource_term_graph "Resource: Term Graph" {
  category   query
  types      [ProductSurfaceResponse, TermGraphPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/term-graph/{termId}?maxHops={n}
    MUST return a ProductSurfaceResponse with data containing a
    TermGraphPayload. Delegates to pe_query_term_graph. The maxHops
    query parameter is optional (default 1, max 5).
    Wasm export: mcp__product_term_graph.
    URI template: specforge://product/term-graph/{termId}
    Query parameters: maxHops (integer, optional, default 1, max 5)
    Response schema:
      data: { term_id, related_terms[], max_hops }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a TermGraphPayload"
    max_hops_default   "maxHops defaults to 1 when query parameter is absent"
    max_hops_capped    "maxHops > 5 is clamped to 5 without error"
    not_found_envelope "non-existent termId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns TermGraphPayload in envelope"
  verify unit "maxHops defaults to 1 when absent"
  verify unit "maxHops > 5 is clamped to 5"
  verify unit "non-existent term returns error envelope"
}

behavior resource_deliverable_completion "Resource: Deliverable Completion" {
  category   query
  types      [ProductSurfaceResponse, DeliverableCompletionPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/deliverable-completion/{deliverableId}
    MUST return a ProductSurfaceResponse with data containing a
    DeliverableCompletionPayload. Delegates to pe_query_deliverable_completion.
    Wasm export: mcp__product_deliverable_completion.
    URI template: specforge://product/deliverable-completion/{deliverableId}
    Response schema:
      data: { deliverable_id, milestone_count, completed_count, completion_ratio, milestone_details[] }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a DeliverableCompletionPayload"
    not_found_envelope "non-existent deliverableId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns DeliverableCompletionPayload in envelope"
  verify unit "non-existent deliverable returns error envelope"
}

behavior resource_persona_channels "Resource: Persona Channels" {
  category   query
  types      [ProductSurfaceResponse, PersonaChannelPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/persona-channels/{personaId}
    MUST return a ProductSurfaceResponse with data containing a
    PersonaChannelPayload. Delegates to pe_query_persona_channels.
    Wasm export: mcp__product_persona_channels.
    URI template: specforge://product/persona-channels/{personaId}
    Response schema:
      data: { persona_id, channels[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a PersonaChannelPayload"
    not_found_envelope "non-existent personaId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns PersonaChannelPayload in envelope"
  verify unit "non-existent persona returns error envelope"
}

behavior resource_journey_deliverables "Resource: Journey Deliverables" {
  category   query
  types      [ProductSurfaceResponse, JourneyDeliverablePayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/journey-deliverables/{journeyId}
    MUST return a ProductSurfaceResponse with data containing a
    JourneyDeliverablePayload. Delegates to pe_query_journey_deliverables.
    Wasm export: mcp__product_journey_deliverables.
    URI template: specforge://product/journey-deliverables/{journeyId}
    Response schema:
      data: { journey_id, deliverables[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a JourneyDeliverablePayload"
    not_found_envelope "non-existent journeyId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns JourneyDeliverablePayload in envelope"
  verify unit "non-existent journey returns error envelope"
}

behavior resource_feature_dependents "Resource: Feature Dependents" {
  category   query
  types      [ProductSurfaceResponse, FeatureDependentPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/feature-dependents/{featureId}
    MUST return a ProductSurfaceResponse with data containing a
    FeatureDependentPayload. Delegates to pe_query_feature_dependents.
    Wasm export: mcp__product_feature_dependents.
    URI template: specforge://product/feature-dependents/{featureId}
    Response schema:
      data: { feature_id, dependents[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a FeatureDependentPayload"
    not_found_envelope "non-existent featureId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns FeatureDependentPayload in envelope"
  verify unit "non-existent feature returns error envelope"
}

behavior resource_deliverable_dependents "Resource: Deliverable Dependents" {
  category   query
  types      [ProductSurfaceResponse, DeliverableDependentPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/deliverable-dependents/{deliverableId}
    MUST return a ProductSurfaceResponse with data containing a
    DeliverableDependentPayload. Delegates to pe_query_deliverable_dependents.
    Wasm export: mcp__product_deliverable_dependents.
    URI template: specforge://product/deliverable-dependents/{deliverableId}
    Response schema:
      data: { deliverable_id, dependents[], count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a DeliverableDependentPayload"
    not_found_envelope "non-existent deliverableId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns DeliverableDependentPayload in envelope"
  verify unit "non-existent deliverable returns error envelope"
}

behavior resource_deliverable_priority "Resource: Deliverable Priority" {
  category   query
  types      [ProductSurfaceResponse, DeliverablePriorityPayload, ProductSurfaceError]
  contract   """
    MCP resource specforge://product/deliverable-priority/{deliverableId}
    MUST return a ProductSurfaceResponse with data containing a
    DeliverablePriorityPayload. Delegates to pe_query_deliverable_priority.
    Wasm export: mcp__product_deliverable_priority.
    URI template: specforge://product/deliverable-priority/{deliverableId}
    Response schema:
      data: { deliverable_id, priority, source_count }
  """
  ensures  {
    envelope_valid     "response is a valid ProductSurfaceResponse with status=ok"
    data_typed         "data field contains a DeliverablePriorityPayload"
    null_priority      "deliverable with no milestones and no journeys returns priority=null"
    not_found_envelope "non-existent deliverableId returns status=error with ProductSurfaceError"
  }

  features [pe_surface_contributions]

  verify unit "resource returns DeliverablePriorityPayload in envelope"
  verify unit "non-existent deliverable returns error envelope"
  verify unit "deliverable with no sources returns null priority"
}

// ════════════════════════════════════════════════════════════════
// MCP Resources — impact and velocity
// ════════════════════════════════════════════════════════════════

behavior resource_persona_features "Resource: Persona Features" {
  category   command
  types      [PersonaFeaturePayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/persona-features/{personaId} MCP resource MUST
    return all features reachable from a persona via multi-hop traversal
    (persona->journey->feature). Returns PersonaFeaturePayload wrapped in
    ProductSurfaceResponse envelope. ENTITY_NOT_FOUND with fuzzy-match
    suggestion if persona does not exist.
  """
  ensures  {
    uri_template  "URI is specforge://product/persona-features/{personaId}"
    payload_typed "data field is PersonaFeaturePayload when status=ok"
    not_found     "missing persona returns status=error with ENTITY_NOT_FOUND"
  }

  features [pe_surface_contributions]

  verify unit "valid persona returns PersonaFeaturePayload"
  verify unit "invalid persona returns ENTITY_NOT_FOUND with suggestion"
}

behavior resource_feature_impact "Resource: Feature Impact" {
  category   command
  types      [FeatureImpactPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/feature-impact/{featureId} MCP resource MUST
    return the transitive impact analysis for a feature: all affected
    journeys, milestones, deliverables, modules, and dependent features.
    Returns FeatureImpactPayload wrapped in ProductSurfaceResponse envelope.
  """
  ensures  {
    uri_template  "URI is specforge://product/feature-impact/{featureId}"
    payload_typed "data field is FeatureImpactPayload when status=ok"
    not_found     "missing feature returns status=error with ENTITY_NOT_FOUND"
  }

  features [pe_surface_contributions]

  verify unit "valid feature returns FeatureImpactPayload with affected entities"
  verify unit "feature with no references returns zero affected entities"
  verify unit "invalid feature returns ENTITY_NOT_FOUND"
}

behavior resource_milestone_velocity "Resource: Milestone Velocity" {
  category   command
  types      [MilestoneVelocityPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/milestone-velocity/{milestoneId} MCP resource
    MUST return velocity metrics for a milestone: feature counts by status,
    completion ratio, days elapsed/remaining, and features per day. Returns
    MilestoneVelocityPayload wrapped in ProductSurfaceResponse envelope.
  """
  ensures  {
    uri_template  "URI is specforge://product/milestone-velocity/{milestoneId}"
    payload_typed "data field is MilestoneVelocityPayload when status=ok"
    not_found     "missing milestone returns ENTITY_NOT_FOUND"
  }

  features [pe_surface_contributions]

  verify unit "valid milestone returns MilestoneVelocityPayload"
  verify unit "milestone with no done features returns null velocity"
  verify unit "invalid milestone returns ENTITY_NOT_FOUND"
}

behavior resource_deliverable_personas "Resource: Deliverable Personas" {
  category   command
  types      [DeliverablePersonaPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/deliverable-personas/{deliverableId} MCP resource
    MUST return the deduplicated set of personas served by a deliverable,
    traversed via deliverable->journey->persona edges. Returns
    DeliverablePersonaPayload wrapped in ProductSurfaceResponse envelope.
    via_journey_ids provides traceability for the traversal path.
  """
  ensures  {
    uri_template  "URI is specforge://product/deliverable-personas/{deliverableId}"
    payload_typed "data field is DeliverablePersonaPayload when status=ok"
    not_found     "missing deliverable returns ENTITY_NOT_FOUND"
    deduplicated  "personas are deduplicated across journeys"
  }

  features [pe_surface_contributions]

  verify unit "deliverable with journeys returns DeliverablePersonaPayload"
  verify unit "deliverable with no journeys returns empty personas"
  verify unit "invalid deliverable returns ENTITY_NOT_FOUND"
}

// ════════════════════════════════════════════════════════════════
// MCP Resources — New Query Resources
// ════════════════════════════════════════════════════════════════

behavior resource_unscheduled_features "Resource: Unscheduled Features" {
  category   resource
  types      [UnscheduledFeaturesPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The unscheduled-features MCP resource MUST expose the
    queryUnscheduledFeatures result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/unscheduled-features"
    payload_typed "data field is UnscheduledFeaturesPayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns UnscheduledFeaturesPayload"
}

behavior resource_feature_overlap "Resource: Feature Overlap" {
  category   resource
  types      [FeatureOverlapPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The feature-overlap MCP resource MUST expose the queryFeatureOverlap
    result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/feature-overlap"
    payload_typed "data field is FeatureOverlapPayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns FeatureOverlapPayload"
}

behavior resource_persona_coverage_matrix "Resource: Persona Coverage Matrix" {
  category   resource
  types      [PersonaCoverageMatrixPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The persona-coverage-matrix MCP resource MUST expose the
    queryPersonaCoverageMatrix result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/persona-coverage-matrix"
    payload_typed "data field is PersonaCoverageMatrixPayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns PersonaCoverageMatrixPayload"
}

behavior resource_critical_path "Resource: Critical Path" {
  category   resource
  types      [CriticalPathPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The critical-path MCP resource MUST expose the queryCriticalPath
    result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/critical-path"
    payload_typed "data field is CriticalPathPayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns CriticalPathPayload"
}

behavior resource_milestone_completion "Resource: Milestone Completion" {
  category   resource
  types      [MilestoneCompletionPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The milestone-completion MCP resource MUST expose the
    queryMilestoneCompletion result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/milestone-completion/{milestoneId}"
    payload_typed "data field is MilestoneCompletionPayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns MilestoneCompletionPayload"
}

behavior resource_journey_coverage "Resource: Journey Coverage" {
  category   resource
  types      [JourneyCoveragePayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The journey-coverage MCP resource MUST expose the
    queryJourneyCoverage result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/journey-coverage/{journeyId}"
    payload_typed "data field is JourneyCoveragePayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns JourneyCoveragePayload"
}

behavior resource_feature_ordering "Resource: Feature Ordering" {
  category   resource
  types      [FeatureOrderingPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The feature-ordering MCP resource MUST expose the
    queryFeatureOrdering result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/feature-ordering"
    payload_typed "data field is FeatureOrderingPayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns FeatureOrderingPayload"
}

behavior resource_milestone_timeline "Resource: Milestone Timeline" {
  category   resource
  types      [MilestoneTimelinePayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The milestone-timeline MCP resource MUST expose the
    queryMilestoneTimeline result as a ProductSurfaceResponse.
  """
  ensures  {
    uri_template  "URI is specforge://product/milestone-timeline"
    payload_typed "data field is MilestoneTimelinePayload when status=ok"
  }

  features [pe_surface_contributions]

  verify unit "resource returns MilestoneTimelinePayload"
}

// ---------------------------------------------------------------------------
// v1.1 MCP resources — release, ownership, effort
// ---------------------------------------------------------------------------

behavior resource_releases "MCP Resource: Release List" {
  category   resource
  types      [ReleaseListResult, ProductSurfaceResponse]
  contract   """
    The MCP resource specforge://product/releases MUST return all release
    entities as a read-only resource.
  """
  ensures  {
    uri_template  "URI is specforge://product/releases"
  }
  features [pe_surface_contributions]

  verify unit "MCP resource returns release list in ProductSurfaceResponse envelope"
}

behavior resource_release_deliverables "MCP Resource: Release Deliverables" {
  category   resource
  types      [ReleaseDeliverablePayload, ProductSurfaceResponse]
  contract   """
    The MCP resource specforge://product/releases/{id}/deliverables MUST
    return deliverables for the specified release.
  """
  ensures  {
    uri_template  "URI is specforge://product/releases/{id}/deliverables"
  }
  features [pe_surface_contributions]

  verify unit "MCP resource returns deliverables for valid release ID"
}

behavior resource_release_milestones "MCP Resource: Release Milestones" {
  category   resource
  types      [ReleaseMilestonePayload, ProductSurfaceResponse]
  contract   """
    The MCP resource specforge://product/releases/{id}/milestones MUST
    return milestones targeted by the specified release.
  """
  ensures  {
    uri_template  "URI is specforge://product/releases/{id}/milestones"
  }
  features [pe_surface_contributions]

  verify unit "MCP resource returns milestones for valid release ID"
}

behavior resource_release_completion "MCP Resource: Release Completion" {
  category   resource
  types      [ReleaseCompletionPayload, ProductSurfaceResponse]
  contract   """
    The MCP resource specforge://product/releases/{id}/completion MUST
    return aggregate completion status.
  """
  ensures  {
    uri_template  "URI is specforge://product/releases/{id}/completion"
  }
  features [pe_surface_contributions]

  verify unit "MCP resource returns completion ratio for valid release ID"
}

behavior resource_owner_workload "MCP Resource: Owner Workload" {
  category   resource
  types      [OwnerWorkloadPayload, ProductSurfaceResponse]
  contract   """
    The MCP resource specforge://product/owner-workload MUST return
    aggregate ownership statistics.
  """
  ensures  {
    uri_template  "URI is specforge://product/owner-workload"
  }
  features [pe_surface_contributions]

  verify unit "MCP resource returns ownership breakdown"
}

behavior resource_weighted_completion "MCP Resource: Weighted Milestone Completion" {
  category   resource
  types      [WeightedMilestoneCompletionPayload, ProductSurfaceResponse]
  contract   """
    The MCP resource specforge://product/milestones/{id}/weighted-completion
    MUST return effort-weighted milestone completion.
  """
  ensures  {
    uri_template  "URI is specforge://product/milestones/{id}/weighted-completion"
  }
  features [pe_surface_contributions]

  verify unit "MCP resource returns weighted completion for valid milestone"
}

// ════════════════════════════════════════════════════════════════
// MCP Resources — channel-features (closing query gap)
// ════════════════════════════════════════════════════════════════

behavior resource_channel_features "Resource: Channel Features" {
  category   resource
  types      [ChannelFeaturePayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/channel-features/{channelId} MCP resource MUST
    return all features reachable from a channel via multi-hop traversal
    (channel->journey->feature). Returns ChannelFeaturePayload wrapped in
    ProductSurfaceResponse envelope. ENTITY_NOT_FOUND with fuzzy-match
    suggestion if channel does not exist.
  """
  ensures  {
    uri_template  "URI is specforge://product/channel-features/{channelId}"
    payload_typed "data field is ChannelFeaturePayload when status=ok"
    not_found     "missing channel returns status=error with ENTITY_NOT_FOUND"
  }

  features [pe_surface_contributions]

  verify unit "valid channel returns ChannelFeaturePayload"
  verify unit "invalid channel returns ENTITY_NOT_FOUND with suggestion"
}

// ════════════════════════════════════════════════════════════════
// MCP Resources — Term & Module Analytics
// ════════════════════════════════════════════════════════════════

behavior resource_term_clusters "Resource: Term Clusters" {
  category   resource
  types      [TermClusterPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/term-clusters MCP resource MUST return connected
    components in the TermSeeAlso subgraph. Delegates to pe_query_term_clusters.
    Returns TermClusterPayload wrapped in ProductSurfaceResponse envelope.
    Wasm export: mcp__product_term_clusters.
    URI: specforge://product/term-clusters
    Response schema:
      data: { clusters[], cluster_count, isolated_count, total_terms }
  """
  ensures  {
    uri_template  "URI is specforge://product/term-clusters"
    payload_typed "data field is TermClusterPayload when status=ok"
    no_params     "resource takes no parameters"
  }

  features [pe_surface_contributions]

  verify unit "resource returns TermClusterPayload in envelope"
  verify unit "empty term graph returns zero clusters"
}

behavior resource_term_density "Resource: Term Density" {
  category   resource
  types      [TermDensityPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/term-density MCP resource MUST return connectivity
    statistics for the TermSeeAlso subgraph. Delegates to pe_query_term_density.
    Returns TermDensityPayload wrapped in ProductSurfaceResponse envelope.
    Wasm export: mcp__product_term_density.
    URI: specforge://product/term-density
    Response schema:
      data: { total_terms, total_see_also, avg_connections, max_connections,
              hub_terms[], isolated_terms[] }
  """
  ensures  {
    uri_template  "URI is specforge://product/term-density"
    payload_typed "data field is TermDensityPayload when status=ok"
    no_params     "resource takes no parameters"
  }

  features [pe_surface_contributions]

  verify unit "resource returns TermDensityPayload in envelope"
  verify unit "empty term graph returns zero stats"
}

behavior resource_module_dependency_depth "Resource: Module Dependency Depth" {
  category   resource
  types      [ModuleDependencyDepthPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/module-depth/{moduleId} MCP resource MUST return
    the longest dependency chain from a module. Delegates to
    pe_query_module_dependency_depth. Returns ModuleDependencyDepthPayload
    wrapped in ProductSurfaceResponse envelope.
    Wasm export: mcp__product_module_depth.
    URI template: specforge://product/module-depth/{moduleId}
    Response schema:
      data: { module_id, depth, longest_chain[] }
  """
  ensures  {
    uri_template     "URI is specforge://product/module-depth/{moduleId}"
    payload_typed    "data field is ModuleDependencyDepthPayload when status=ok"
    not_found        "missing moduleId returns status=error with ENTITY_NOT_FOUND"
  }

  features [pe_surface_contributions]

  verify unit "resource returns ModuleDependencyDepthPayload in envelope"
  verify unit "non-existent module returns ENTITY_NOT_FOUND"
}

behavior resource_module_coupling "Resource: Module Coupling" {
  category   resource
  types      [ModuleCouplingPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The specforge://product/module-coupling MCP resource MUST return
    fan-in/fan-out coupling metrics for all modules. Delegates to
    pe_query_module_coupling. Returns ModuleCouplingPayload wrapped in
    ProductSurfaceResponse envelope.
    Wasm export: mcp__product_module_coupling.
    URI: specforge://product/module-coupling
    Response schema:
      data: { modules[], avg_fan_in, avg_fan_out, most_coupled_id, total_modules }
  """
  ensures  {
    uri_template  "URI is specforge://product/module-coupling"
    payload_typed "data field is ModuleCouplingPayload when status=ok"
    no_params     "resource takes no parameters"
  }

  features [pe_surface_contributions]

  verify unit "resource returns ModuleCouplingPayload in envelope"
  verify unit "empty module graph returns empty modules"
}

// ════════════════════════════════════════════════════════════════
// MCP Resource — Channel Coverage Matrix
// ════════════════════════════════════════════════════════════════

behavior resource_channel_coverage_matrix "Resource: Channel Coverage Matrix" {
  category   resource
  types      [ChannelCoverageMatrixPayload, ProductSurfaceResponse, ProductSurfaceError]
  contract   """
    The channel-coverage-matrix MCP resource MUST expose the
    queryChannelCoverageMatrix result as a ProductSurfaceResponse.
    Symmetric counterpart to resource_persona_coverage_matrix.
    Wasm export: mcp__product_channel_coverage_matrix.
    URI: specforge://product/channel-coverage-matrix
    Response schema:
      data: { channels[], total_features, overall_coverage }
  """
  ensures  {
    uri_template  "URI is specforge://product/channel-coverage-matrix"
    payload_typed "data field is ChannelCoverageMatrixPayload when status=ok"
    no_params     "resource takes no parameters"
  }

  features [pe_surface_contributions]

  verify unit "resource returns ChannelCoverageMatrixPayload in envelope"
  verify unit "empty channel graph returns empty channels array"
}
