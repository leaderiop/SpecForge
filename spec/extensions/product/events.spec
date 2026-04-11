// @specforge/product extension events — observability and orchestration hooks
//
// Events emitted by the product extension during validation and
// traceability computation. Events fall into two categories:
//
// 1. Orchestration events (with consumers): trigger downstream behaviors
//    that depend on registration or computation completing first.
// 2. Observability events (empty consumers): emitted on named channels
//    for external tooling (MCP notifications, dashboards, CI integrations)
//    to subscribe without coupling to the extension internals.
//
// ── Trigger-to-Source Cross-Reference ────────────────────────
//
// Every event's `trigger` field references a behavior defined in one of
// the imported files. This index maps trigger IDs to their source file
// for implementer navigation:
//
// behaviors-registration.spec:
//   pe_register_entity_kinds, pe_register_edge_types,
//   pe_register_field_definitions, pe_register_validation_rules
//
// behaviors-operations.spec:
//   pe_declare_surface_contributions, pe_render_product_entities,
//   pe_emit_validation_rule_details
//
// behaviors-queries.spec:
//   pe_query_milestone_completion, pe_query_deliverable_traceability,
//   pe_query_journey_coverage, pe_query_feature_ordering,
//   pe_query_milestone_timeline, pe_query_feature_deliverables,
//   pe_query_feature_milestones, pe_query_persona_journeys,
//   pe_query_channel_journeys, pe_query_module_deliverables,
//   pe_query_milestone_deliverables, pe_query_module_features,
//   pe_query_term_graph, pe_query_deliverable_completion,
//   pe_query_milestone_velocity, pe_query_persona_features,
//   pe_query_feature_impact,
//   pe_query_unscheduled_features, pe_query_feature_overlap,
//   pe_query_persona_coverage_matrix, pe_query_critical_path,
//   pe_query_persona_channels, pe_query_journey_deliverables,
//   pe_query_feature_dependents, pe_query_deliverable_dependents,
//   pe_query_deliverable_priority, pe_query_deliverable_personas,
//   pe_query_channel_features,
//   pe_query_term_clusters, pe_query_term_density,
//   pe_query_module_dependency_depth, pe_query_module_coupling,
//   pe_query_channel_coverage_matrix, pe_query_partial_graph
//
// behaviors-v1-1.spec:
//   pe_query_owner_workload, pe_query_weighted_milestone_completion,
//   pe_query_release_deliverables, pe_query_release_milestones,
//   pe_query_release_completion
//
// validation-structural.spec:
//   detect_module_cycles, detect_milestone_cycles,
//   detect_feature_dependency_cycles, detect_deliverable_cycles
//
// validation-lifecycle.spec:
//   detect_release_dependency_cycles, validate_release_status_transition

use "extensions/product/types"

// ── Orchestration Events ────────────────────────────────────
// Registration chain: kinds → edges → fields → validation rules

event pe_entity_kinds_registered "Product Entity Kinds Registered" {
  trigger  pe_register_entity_kinds
  payload  ProductEntityKindsRegisteredPayload
  channel  "product.entity_kinds_registered"
  consumers [pe_register_edge_types]

  verify integration "Product Entity Kinds Registered"
}

event pe_edge_types_registered "Product Edge Types Registered" {
  trigger  pe_register_edge_types
  payload  ProductEdgeTypesRegisteredPayload
  channel  "product.edge_types_registered"
  consumers [pe_register_field_definitions]

  verify integration "Product Edge Types Registered"
}

event pe_field_definitions_registered "Product Field Definitions Registered" {
  trigger  pe_register_field_definitions
  payload  ProductFieldsRegisteredPayload
  channel  "product.field_definitions_registered"
  consumers [pe_register_validation_rules]

  verify integration "Product Field Definitions Registered"
}

// pe_query_milestone_timeline was removed as a consumer of this event.
// I058 overdue detection is now query-time only (not validation-time),
// preserving deterministic compilation. See ADR pe_i058_query_time_only.
event pe_validation_complete "Product Validation Complete" {
  trigger  pe_register_validation_rules
  payload  ProductValidationPayload
  channel  "product.validation_complete"
  consumers []

  verify integration "Product Validation Complete"
}

// ── Observability Events ────────────────────────────────────
// These events have no consumers by design. External tooling subscribes
// via channel names at runtime (MCP notifications, CI webhooks, etc.).

event pe_traceability_computed "Product Traceability Computed" {
  trigger  pe_register_validation_rules
  payload  ProductTraceabilityPayload
  channel  "product.traceability_computed"
  consumers []

  verify integration "Product Traceability Computed"
}

event pe_module_cycle_detected "Product Module Cycle Detected" {
  trigger  detect_module_cycles
  payload  ProductCycleDetectedPayload
  channel  "product.module_cycle_detected"
  consumers []

  verify integration "Product Module Cycle Detected"
}

event pe_milestone_cycle_detected "Product Milestone Cycle Detected" {
  trigger  detect_milestone_cycles
  payload  ProductCycleDetectedPayload
  channel  "product.milestone_cycle_detected"
  consumers []

  verify integration "Product Milestone Cycle Detected"
}

event pe_feature_cycle_detected "Product Feature Cycle Detected" {
  trigger  detect_feature_dependency_cycles
  payload  ProductCycleDetectedPayload
  channel  "product.feature_cycle_detected"
  consumers []

  verify integration "Product Feature Cycle Detected"
}

// ── Query Observability Events ──────────────────────────────
// Emitted when graph queries complete, allowing external tooling
// to observe query activity and results.

event pe_milestone_completion_queried "Milestone Completion Queried" {
  trigger  pe_query_milestone_completion
  payload  MilestoneCompletionPayload
  channel  "product.milestone_completion_queried"
  consumers []

  verify integration "Milestone Completion Queried"
}

event pe_journey_coverage_queried "Journey Coverage Queried" {
  trigger  pe_query_journey_coverage
  payload  JourneyCoveragePayload
  channel  "product.journey_coverage_queried"
  consumers []

  verify integration "Journey Coverage Queried"
}

event pe_deliverable_cycle_detected "Product Deliverable Cycle Detected" {
  trigger  detect_deliverable_cycles
  payload  ProductDeliverableCycleDetectedPayload
  channel  "product.deliverable_cycle_detected"
  consumers []

  verify integration "Product Deliverable Cycle Detected"
}

event pe_feature_ordering_queried "Feature Ordering Queried" {
  trigger  pe_query_feature_ordering
  payload  FeatureOrderingPayload
  channel  "product.feature_ordering_queried"
  consumers []

  verify integration "Feature Ordering Queried"
}

event pe_milestone_timeline_queried "Milestone Timeline Queried" {
  trigger  pe_query_milestone_timeline
  payload  MilestoneTimelinePayload
  channel  "product.milestone_timeline_queried"
  consumers []

  verify integration "Milestone Timeline Queried"
}

event pe_deliverable_traceability_queried "Deliverable Traceability Queried" {
  trigger  pe_query_deliverable_traceability
  payload  DeliverableTraceabilityPayload
  channel  "product.deliverable_traceability_queried"
  consumers []

  verify integration "Deliverable Traceability Queried"
}

event pe_feature_deliverables_queried "Feature Deliverables Queried" {
  trigger  pe_query_feature_deliverables
  payload  FeatureDeliverablePayload
  channel  "product.feature_deliverables_queried"
  consumers []

  verify integration "Feature Deliverables Queried"
}

event pe_feature_milestones_queried "Feature Milestones Queried" {
  trigger  pe_query_feature_milestones
  payload  FeatureMilestonePayload
  channel  "product.feature_milestones_queried"
  consumers []

  verify integration "Feature Milestones Queried"
}

event pe_persona_journeys_queried "Persona Journeys Queried" {
  trigger  pe_query_persona_journeys
  payload  PersonaJourneyPayload
  channel  "product.persona_journeys_queried"
  consumers []

  verify integration "Persona Journeys Queried"
}

event pe_channel_journeys_queried "Channel Journeys Queried" {
  trigger  pe_query_channel_journeys
  payload  ChannelJourneyPayload
  channel  "product.channel_journeys_queried"
  consumers []

  verify integration "Channel Journeys Queried"
}

event pe_module_deliverables_queried "Module Deliverables Queried" {
  trigger  pe_query_module_deliverables
  payload  ModuleDeliverablePayload
  channel  "product.module_deliverables_queried"
  consumers []

  verify integration "Module Deliverables Queried"
}

event pe_milestone_deliverables_queried "Milestone Deliverables Queried" {
  trigger  pe_query_milestone_deliverables
  payload  MilestoneDeliverablePayload
  channel  "product.milestone_deliverables_queried"
  consumers []

  verify integration "Milestone Deliverables Queried"
}

event pe_module_features_queried "Module Features Queried" {
  trigger  pe_query_module_features
  payload  ModuleFeaturePayload
  channel  "product.module_features_queried"
  consumers []

  verify integration "Module Features Queried"
}

event pe_term_graph_queried "Term Graph Queried" {
  trigger  pe_query_term_graph
  payload  TermGraphPayload
  channel  "product.term_graph_queried"
  consumers []

  verify integration "Term Graph Queried"
}

event pe_deliverable_completion_queried "Deliverable Completion Queried" {
  trigger  pe_query_deliverable_completion
  payload  DeliverableCompletionPayload
  channel  "product.deliverable_completion_queried"
  consumers []

  verify integration "Deliverable Completion Queried"
}

event pe_persona_channels_queried "Persona Channels Queried" {
  trigger  pe_query_persona_channels
  payload  PersonaChannelPayload
  channel  "product.persona_channels_queried"
  consumers []

  verify integration "Persona Channels Queried"
}

event pe_journey_deliverables_queried "Journey Deliverables Queried" {
  trigger  pe_query_journey_deliverables
  payload  JourneyDeliverablePayload
  channel  "product.journey_deliverables_queried"
  consumers []

  verify integration "Journey Deliverables Queried"
}

event pe_feature_dependents_queried "Feature Dependents Queried" {
  trigger  pe_query_feature_dependents
  payload  FeatureDependentPayload
  channel  "product.feature_dependents_queried"
  consumers []

  verify integration "Feature Dependents Queried"
}

event pe_deliverable_dependents_queried "Deliverable Dependents Queried" {
  trigger  pe_query_deliverable_dependents
  payload  DeliverableDependentPayload
  channel  "product.deliverable_dependents_queried"
  consumers []

  verify integration "Deliverable Dependents Queried"
}

event pe_deliverable_priority_queried "Deliverable Priority Queried" {
  trigger  pe_query_deliverable_priority
  payload  DeliverablePriorityPayload
  channel  "product.deliverable_priority_queried"
  consumers []

  verify integration "Deliverable Priority Queried"
}

// ── Surface Operation Events ────────────────────────────────
// Emitted when CLI commands or MCP resources are invoked, allowing
// external tooling to observe surface access patterns and latency.
//
// Note: These events are emitted by the surface dispatch layer at runtime
// (any cmd__* or mcp__* Wasm export), not by pe_declare_surface_contributions
// which only registers surfaces at startup. The trigger field references
// pe_declare_surface_contributions because it is the behavior that establishes
// the surface contracts — at runtime, the compiler's surface dispatcher emits
// these events after each invocation completes or fails.

event pe_cli_command_executed "CLI Command Executed" {
  trigger  pe_declare_surface_contributions
  payload  ProductSurfaceOperationPayload
  channel  "product.cli_command_executed"
  consumers []

  verify integration "CLI Command Executed"
}

event pe_mcp_resource_accessed "MCP Resource Accessed" {
  trigger  pe_declare_surface_contributions
  payload  ProductSurfaceOperationPayload
  channel  "product.mcp_resource_accessed"
  consumers []

  verify integration "MCP Resource Accessed"
}

// ── New Query Observability Events ───────────────────────────

event pe_persona_features_queried "Persona Features Queried" {
  trigger  pe_query_persona_features
  payload  PersonaFeaturePayload
  channel  "product.persona_features_queried"
  consumers []

  verify integration "Persona Features Queried"
}

event pe_feature_impact_queried "Feature Impact Queried" {
  trigger  pe_query_feature_impact
  payload  FeatureImpactPayload
  channel  "product.feature_impact_queried"
  consumers []

  verify integration "Feature Impact Queried"
}

event pe_milestone_velocity_queried "Milestone Velocity Queried" {
  trigger  pe_query_milestone_velocity
  payload  MilestoneVelocityPayload
  channel  "product.milestone_velocity_queried"
  consumers []

  verify integration "Milestone Velocity Queried"
}

event pe_deliverable_personas_queried "Deliverable Personas Queried" {
  trigger  pe_query_deliverable_personas
  payload  DeliverablePersonaPayload
  channel  "product.deliverable_personas_queried"
  consumers []

  verify integration "Deliverable Personas Queried"
}

// ── New Query Observability Events (Phase 2) ────────────────

event pe_unscheduled_features_queried "Unscheduled Features Queried" {
  trigger  pe_query_unscheduled_features
  payload  UnscheduledFeaturesPayload
  channel  "product.unscheduled_features_queried"
  consumers []

  verify integration "Unscheduled Features Queried"
}

event pe_feature_overlap_queried "Feature Overlap Queried" {
  trigger  pe_query_feature_overlap
  payload  FeatureOverlapPayload
  channel  "product.feature_overlap_queried"
  consumers []

  verify integration "Feature Overlap Queried"
}

event pe_persona_coverage_matrix_queried "Persona Coverage Matrix Queried" {
  trigger  pe_query_persona_coverage_matrix
  payload  PersonaCoverageMatrixPayload
  channel  "product.persona_coverage_matrix_queried"
  consumers []

  verify integration "Persona Coverage Matrix Queried"
}

event pe_critical_path_queried "Critical Path Queried" {
  trigger  pe_query_critical_path
  payload  CriticalPathPayload
  channel  "product.critical_path_queried"
  consumers []

  verify integration "Critical Path Queried"
}

// ── Rendering Events ──────────────────────────────────────

event pe_product_entities_rendered "Product Entities Rendered" {
  trigger  pe_render_product_entities
  payload  ProductRenderPayload
  channel  "product.entities_rendered"
  consumers []

  verify integration "Product Entities Rendered"
}

// ── Validation Rule Observability Events ──────────────────

event pe_validation_rule_fired "Validation Rule Fired" {
  trigger  pe_emit_validation_rule_details
  payload  ProductValidationRuleFiredPayload
  channel  "product.validation_rule_fired"
  consumers []

  verify integration "Validation Rule Fired"
}

event pe_validation_summary "Validation Summary" {
  trigger  pe_emit_validation_rule_details
  payload  ProductValidationSummaryPayload
  channel  "product.validation_summary"
  consumers []

  verify integration "Validation Summary"
}

// ---------------------------------------------------------------------------
// v1.1 events — release, ownership, effort
// ---------------------------------------------------------------------------

event pe_release_cycle_detected "Release Cycle Detected" {
  trigger    detect_release_dependency_cycles
  payload    ProductCycleDetectedPayload
  channel    "product.release_cycle_detected"
  consumers  []

  verify integration "Release Cycle Detected"
}

event pe_owner_workload_queried "Owner Workload Queried" {
  trigger    pe_query_owner_workload
  payload    OwnerWorkloadPayload
  channel    "product.owner_workload_queried"
  consumers  []

  verify integration "Owner Workload Queried"
}

event pe_weighted_completion_queried "Weighted Milestone Completion Queried" {
  trigger    pe_query_weighted_milestone_completion
  payload    WeightedMilestoneCompletionPayload
  channel    "product.weighted_completion_queried"
  consumers  []

  verify integration "Weighted Milestone Completion Queried"
}

event pe_release_deliverables_queried "Release Deliverables Queried" {
  trigger    pe_query_release_deliverables
  payload    ReleaseDeliverablePayload
  channel    "product.release_deliverables_queried"
  consumers  []

  verify integration "Release Deliverables Queried"
}

event pe_release_milestones_queried "Release Milestones Queried" {
  trigger    pe_query_release_milestones
  payload    ReleaseMilestonePayload
  channel    "product.release_milestones_queried"
  consumers  []

  verify integration "Release Milestones Queried"
}

event pe_release_completion_queried "Release Completion Queried" {
  trigger    pe_query_release_completion
  payload    ReleaseCompletionPayload
  channel    "product.release_completion_queried"
  consumers  []

  verify integration "Release Completion Queried"
}

event pe_channel_features_queried "Channel Features Queried" {
  trigger    pe_query_channel_features
  payload    ChannelFeaturePayload
  channel    "product.channel_features_queried"
  consumers  []

  verify integration "Channel Features Queried"
}

event pe_release_status_transition_validated "Release Status Transition Validated" {
  trigger    validate_release_status_transition
  payload    StatusTransitionViolation
  channel    "product.release_status_transition_validated"
  consumers  []

  verify integration "Release Status Transition Validated"
}

// ── Term & Module Analytics Events ─────────────────────────

event pe_term_clusters_queried "Term Clusters Queried" {
  trigger    pe_query_term_clusters
  payload    TermClusterPayload
  channel    "product.query_observability"
  consumers  []

  verify integration "Term Clusters Queried"
}

event pe_term_density_queried "Term Density Queried" {
  trigger    pe_query_term_density
  payload    TermDensityPayload
  channel    "product.term_density_queried"
  consumers  []

  verify integration "Term Density Queried"
}

event pe_module_dependency_depth_queried "Module Dependency Depth Queried" {
  trigger    pe_query_module_dependency_depth
  payload    ModuleDependencyDepthPayload
  channel    "product.query_observability"
  consumers  []

  verify integration "Module Dependency Depth Queried"
}

event pe_module_coupling_queried "Module Coupling Queried" {
  trigger    pe_query_module_coupling
  payload    ModuleCouplingPayload
  channel    "product.query_observability"
  consumers  []

  verify integration "Module Coupling Queried"
}

event pe_channel_coverage_matrix_queried "Channel Coverage Matrix Queried" {
  trigger    pe_query_channel_coverage_matrix
  payload    ChannelCoverageMatrixPayload
  channel    "product.query_observability"
  consumers  []

  verify integration "Channel Coverage Matrix Queried"
}

// ── Error Events ─────────────────────────────────────────────
// Emitted when queries or surface operations fail. These complement the
// success-path observability events above, enabling external tooling to
// subscribe to failures for alerting, debugging, and reliability monitoring.

// pe_query_failed is emitted by ANY query behavior (pe_query_*) when it
// returns a ProductQueryError instead of a success payload. All query
// behaviors emit this event on their error path.
event pe_query_failed "Query Failed" {
  trigger    pe_query_milestone_completion
  payload    ProductQueryFailedPayload
  channel    "product.errors"
  consumers  []

  verify integration "Query Failed"
}

// pe_surface_error is emitted by the surface dispatch layer when a CLI
// command or MCP resource invocation produces a ProductSurfaceError
// response. Emitted alongside the success-path pe_cli_command_executed
// or pe_mcp_resource_accessed event (which fires regardless of outcome),
// providing a dedicated error channel for alerting.
event pe_surface_error "Surface Operation Error" {
  trigger    pe_declare_surface_contributions
  payload    ProductSurfaceFailedPayload
  channel    "product.errors"
  consumers  []

  verify integration "Surface Operation Error"
}
