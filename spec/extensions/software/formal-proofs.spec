// @specforge/software formal proofs — proof obligations + info diagnostics

use types/zero-entity-core
use extensions/software/types
use extensions/software/invariants

behavior se_proof_obligation_pass "Proof Obligation Compiler Pass" {
  category command
  types [ContractCondition, RefinementChain]

  contract """
    Generate machine-readable verification conditions after all
    analysis passes complete.
  """

  requires {
    all_passes_done        "contract_check, refinement_verify, process_analyze passes completed"
  }

  ensures {
    contract_obligations   "contract_preservation obligations generated (requires/ensures hold)"
    invariant_obligations  "invariant_preservation obligations generated (maintains hold)"
    refinement_obligations "refinement_correctness obligations generated (concrete satisfies abstract)"
    json_output            "each obligation emitted as structured JSON: entity ID, kind, description, discharge status"
  }

  verify unit "contract_preservation obligations generated"
  verify unit "invariant_preservation obligations generated"
  verify unit "refinement_correctness obligations generated"
  verify unit "obligations emitted as structured JSON"

}

behavior se_track_proof_discharge "Track Proof Obligation Discharge" {
  category query
  types [ProofObligation, ProofDischargeStatus]

  contract """
    Track which proof obligations are discharged by existing tests
    or by static analysis heuristics.
  """

  requires {
    obligations_generated  "proof obligations have been generated"
  }

  ensures {
    test_discharge         "obligation covered by verify/test transitions to test_verified"
    analysis_discharge     "obligation discharged by static analysis transitions to auto_proved"
    undischarged_warned    "undischarged obligation produces W035 warning"
    tautological_auto      "postcondition that is tautologically true (e.g. 'result is returned') auto-discharges to auto_proved"
    enforced_invariant_auto "invariant with enforced_by referencing a behavior that has a matching maintains condition auto-discharges to auto_proved"
    refinement_superset_auto "concrete behavior whose ensures is a strict superset of abstract ensures auto-discharges refinement_correctness to auto_proved"
  }

  verify unit "obligation discharged by test transitions to test_verified"
  verify unit "obligation discharged by analysis transitions to auto_proved"
  verify unit "undischarged obligation produces W035"
  verify unit "tautological postcondition auto-discharges"
  verify unit "enforced invariant with matching maintains auto-discharges"
  verify unit "concrete ensures superset of abstract ensures auto-discharges"
  verify unit "non-trivial obligation remains pending without test"

}

// ── Info Diagnostics (RES-25) ────────────────────────────────

behavior se_emit_proof_verified_info "I008: Proof Obligation Verified by Test" {
  category query
  types [ProofObligation]

  contract """
    When a proof obligation transitions to test_verified status, emit
    I008 info diagnostic confirming which test discharges the obligation.
  """

  ensures {
    info_emitted           "test_verified obligation emits I008 with test name"
  }

  verify unit "proof obligation verified by test produces I008"

}

behavior se_emit_deadlock_freedom_info "I009: Deadlock Freedom Verified" {
  category query

  contract """
    When the process_analyze pass confirms no deadlocks exist in the
    event graph, emit I009 info diagnostic confirming deadlock freedom.
  """

  ensures {
    info_emitted           "deadlock-free event graph emits I009"
  }

  verify unit "deadlock-free event graph produces I009"

}

behavior se_emit_formal_analysis_available "I015: Formal Analysis Available" {
  category query

  contract """
    When the @specforge/software extension detects behaviors with
    requires/ensures blocks, emit I015 info suggesting that the user
    run specforge analyze for deeper formal analysis. I015 is allocated
    to @specforge/software (not core).
  """

  ensures {
    info_emitted           "presence of contract blocks emits I015 suggesting specforge analyze"
  }

  verify unit "behaviors with requires/ensures trigger I015 info"

}

// ── R5: Progressive Formality Gate ───────────────────────────

behavior se_detect_formality_level "I014: Progressive Formality Level" {
  category query
  types [FormalityLevel, ProofObligation]

  contract """
    Compute the per-entity formality level using the FormalityLevel
    enum and emit I014 info for entities at Level 2 (contracts) or
    above. This enables progressive adoption tracking.
  """

  requires {
    all_passes_done        "contract_check, refinement_verify, process_analyze, proof_obligation passes completed"
  }

  ensures {
    level_computed         "each entity's formality level computed: prose (no structured data), entity_graph (has edges), contracts (has requires/ensures), invariants (has maintains + invariant refs), proofs (all obligations discharged)"
    info_emitted           "entity at Level 2+ emits I014 with current level and next-level suggestion"
    level_zero_silent      "entity at Level 0 (prose) emits no formality diagnostic"
    level_one_silent       "entity at Level 1 (entity_graph) emits no formality diagnostic"
  }

  verify unit "entity with requires/ensures computes as Level 2 (contracts)"
  verify unit "entity with maintains and invariant refs computes as Level 3 (invariants)"
  verify unit "entity with all obligations discharged computes as Level 4 (proofs)"
  verify unit "entity at Level 2+ emits I014"
  verify unit "entity at Level 0 emits no formality diagnostic"

}
