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
//   pe_declare_surface_contributions, pe_render_product_entities
//
// behaviors-queries.spec:
//   pe_query_milestone_completion, pe_query_deliverable_traceability,
//   pe_query_journey_coverage, pe_query_feature_ordering,
//   pe_query_milestone_timeline, pe_query_feature_deliverables,
//   pe_query_term_graph, pe_query_deliverable_completion,
//   pe_query_milestone_velocity, pe_query_persona_features,
//   pe_query_feature_impact,
//   pe_query_unscheduled_features, pe_query_feature_overlap,
//   pe_query_persona_coverage_matrix, pe_query_critical_path,
//   pe_query_persona_channels,
//   pe_query_feature_dependents, pe_query_deliverable_dependents,
//   pe_query_deliverable_priority, pe_query_deliverable_personas,
//   pe_query_channel_features,
//   pe_query_term_clusters, pe_query_term_density,
//   pe_query_module_dependency_depth, pe_query_module_coupling,
//   pe_query_channel_coverage_matrix, pe_query_partial_graph
//
// behaviors-v1-1.spec:
//   pe_query_owner_workload, pe_query_weighted_milestone_completion,
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
  payload ProductEntityKindsRegisteredPayload
  channel "product.entity_kinds_registered"
  verify integration "Product Entity Kinds Registered"
}

event pe_edge_types_registered "Product Edge Types Registered" {
  payload ProductEdgeTypesRegisteredPayload
  channel "product.edge_types_registered"
  verify integration "Product Edge Types Registered"
}

event pe_field_definitions_registered "Product Field Definitions Registered" {
  payload ProductFieldsRegisteredPayload
  channel "product.field_definitions_registered"
  verify integration "Product Field Definitions Registered"
}

// pe_query_milestone_timeline was removed as a consumer of this event.
// I058 overdue detection is now query-time only (not validation-time),
// preserving deterministic compilation. See ADR pe_i058_query_time_only.
event pe_validation_complete "Product Validation Complete" {
  payload ProductValidationPayload
  channel "product.validation_complete"
  verify integration "Product Validation Complete"
}

// ── Observability Events ────────────────────────────────────
// These events have no consumers by design. External tooling subscribes
// via channel names at runtime (MCP notifications, CI webhooks, etc.).

event pe_module_cycle_detected "Product Module Cycle Detected" {
  payload ProductCycleDetectedPayload
  channel "product.module_cycle_detected"
  verify integration "Product Module Cycle Detected"
}

event pe_milestone_cycle_detected "Product Milestone Cycle Detected" {
  payload ProductCycleDetectedPayload
  channel "product.milestone_cycle_detected"
  verify integration "Product Milestone Cycle Detected"
}

event pe_feature_cycle_detected "Product Feature Cycle Detected" {
  payload ProductCycleDetectedPayload
  channel "product.feature_cycle_detected"
  verify integration "Product Feature Cycle Detected"
}

// Commands emit no events: a command has no event channel. MCP records
// each one as surface_command_dispatched (ADR 0011).

event pe_deliverable_cycle_detected "Product Deliverable Cycle Detected" {
  payload ProductDeliverableCycleDetectedPayload
  channel "product.deliverable_cycle_detected"
  verify integration "Product Deliverable Cycle Detected"
}

// ── New Query Observability Events ───────────────────────────

// ── New Query Observability Events (Phase 2) ────────────────

// ── Rendering Events ──────────────────────────────────────

event pe_product_entities_rendered "Product Entities Rendered" {
  payload ProductRenderPayload
  channel "product.entities_rendered"
  verify integration "Product Entities Rendered"
}

// ── Validation Rule Observability Events ──────────────────

event pe_validation_rule_fired "Validation Rule Fired" {
  payload ProductValidationRuleFiredPayload
  channel "product.validation_rule_fired"
  verify integration "Validation Rule Fired"
}

event pe_validation_summary "Validation Summary" {
  payload ProductValidationSummaryPayload
  channel "product.validation_summary"
  verify integration "Validation Summary"
}

// ---------------------------------------------------------------------------
// v1.1 events — release, ownership, effort
// ---------------------------------------------------------------------------

event pe_release_cycle_detected "Release Cycle Detected" {
  payload ProductCycleDetectedPayload
  channel "product.release_cycle_detected"
  verify integration "Release Cycle Detected"
}

event pe_release_status_transition_validated "Release Status Transition Validated" {
  payload StatusTransitionViolation
  channel "product.release_status_transition_validated"
  verify integration "Release Status Transition Validated"
}

// ── Term & Module Analytics Events ─────────────────────────

// pe_query_failed is emitted by ANY query behavior (pe_query_*) when it
// returns a ProductQueryError instead of a success payload. All query
// behaviors emit this event on their error path.
