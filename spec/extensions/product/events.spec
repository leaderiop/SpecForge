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
  payload  ProductEntityKindsRegisteredPayload
  channel  "product.entity_kinds_registered"

  verify integration "Product Entity Kinds Registered"
}

event pe_edge_types_registered "Product Edge Types Registered" {
  payload  ProductEdgeTypesRegisteredPayload
  channel  "product.edge_types_registered"

  verify integration "Product Edge Types Registered"
}

event pe_field_definitions_registered "Product Field Definitions Registered" {
  payload  ProductFieldsRegisteredPayload
  channel  "product.field_definitions_registered"

  verify integration "Product Field Definitions Registered"
}

// pe_query_milestone_timeline was removed as a consumer of this event.
// I058 overdue detection is now query-time only (not validation-time),
// preserving deterministic compilation. See ADR pe_i058_query_time_only.
event pe_validation_complete "Product Validation Complete" {
  payload  ProductValidationPayload
  channel  "product.validation_complete"

  verify integration "Product Validation Complete"
}

// ── Observability Events ────────────────────────────────────
// These events have no consumers by design. External tooling subscribes
// via channel names at runtime (MCP notifications, CI webhooks, etc.).

event pe_traceability_computed "Product Traceability Computed" {
  payload  ProductTraceabilityPayload
  channel  "product.traceability_computed"

  verify integration "Product Traceability Computed"
}

event pe_module_cycle_detected "Product Module Cycle Detected" {
  payload  ProductCycleDetectedPayload
  channel  "product.module_cycle_detected"

  verify integration "Product Module Cycle Detected"
}

event pe_milestone_cycle_detected "Product Milestone Cycle Detected" {
  payload  ProductCycleDetectedPayload
  channel  "product.milestone_cycle_detected"

  verify integration "Product Milestone Cycle Detected"
}

event pe_feature_cycle_detected "Product Feature Cycle Detected" {
  payload  ProductCycleDetectedPayload
  channel  "product.feature_cycle_detected"

  verify integration "Product Feature Cycle Detected"
}

// ── Query Observability Events ──────────────────────────────
// Emitted when graph queries complete, allowing external tooling
// to observe query activity and results.

event pe_milestone_completion_queried "Milestone Completion Queried" {
  payload  MilestoneCompletionPayload
  channel  "product.milestone_completion_queried"

  verify integration "Milestone Completion Queried"
}

event pe_journey_coverage_queried "Journey Coverage Queried" {
  payload  JourneyCoveragePayload
  channel  "product.journey_coverage_queried"

  verify integration "Journey Coverage Queried"
}

event pe_deliverable_cycle_detected "Product Deliverable Cycle Detected" {
  payload  ProductDeliverableCycleDetectedPayload
  channel  "product.deliverable_cycle_detected"

  verify integration "Product Deliverable Cycle Detected"
}

event pe_feature_ordering_queried "Feature Ordering Queried" {
  payload  FeatureOrderingPayload
  channel  "product.feature_ordering_queried"

  verify integration "Feature Ordering Queried"
}

event pe_milestone_timeline_queried "Milestone Timeline Queried" {
  payload  MilestoneTimelinePayload
  channel  "product.milestone_timeline_queried"

  verify integration "Milestone Timeline Queried"
}

event pe_deliverable_traceability_queried "Deliverable Traceability Queried" {
  payload  DeliverableTraceabilityPayload
  channel  "product.deliverable_traceability_queried"

  verify integration "Deliverable Traceability Queried"
}

event pe_feature_deliverables_queried "Feature Deliverables Queried" {
  payload  FeatureDeliverablePayload
  channel  "product.feature_deliverables_queried"

  verify integration "Feature Deliverables Queried"
}

event pe_feature_milestones_queried "Feature Milestones Queried" {
  payload  FeatureMilestonePayload
  channel  "product.feature_milestones_queried"

  verify integration "Feature Milestones Queried"
}

event pe_persona_journeys_queried "Persona Journeys Queried" {
  payload  PersonaJourneyPayload
  channel  "product.persona_journeys_queried"

  verify integration "Persona Journeys Queried"
}

event pe_channel_journeys_queried "Channel Journeys Queried" {
  payload  ChannelJourneyPayload
  channel  "product.channel_journeys_queried"

  verify integration "Channel Journeys Queried"
}

event pe_module_deliverables_queried "Module Deliverables Queried" {
  payload  ModuleDeliverablePayload
  channel  "product.module_deliverables_queried"

  verify integration "Module Deliverables Queried"
}

event pe_milestone_deliverables_queried "Milestone Deliverables Queried" {
  payload  MilestoneDeliverablePayload
  channel  "product.milestone_deliverables_queried"

  verify integration "Milestone Deliverables Queried"
}

event pe_module_features_queried "Module Features Queried" {
  payload  ModuleFeaturePayload
  channel  "product.module_features_queried"

  verify integration "Module Features Queried"
}

event pe_term_graph_queried "Term Graph Queried" {
  payload  TermGraphPayload
  channel  "product.term_graph_queried"

  verify integration "Term Graph Queried"
}

event pe_deliverable_completion_queried "Deliverable Completion Queried" {
  payload  DeliverableCompletionPayload
  channel  "product.deliverable_completion_queried"

  verify integration "Deliverable Completion Queried"
}

event pe_persona_channels_queried "Persona Channels Queried" {
  payload  PersonaChannelPayload
  channel  "product.persona_channels_queried"

  verify integration "Persona Channels Queried"
}

event pe_journey_deliverables_queried "Journey Deliverables Queried" {
  payload  JourneyDeliverablePayload
  channel  "product.journey_deliverables_queried"

  verify integration "Journey Deliverables Queried"
}

event pe_feature_dependents_queried "Feature Dependents Queried" {
  payload  FeatureDependentPayload
  channel  "product.feature_dependents_queried"

  verify integration "Feature Dependents Queried"
}

event pe_deliverable_dependents_queried "Deliverable Dependents Queried" {
  payload  DeliverableDependentPayload
  channel  "product.deliverable_dependents_queried"

  verify integration "Deliverable Dependents Queried"
}

event pe_deliverable_priority_queried "Deliverable Priority Queried" {
  payload  DeliverablePriorityPayload
  channel  "product.deliverable_priority_queried"

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
  payload  ProductSurfaceOperationPayload
  channel  "product.cli_command_executed"

  verify integration "CLI Command Executed"
}

event pe_mcp_resource_accessed "MCP Resource Accessed" {
  payload  ProductSurfaceOperationPayload
  channel  "product.mcp_resource_accessed"

  verify integration "MCP Resource Accessed"
}

// ── New Query Observability Events ───────────────────────────

event pe_persona_features_queried "Persona Features Queried" {
  payload  PersonaFeaturePayload
  channel  "product.persona_features_queried"

  verify integration "Persona Features Queried"
}

event pe_feature_impact_queried "Feature Impact Queried" {
  payload  FeatureImpactPayload
  channel  "product.feature_impact_queried"

  verify integration "Feature Impact Queried"
}

event pe_milestone_velocity_queried "Milestone Velocity Queried" {
  payload  MilestoneVelocityPayload
  channel  "product.milestone_velocity_queried"

  verify integration "Milestone Velocity Queried"
}

event pe_deliverable_personas_queried "Deliverable Personas Queried" {
  payload  DeliverablePersonaPayload
  channel  "product.deliverable_personas_queried"

  verify integration "Deliverable Personas Queried"
}

// ── New Query Observability Events (Phase 2) ────────────────

event pe_unscheduled_features_queried "Unscheduled Features Queried" {
  payload  UnscheduledFeaturesPayload
  channel  "product.unscheduled_features_queried"

  verify integration "Unscheduled Features Queried"
}

event pe_feature_overlap_queried "Feature Overlap Queried" {
  payload  FeatureOverlapPayload
  channel  "product.feature_overlap_queried"

  verify integration "Feature Overlap Queried"
}

event pe_persona_coverage_matrix_queried "Persona Coverage Matrix Queried" {
  payload  PersonaCoverageMatrixPayload
  channel  "product.persona_coverage_matrix_queried"

  verify integration "Persona Coverage Matrix Queried"
}

event pe_critical_path_queried "Critical Path Queried" {
  payload  CriticalPathPayload
  channel  "product.critical_path_queried"

  verify integration "Critical Path Queried"
}

// ── Rendering Events ──────────────────────────────────────

event pe_product_entities_rendered "Product Entities Rendered" {
  payload  ProductRenderPayload
  channel  "product.entities_rendered"

  verify integration "Product Entities Rendered"
}

// ── Validation Rule Observability Events ──────────────────

event pe_validation_rule_fired "Validation Rule Fired" {
  payload  ProductValidationRuleFiredPayload
  channel  "product.validation_rule_fired"

  verify integration "Validation Rule Fired"
}

event pe_validation_summary "Validation Summary" {
  payload  ProductValidationSummaryPayload
  channel  "product.validation_summary"

  verify integration "Validation Summary"
}

// ---------------------------------------------------------------------------
// v1.1 events — release, ownership, effort
// ---------------------------------------------------------------------------

event pe_release_cycle_detected "Release Cycle Detected" {
  payload    ProductCycleDetectedPayload
  channel    "product.release_cycle_detected"

  verify integration "Release Cycle Detected"
}

event pe_owner_workload_queried "Owner Workload Queried" {
  payload    OwnerWorkloadPayload
  channel    "product.owner_workload_queried"

  verify integration "Owner Workload Queried"
}

event pe_weighted_completion_queried "Weighted Milestone Completion Queried" {
  payload    WeightedMilestoneCompletionPayload
  channel    "product.weighted_completion_queried"

  verify integration "Weighted Milestone Completion Queried"
}

event pe_release_deliverables_queried "Release Deliverables Queried" {
  payload    ReleaseDeliverablePayload
  channel    "product.release_deliverables_queried"

  verify integration "Release Deliverables Queried"
}

event pe_release_milestones_queried "Release Milestones Queried" {
  payload    ReleaseMilestonePayload
  channel    "product.release_milestones_queried"

  verify integration "Release Milestones Queried"
}

event pe_release_completion_queried "Release Completion Queried" {
  payload    ReleaseCompletionPayload
  channel    "product.release_completion_queried"

  verify integration "Release Completion Queried"
}

event pe_channel_features_queried "Channel Features Queried" {
  payload    ChannelFeaturePayload
  channel    "product.channel_features_queried"

  verify integration "Channel Features Queried"
}

event pe_release_status_transition_validated "Release Status Transition Validated" {
  payload    StatusTransitionViolation
  channel    "product.release_status_transition_validated"

  verify integration "Release Status Transition Validated"
}

// ── Term & Module Analytics Events ─────────────────────────

event pe_term_clusters_queried "Term Clusters Queried" {
  payload    TermClusterPayload
  channel    "product.query_observability"

  verify integration "Term Clusters Queried"
}

event pe_term_density_queried "Term Density Queried" {
  payload    TermDensityPayload
  channel    "product.term_density_queried"

  verify integration "Term Density Queried"
}

event pe_module_dependency_depth_queried "Module Dependency Depth Queried" {
  payload    ModuleDependencyDepthPayload
  channel    "product.query_observability"

  verify integration "Module Dependency Depth Queried"
}

event pe_module_coupling_queried "Module Coupling Queried" {
  payload    ModuleCouplingPayload
  channel    "product.query_observability"

  verify integration "Module Coupling Queried"
}

event pe_channel_coverage_matrix_queried "Channel Coverage Matrix Queried" {
  payload    ChannelCoverageMatrixPayload
  channel    "product.query_observability"

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
  payload    ProductQueryFailedPayload
  channel    "product.errors"

  verify integration "Query Failed"
}

// pe_surface_error is emitted by the surface dispatch layer when a CLI
// command or MCP resource invocation produces a ProductSurfaceError
// response. Emitted alongside the success-path pe_cli_command_executed
// or pe_mcp_resource_accessed event (which fires regardless of outcome),
// providing a dedicated error channel for alerting.
event pe_surface_error "Surface Operation Error" {
  payload    ProductSurfaceFailedPayload
  channel    "product.errors"

  verify integration "Surface Operation Error"
}
