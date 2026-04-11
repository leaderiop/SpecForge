// @specforge/formal extension failure modes — FMEA risk analysis
//
// Moved from @specforge/software per 10-expert panel. These failure modes
// are specific to formal analysis concerns (structured conditions,
// specification layering, event graph linting, coverage tracking).

use "extensions/formal/invariants"

failure_mode silent_deadlock_miss "Silent Cycle Miss" {
  invariant       fa_condition_consistency
  severity        8
  occurrence      2
  detection       5
  rpn             80
  cause           "Tarjan SCC algorithm fails to detect a circular dependency because the bipartite graph construction omits barrier edges from sync blocks, creating a false negative in cycle detection"
  effect          "Cycle exists in the event graph but specforge analyze event-graph reports clean — production system deadlocks under concurrent load; agent receives incorrect 'cycle-free' context"
  mitigation      "Property-based tests generate random event graphs with known cycles and verify detection; barrier edges explicitly included in bipartite graph via fa_build_event_bipartite_graph; regression test for every reported false negative"
  post_mitigation {
    severity   8
    occurrence 2
    detection  3
    rpn        48
  }
}

failure_mode false_positive_layering_violation "False Positive Layering Condition Mismatch" {
  invariant       fa_condition_consistency
  severity        6
  occurrence      3
  detection       4
  rpn             72
  cause           "Condition check pass incorrectly flags a valid layering as a condition mismatch because the postcondition comparison uses string equality instead of named-condition set inclusion"
  effect          "Developer receives spurious E031 errors on valid layering, loses trust in formal analysis, and disables the condition_check pass"
  mitigation      "Postcondition comparison uses structured condition names (not prose descriptions) for set inclusion checks; integration tests with semantically equivalent but textually different conditions; escape hatch via @suppress annotation"
  post_mitigation {
    severity   6
    occurrence 2
    detection  3
    rpn        36
  }
}

failure_mode coverage_tracking_leak "Coverage Tracking Item Leak" {
  invariant       fa_condition_consistency
  severity        7
  occurrence      2
  detection       5
  rpn             70
  cause           "Coverage tracking pass fails to generate an item for a contract condition because the condition is in a maintains block that the pass does not traverse — maintains conditions are only checked for invariant_coverage but not for condition_coverage"
  effect          "A contract condition exists without a corresponding coverage tracking item — it appears verified when it has never been checked, creating a false sense of completeness"
  mitigation      "Exhaustive item generation: every condition in requires, ensures, and maintains produces exactly one item; count assertion: total items >= total conditions across all contract blocks; integration test with maintains-only entities"
  post_mitigation {
    severity   7
    occurrence 2
    detection  3
    rpn        42
  }
}

failure_mode event_graph_analysis_timeout "Event Graph Analysis Timeout" {
  invariant       fa_condition_consistency
  severity        5
  occurrence      3
  detection       2
  rpn             30
  cause           "Large event graph with many sync barriers causes the event_graph_analyze pass to exceed the 30-second timeout, producing an incomplete EventGraphAnalysisReport with timed_out=true"
  effect          "Some event flow sub-analyses (cycle, retry cycle, connectivity) are omitted from the report — user sees partial results without clear indication of what was skipped"
  mitigation      "Timeout warning includes explicit list of incomplete sub-analyses; timed_out field in EventGraphAnalysisReport enables programmatic detection; --timeout CLI flag allows increasing the limit for large projects; incremental analysis caches previous results"
  post_mitigation {
    severity   5
    occurrence 2
    detection  2
    rpn        20
  }
}

failure_mode protocol_ordering_false_positive "Protocol Ordering False Positive" {
  invariant       fa_condition_consistency
  severity        5
  occurrence      3
  detection       3
  rpn             45
  cause           "Protocol ordering validation flags a valid event sequence as conflicting because the topological sort of the event graph does not match the protocol's declared ordering — even though both orderings are valid linearizations of the same partial order"
  effect          "Developer receives spurious W068 warnings on valid protocol configurations, loses trust in protocol validation, and removes ordering declarations"
  mitigation      "Ordering validation uses partial order comparison (not linear equality) — protocol ordering is treated as a constraint set, not a total order; any topologically valid linearization passes; integration tests with known-valid partial orders"
  post_mitigation {
    severity   5
    occurrence 2
    detection  2
    rpn        20
  }
}

failure_mode axiom_circularity_risk "Axiom Circularity Risk" {
  invariant       fa_condition_consistency
  severity        7
  occurrence      2
  detection       4
  rpn             56
  cause           "User creates axioms that implicitly depend on each other through their referenced conditions — axiom A's conditions assume axiom B, and axiom B's conditions assume axiom A — creating a circular foundation that undermines the assumed-true guarantee"
  effect          "The formal foundation appears solid but rests on circular reasoning — conditions are 'satisfied' by axioms that ultimately depend on themselves; agents receive incorrect 'axiom-supported' context"
  mitigation      "Cycle detection on the condition→axiom→condition graph: AssumedBy edges transitively analyzed for cycles; circular axiom chains produce a warning; integration tests with known-circular axiom configurations"
  post_mitigation {
    severity   7
    occurrence 2
    detection  2
    rpn        28
  }
}

failure_mode false_negative_condition_check "False Negative Condition Check" {
  invariant       fa_condition_consistency
  severity        8
  occurrence      2
  detection       6
  rpn             96
  cause           "Condition check pass fails to flag a real layering condition mismatch because postcondition comparison uses string equality — a semantically weakened postcondition with different wording passes undetected"
  effect          "Refined behavior silently weakens its parent's guarantees — downstream consumers rely on postconditions that the concrete implementation does not actually satisfy; agent generates code trusting incorrect conditions"
  mitigation      "Structured condition names (not prose descriptions) for set inclusion checks; mandatory layering_coverage items for all refined behaviors; integration tests with known-weak layering that must be caught; auto-discharge requires opt-in via @auto-discharge-eligible"
  post_mitigation {
    severity   8
    occurrence 2
    detection  3
    rpn        48
  }
}

failure_mode refinement_delta_drift "Refinement Condition Delta Drift" {
  invariant       fa_condition_consistency
  severity        6
  occurrence      3
  detection       4
  rpn             72
  cause           "Refinement entity's condition delta becomes stale when abstract/concrete behavior conditions are modified"
  effect          "Agent receives incorrect condition delta context; generated code may not preserve abstract guarantees"
  mitigation      "W071 detects refinements without deltas; layering_verify cross-checks delta against actual behavior conditions"
  post_mitigation {
    severity   6
    occurrence 2
    detection  3
    rpn        36
  }
}

failure_mode process_alphabet_incomplete "Process Alphabet Incomplete" {
  invariant       fa_condition_consistency
  severity        5
  occurrence      3
  detection       3
  rpn             45
  cause           "Process alphabet does not include all events that participate — events added to system but not to process"
  effect          "Event graph analysis misses cross-process interactions; deadlock detection has blind spots"
  mitigation      "W074 detects empty alphabets; bipartite graph cross-references actual event edges against declared alphabets"
  post_mitigation {
    severity   5
    occurrence 2
    detection  2
    rpn        20
  }
}
