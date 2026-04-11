// @specforge/formal coverage tracking — coverage tracking items + info diagnostics
//
// Moved from @specforge/software formal-proofs.spec per 10-expert panel.
// Terminology: "Verification Obligations" -> "Coverage Tracking Items"
// All behavior IDs renamed se_ -> fa_
// CoverageDischargeStatus extended with test_written and test_failing
// W035 aggregated: single summary per compilation with breakdown by kind

use "extensions/formal/invariants"
use "extensions/formal/types"
use "types/zero-entity-core"

behavior fa_coverage_tracking_pass "Coverage Tracking Compiler Pass" {
  category command
  types    [ConditionEntry, RefinementChain, CoverageTrackingItem, CoverageTrackingKind]
  produces  [fa_coverage_items_generated]
  contract """
    Generate machine-readable coverage tracking items after all
    analysis passes complete.
  """
  requires {
    all_passes_done        "condition_check, layering_verify, event_graph_analyze passes completed"
  }
  ensures  {
    condition_items        "condition_coverage items generated (requires/ensures hold)"
    invariant_items        "invariant_coverage items generated (maintains hold)"
    layering_items         "layering_coverage items generated (concrete satisfies abstract)"
    axiom_excluded         "axiom entities are explicitly excluded — axioms are assumed true, no coverage tracking items generated"
    layering_entity_items  "layering_coverage items generated for refinement entities (condition delta verified)"
    process_items          "process_coverage items generated for process entities (alphabet completeness, composition safety)"
    json_output            "each item emitted as structured JSON: entity ID, kind, description, discharge status"
  }

  features [fa_coverage_tracking]

  verify unit "condition_coverage items generated"
  verify unit "invariant_coverage items generated"
  verify unit "layering_coverage items generated"
  verify unit "items emitted as structured JSON"
}

behavior fa_track_coverage_discharge "Track Coverage Item Discharge" {
  category query
  types    [CoverageTrackingItem, CoverageDischargeStatus]
  contract """
    Track which coverage items are discharged by existing tests
    or by static analysis heuristics. Auto-discharge is OFF by default;
    entities or conditions must opt in via @auto-discharge-eligible.
    Extended status model: pending -> test_written -> test_failing ->
    test_covered (or heuristic_ok for opted-in auto-discharge).
  """
  requires {
    items_generated        "coverage tracking items have been generated"
  }
  ensures  {
    test_written_tracked   "item with associated test file transitions to test_written"
    test_failing_tracked   "item with failing test transitions to test_failing"
    test_discharge         "item covered by passing verify/test transitions to test_covered"
    analysis_discharge     "item discharged by static analysis (when opted in) transitions to heuristic_ok"
    undischarged_summary   "undischarged items produce single W035 summary per compilation with breakdown by kind (condition/invariant/layering) and link to drill-down command (specforge analyze coverage)"
    opt_in_required        "auto-discharge only applies to entities or conditions annotated @auto-discharge-eligible"
    tautological_auto      "opted-in postcondition that is tautologically true auto-discharges to heuristic_ok"
    enforced_invariant_auto "opted-in invariant with enforces referencing matching maintains condition auto-discharges to heuristic_ok"
    layering_superset_auto "opted-in concrete behavior whose ensures is a strict superset of abstract ensures auto-discharges layering_coverage to heuristic_ok"
    tautological_criteria     "tautological auto-discharge applies only when: (a) postcondition uses no identifiers from requires, (b) description matches trivial patterns, (c) condition has no side-effect verbs — this is a closed list of criteria"
  }

  features [fa_coverage_tracking]

  verify unit "item with test file transitions to test_written"
  verify unit "item with failing test transitions to test_failing"
  verify unit "item discharged by test transitions to test_covered"
  verify unit "item discharged by analysis transitions to heuristic_ok"
  verify unit "undischarged items produce single W035 summary"
  verify unit "W035 summary includes breakdown by kind"
  verify unit "tautological postcondition auto-discharges (when opted in)"
  verify unit "enforced invariant with matching maintains auto-discharges (when opted in)"
  verify unit "concrete ensures superset of abstract ensures auto-discharges (when opted in)"
  verify unit "non-trivial item remains pending without test"
  verify unit "@auto-discharge-eligible annotation enables auto-discharge"
  verify unit "entity without @auto-discharge-eligible skips all auto-discharge heuristics"
}

// ── Info Diagnostics ─────────────────────────────────────────

behavior fa_emit_coverage_item_covered_info "I008: Coverage Item Covered by Test" {
  category query
  types    [CoverageTrackingItem]
  contract """
    When a coverage tracking item transitions to test_covered status,
    emit I008 info diagnostic confirming which test discharges the
    item.
  """
  ensures  {
    info_emitted           "test_covered item emits I008 with test name"
  }

  features [fa_coverage_tracking]

  verify unit "coverage item covered by test produces I008"
}

behavior fa_emit_no_structural_cycles_info "I009: No Structural Cycles Detected" {
  category query
  contract """
    When the event_graph_analyze pass confirms no unmitigated cycles
    exist in the event graph, emit I009 info diagnostic. Note: this is
    structural analysis only — runtime deadlocks from dynamic conditions
    are not detected.
  """
  ensures  {
    info_emitted           "cycle-free event graph emits I009"
  }

  features [fa_coverage_tracking]

  verify unit "cycle-free event graph produces I009"
}

behavior fa_emit_formal_analysis_available "I015: Formal Analysis Available" {
  category query
  contract """
    When the @specforge/formal extension detects behaviors with
    requires/ensures blocks, emit I015 info suggesting that the user
    run specforge analyze for deeper formal analysis.
  """
  ensures  {
    info_emitted           "presence of condition blocks emits I015 suggesting specforge analyze"
  }

  features [fa_coverage_tracking]

  verify unit "behaviors with requires/ensures trigger I015 info"
}

// ── Specification Depth Detection ────────────────────────────

behavior fa_detect_specification_depth "I014: Specification Depth Level" {
  category query
  types    [SpecificationDepthLevel, CoverageTrackingItem]
  contract """
    Compute the per-entity specification depth level using the
    SpecificationDepthLevel enum and emit I014 info for entities at
    Level 2 (conditions) or above. Level 4 (proofs) requires
    test_covered items — heuristic_ok alone is insufficient.
  """
  requires {
    all_passes_done        "condition_check, layering_verify, event_graph_analyze, coverage_tracking passes completed"
  }
  ensures  {
    level_computed         "each entity's depth level computed: prose (no structured data), entity_graph (has edges), conditions (has requires/ensures), invariants (has maintains + invariant refs), proofs (all items test_covered — heuristic_ok alone insufficient)"
    multi_dimensional      "each entity also assessed on orthogonal dimensions: condition_depth, invariant_coverage, test_evidence"
    info_emitted           "entity at Level 2+ emits I014 with current level and next-level suggestion"
    level_zero_silent      "entity at Level 0 (prose) emits no depth diagnostic"
    level_one_silent       "entity at Level 1 (entity_graph) emits no depth diagnostic"
    adoption_nudge         "when >5 behaviors are at Level 0-1, I014 includes suggestion to adopt requires/ensures on critical behaviors"
    auto_discharge_audit   "I014 includes count of heuristic_ok items for review"
  }

  features [fa_coverage_tracking]

  verify unit "entity with requires/ensures computes as Level 2 (conditions)"
  verify unit "entity with maintains and invariant refs computes as Level 3 (invariants)"
  verify unit "entity with all items test_covered computes as Level 4 (proofs)"
  verify unit "entity with only heuristic_ok items does NOT reach Level 4"
  verify unit "entity at Level 2+ emits I014"
  verify unit "entity at Level 0 emits no depth diagnostic"
  verify unit ">5 prose-only behaviors triggers adoption nudge in I014"
}
