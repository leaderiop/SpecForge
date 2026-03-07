// @specforge/software formal concurrency — CSP

use types/zero-entity-core
use extensions/software/types
use extensions/software/invariants

behavior se_parse_sync_block "Parse Sync Block" {
  category command
  types [SyncBlock, SoftwareEvent]

  contract """
    Recognize the sync { } block on event entities as CSP
    synchronization constraints.
  """

  ensures {
    barrier_parsed         "sync block with barrier (list of behavior references) parsed"
    timeout_parsed         "sync block with timeout (duration string with description) parsed"
    non_event_warned       "sync block on non-event entity produces warning"
  }

  verify unit "sync block with barrier parsed"
  verify unit "sync block with timeout parsed"
  verify unit "sync block on non-event produces warning"

}

// ── R8: Bipartite graph construction ─────────────────────────

behavior se_build_event_bipartite_graph "Build Event-Behavior Bipartite Graph" {
  category command
  types [SoftwareEvent, SoftwareBehavior, SyncBlock]

  contract """
    Construct the event-behavior bipartite graph from Produces and
    Consumes edges. This graph is the input to all CSP analysis
    sub-passes (deadlock, livelock, starvation, channel checks).
  """

  requires {
    edges_built            "graph with Produces/Consumes edges is fully built"
    events_exist           "at least one event entity exists in the graph"
  }

  ensures {
    bipartite_nodes        "bipartite graph contains event nodes and behavior nodes as two disjoint sets"
    producers_included     "every behavior with a Produces edge appears as a producer node"
    consumers_included     "every behavior with a Consumes edge appears as a consumer node"
    barrier_edges          "sync block barrier references create additional synchronization edges"
    no_orphan_nodes        "bipartite graph contains no orphan nodes disconnected from all edges"
  }

  verify unit "bipartite graph built from produces/consumes edges"
  verify unit "barrier references create synchronization edges"
  verify property "bipartite graph has no orphan nodes"
  verify property "all producers and consumers are included"

}

behavior se_detect_event_deadlocks "E034: Detect Event Deadlocks" {
  category query
  types [SoftwareEvent, SyncBlock]

  contract """
    Detect circular event dependencies that may cause deadlocks
    using Tarjan's SCC algorithm on the event-behavior bipartite graph.
  """

  requires {
    bipartite_graph_built  "event-behavior bipartite graph is constructed"
  }

  ensures {
    cycle_detected         "circular event dependency produces E034 error"
    non_cycle_passes       "non-circular event dependency produces no diagnostic"
    cycle_path_shown       "E034 includes full cycle path in diagnostic message"
  }

  verify unit "circular event dependency detected as E034"
  verify unit "non-circular event dependency passes"
  verify unit "E034 includes full cycle path in diagnostic"

}

behavior se_detect_channel_type_mismatch "E035: Channel Type Mismatch" {
  category query
  types [SoftwareEvent, SoftwareTypeDef]

  contract """
    Verify that event producers and consumers agree on payload type.
  """

  requires {
    payload_declared       "event has a payload type reference"
  }

  ensures {
    matching_passes        "matching producer/consumer payload types produce no diagnostic"
    mismatch_error         "mismatching payload types produce E035 error"
  }

  verify unit "matching producer/consumer payload types pass"
  verify unit "mismatching payload types produce E035"

}

behavior se_detect_unmatched_producers "W029: Unmatched Producers" {
  category query
  types [SoftwareEvent]

  contract """
    Detect events that have producers but no consumers. This is a warning
    rather than an error because fire-and-forget events, future consumers,
    and external system consumers are all valid patterns.
  """

  ensures {
    matched_passes         "event with producers and consumers produces no diagnostic"
    unmatched_warned       "event with no consumers produces W029 warning"
  }

  verify unit "event with producers and consumers passes"
  verify unit "event with no consumers produces W029"

}

behavior se_detect_unbounded_channel "W034: Unbounded Channel Buffer" {
  category query
  types [SoftwareEvent, SyncBlock]

  contract """
    Detect events with no sync timeout or buffer limit, which may
    accumulate unbounded messages under load.
  """

  ensures {
    unbounded_detected     "event channel with no timeout and no buffer limit produces W034"
    bounded_passes         "event channel with timeout or buffer limit produces no diagnostic"
  }

  verify unit "event channel with no sync timeout produces W034"
  verify unit "event channel with sync timeout passes"

}

behavior se_detect_starvation_risk "W033: Starvation Risk" {
  category query
  types [SoftwarePort, SoftwareEvent]

  contract """
    Detect ports with unfair access patterns where one consumer
    may be starved by another.
  """

  ensures {
    starvation_detected    "port with unfair access pattern produces W033"
    fair_passes            "port with fair access or single consumer passes"
  }

  verify unit "port with unfair access pattern produces W033"
  verify unit "port with single consumer passes"

}

behavior se_detect_livelock_risk "W032: Livelock Risk" {
  category query
  types [SoftwareEvent, SyncBlock]

  contract """
    Detect potential livelocks: event chains where a consumer
    re-triggers the same event without backoff or termination.
  """

  ensures {
    retrigger_detected     "re-triggering without backoff produces W032 warning"
    backoff_passes         "re-triggering with timeout/backoff produces no diagnostic"
  }

  verify unit "re-triggering without backoff detected as W032"
  verify unit "re-triggering with timeout/backoff passes"

}

behavior se_process_analyze_pass "Process Analyze Compiler Pass" {
  category command
  types [SoftwareEvent, SyncBlock, SoftwareTypeDef, ConcurrencyAnalysisReport]

  contract """
    The process_analyze compiler pass performs full CSP analysis over
    the event-behavior graph after refinement_verify.
  """

  requires {
    refinement_verify_done "refinement_verify pass has completed"
    graph_constructed      "entity graph with produces/consumes edges is built"
  }

  ensures {
    bipartite_delegated    "event-behavior bipartite graph construction delegated to se_build_event_bipartite_graph"
    deadlock_checked       "Tarjan SCC deadlock detection runs (E034)"
    channel_checked        "channel type compatibility checked (E035)"
    unmatched_checked      "unmatched producers detected (W029)"
    livelock_checked       "livelock risks detected (W032)"
    starvation_checked     "starvation risks on ports with unfair access detected (W033)"
    buffer_checked         "unbounded channel buffers detected (W034)"
    timeout_handled        "barrier timeout expiry sets timed_out=true on ConcurrencyAnalysisReport and emits warning with incomplete sub-analysis count"
    partial_results        "completed sub-analyses included in report; only incomplete ones omitted on timeout"
  }

  verify unit "bipartite graph construction delegated to se_build_event_bipartite_graph"
  verify unit "deadlock detection runs on SCC"
  verify unit "channel type check runs on all events"
  verify unit "pass runs after refinement_verify"
  verify unit "barrier timeout sets timed_out=true and emits warning"
  verify unit "partial results included on timeout"

}
