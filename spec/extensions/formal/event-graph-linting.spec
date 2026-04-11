// @specforge/formal event graph linting — structural event flow analysis
//
// Moved from @specforge/software formal-concurrency.spec per 10-expert panel.
// Terminology: "CSP Event Flow Analysis" -> "Event Graph Linting"
// E034: now checks for mitigations before firing (sync.timeout, @idempotent, circuit_breaker)
// W032: "Livelock Risk" -> "Unmitigated Retry Cycle"
// W033: "Starvation Risk" -> "Asymmetric Connectivity Warning"
// All behavior IDs renamed se_ -> fa_

use "extensions/formal/invariants"
use "extensions/formal/types"
use "types/zero-entity-core"

behavior fa_parse_sync_block "Parse Sync Block" {
  category command
  types    [SyncBlock, DeliverySemantics]
  contract """
    Recognize the sync { } block on event entities as synchronization
    constraints. Supports barrier (behavior references), timeout
    (duration string), and delivery semantics.
  """
  ensures  {
    barrier_parsed         "sync block with barrier (list of behavior references) parsed"
    timeout_parsed         "sync block with timeout (duration string with description) parsed"
    delivery_parsed        "sync block with delivery (at_most_once | at_least_once | exactly_once) parsed"
    non_event_warned       "sync block on non-event entity produces warning"
  }

  features [fa_event_graph_linting]

  verify unit "sync block with barrier parsed"
  verify unit "sync block with timeout parsed"
  verify unit "sync block with delivery semantics parsed"
  verify unit "sync block on non-event produces warning"
}

behavior fa_build_event_bipartite_graph "Build Event-Behavior Bipartite Graph" {
  category command
  types    [SyncBlock, FormalProtocol]
  contract """
    Construct the event-behavior bipartite graph from Produces and
    Consumes edges. Consumers are derived from Consumes edges only
    (the graph is the sole authority for consumption relationships).
    FollowsProtocol edges from events to protocol entities are
    incorporated — protocol ordering constraints are included as
    additional synchronization edges in the bipartite graph.
    This graph is the input to all event flow analysis sub-passes
    (cycle detection, retry cycle, connectivity, channel checks,
    protocol ordering validation).
  """
  requires {
    edges_built            "graph with Produces/Consumes/FollowsProtocol edges is fully built"
    events_exist           "at least one event entity exists in the graph"
  }
  ensures  {
    bipartite_nodes        "bipartite graph contains event nodes and behavior nodes as two disjoint sets"
    producers_included     "every behavior with a Produces edge appears as a producer node"
    consumers_from_edges   "consumers derived from Consumes edges only (not from entity fields)"
    barrier_edges          "sync block barrier references create additional synchronization edges"
    protocol_edges         "FollowsProtocol edges incorporated — protocol ordering constraints included"
    no_orphan_nodes        "bipartite graph contains no orphan nodes disconnected from all edges"
  }

  features [fa_event_graph_linting]

  verify unit "bipartite graph built from produces/consumes edges"
  verify unit "consumers derived from Consumes edges, not entity fields"
  verify unit "barrier references create synchronization edges"
  verify unit "FollowsProtocol edges incorporated into bipartite graph"
  verify property "bipartite graph has no orphan nodes"
  verify property "all producers and consumers are included"
}

behavior fa_detect_unmitigated_cycles "E034: Detect Unmitigated Cycles" {
  category query
  types    [SyncBlock]
  contract """
    Detect circular event dependencies using Tarjan's SCC algorithm
    on the event-behavior bipartite graph. Before firing E034, check
    for mitigations: sync.timeout on any event in the cycle,
    @idempotent annotation, or circuit_breaker pattern. Mitigated
    cycles pass silently. Only bare, unmitigated cycles produce E034.
  """
  requires {
    bipartite_graph_built  "event-behavior bipartite graph is constructed"
  }
  ensures  {
    unmitigated_detected   "unmitigated circular dependency produces E034 error"
    mitigated_passes       "cycle with sync.timeout, @idempotent, or circuit_breaker passes silently"
    non_cycle_passes       "non-circular event dependency produces no diagnostic"
    cycle_path_shown       "E034 includes full cycle path and lists missing mitigations"
  }

  features [fa_event_graph_linting]

  verify unit "unmitigated circular event dependency detected as E034"
  verify unit "cycle with sync.timeout passes silently"
  verify unit "cycle with @idempotent annotation passes silently"
  verify unit "non-circular event dependency passes"
  verify unit "E034 includes full cycle path and missing mitigations"
}

behavior fa_detect_payload_type_mismatch "E035: Payload Type Mismatch" {
  category query
  contract """
    Verify that event producers and consumers agree on payload type.
  """
  requires {
    payload_declared       "event has a payload type reference"
  }
  ensures  {
    matching_passes        "matching producer/consumer payload types produce no diagnostic"
    mismatch_error         "mismatching payload types produce E035 error"
  }

  features [fa_event_graph_linting]

  verify unit "matching producer/consumer payload types pass"
  verify unit "mismatching payload types produce E035"
}

behavior fa_detect_unmatched_producers "W029: Unmatched Producers" {
  category query
  contract """
    Detect events that have producers but no consumers (derived from
    Consumes edges). This is a warning rather than an error because
    fire-and-forget events, future consumers, and external system
    consumers are all valid patterns.
  """
  ensures  {
    matched_passes         "event with producers and consumers produces no diagnostic"
    unmatched_warned       "event with no consumers produces W029 warning"
  }

  features [fa_event_graph_linting]

  verify unit "event with producers and consumers passes"
  verify unit "event with no consumers produces W029"
}

behavior fa_detect_unbounded_channel "W034: Unbounded Channel Buffer" {
  category query
  types    [SyncBlock]
  contract """
    Detect events with no sync timeout or buffer limit, which may
    accumulate unbounded messages under load.
  """
  ensures  {
    unbounded_detected     "event channel with no timeout and no buffer limit produces W034"
    bounded_passes         "event channel with timeout or buffer limit produces no diagnostic"
  }

  features [fa_event_graph_linting]

  verify unit "event channel with no sync timeout produces W034"
  verify unit "event channel with sync timeout passes"
}

behavior fa_detect_asymmetric_connectivity "W033: Asymmetric Connectivity Warning" {
  category query
  contract """
    Detect ports with structural patterns that suggest unbalanced
    access: multiple consumers sharing a port where one consumer has
    significantly more incoming edges. This is a structural complexity
    hint, not a formal fairness guarantee. Detection criteria: port
    has >1 consumer AND consumer edge-count ratio exceeds 3:1.
  """
  ensures  {
    asymmetric_detected    "port with structurally unbalanced access pattern produces W033"
    balanced_passes        "port with balanced access or single consumer passes"
    suggestion             "W033 includes suggestion to review access patterns"
  }

  features [fa_event_graph_linting]

  verify unit "port with unbalanced access pattern produces W033"
  verify unit "port with single consumer passes"
}

behavior fa_detect_unmitigated_retry_cycle "W032: Unmitigated Retry Cycle" {
  category query
  types    [SyncBlock]
  contract """
    Detect event chains where a consumer re-triggers the same event
    without backoff or termination. A cycle in the event-behavior
    graph where no event has a sync block with timeout.
  """
  ensures  {
    retrigger_detected     "cycle without timeout in any sync block produces W032 warning"
    backoff_passes         "cycle with at least one timeout/backoff produces no diagnostic"
    suggestion             "W032 includes suggestion to add timeout to sync block"
  }

  features [fa_event_graph_linting]

  verify unit "re-triggering without backoff detected as W032"
  verify unit "re-triggering with timeout/backoff passes"
}

behavior fa_event_graph_analyze_pass "Event Graph Analyze Compiler Pass" {
  category command
  types    [SyncBlock, FormalProtocol, EventGraphAnalysisReport]
  produces  [fa_event_graph_analysis_complete]
  contract """
    The event_graph_analyze compiler pass performs full event flow
    analysis over the event-behavior graph after layering_verify.
    Includes protocol ordering validation for events with
    FollowsProtocol edges.
  """
  requires {
    layering_verify_done   "layering_verify pass has completed"
    graph_constructed      "entity graph with produces/consumes/follows_protocol edges is built"
    timeout_configured     "analysis timeout set from --timeout flag (default: 30s)"
  }
  ensures  {
    bipartite_delegated    "event-behavior bipartite graph construction delegated to fa_build_event_bipartite_graph"
    cycle_checked          "Tarjan SCC unmitigated cycle detection runs (E034)"
    channel_checked        "payload type compatibility checked (E035)"
    unmatched_checked      "unmatched producers detected (W029)"
    retry_cycle_checked    "unmitigated retry cycles detected (W032)"
    connectivity_checked   "asymmetric connectivity on ports detected (W033)"
    buffer_checked         "unbounded channel buffers detected (W034)"
    protocol_ordering      "protocol ordering validation: protocol ordering must be consistent with event graph topology"
    protocol_sync_consistency "protocol entity ordering must match event sync block constraints when both exist"
    timeout_handled        "barrier timeout expiry sets timed_out=true on EventGraphAnalysisReport and emits warning with incomplete sub-analysis count"
    partial_results        "completed sub-analyses included in report; only incomplete ones omitted on timeout"
    timeout_configurable   "barrier timeout uses configured value, not hardcoded 30s"
    process_integrated     "process entities integrated into bipartite graph when present"
    process_deadlock       "process-level deadlock detection runs on parallel-composed processes"
  }

  features [fa_event_graph_linting]

  verify unit "bipartite graph construction delegated to fa_build_event_bipartite_graph"
  verify unit "unmitigated cycle detection runs on SCC"
  verify unit "payload type check runs on all events"
  verify unit "pass runs after layering_verify"
  verify unit "protocol ordering validation runs on events with FollowsProtocol edges"
  verify unit "protocol-sync block consistency checked"
  verify unit "barrier timeout sets timed_out=true and emits warning"
  verify unit "partial results included on timeout"
  verify unit "configured timeout overrides default 30s"
}

behavior fa_parse_process_entity "Parse Process Entity" {
  category command
  types    [FormalProcess, ProcessState, CompositionOperator]
  contract """
    Parse process entity declarations. Creates ParticipatesIn edges
    (event -> process) from the alphabet field and ProcessComposition
    edges (process -> process) from composition references. Validates
    that alphabet references are event entities.
  """
  ensures  {
    participates_in_edges  "ParticipatesIn edges created from alphabet events to process"
    composition_edges      "ProcessComposition edges created between composed processes"
    alphabet_validated     "alphabet entries must reference existing event entities"
    states_parsed          "process states parsed with initial/accepting flags"
  }

  features [fa_process_modeling]

  verify unit "process entity parsed with ParticipatesIn edges from alphabet"
  verify unit "ProcessComposition edges created for composed processes"
  verify unit "alphabet referencing non-event produces error"
  verify unit "process states parsed with initial/accepting flags"
}

behavior fa_integrate_process_with_event_graph "Integrate Process Entities into Event Graph" {
  invariants [fa_process_composition_dag]
  category command
  types    [FormalProcess, SyncBlock, FormalProtocol]
  contract """
    Incorporate process-level information into the event-behavior
    bipartite graph. Process alphabet membership implies event
    participation. Composition operators inform deadlock analysis.
    Events with both a sync block AND a process field are valid
    (dual-mode).
  """
  requires {
    bipartite_built        "event-behavior bipartite graph is constructed (fa_build_event_bipartite_graph)"
    processes_parsed       "process entities are parsed (fa_parse_process_entity)"
  }
  ensures  {
    process_integrated     "process entities integrated into bipartite graph"
    alphabet_implies_participation "process alphabet membership implies event participation edges"
    composition_informs_analysis   "composition operators inform deadlock analysis heuristics"
    dual_mode_valid        "events with both sync block and process field are valid"
  }

  features [fa_process_modeling]

  verify unit "process entities integrated into bipartite graph"
  verify unit "alphabet membership creates participation edges"
  verify unit "composition operators inform deadlock analysis"
  verify unit "events with both sync block and process field are valid"
}

behavior fa_detect_process_deadlock "Process-Level Deadlock Detection" {
  category query
  types    [FormalProcess, CompositionOperator]
  contract """
    Detect deadlocks between parallel-composed processes via alphabet
    overlap analysis. Two processes composed in parallel whose alphabets
    share events but lack synchronization may deadlock. Sequential
    composition is inherently safe. Extends E034 diagnostics with
    process-level context.
  """
  requires {
    process_integrated     "process entities integrated into event graph"
  }
  ensures  {
    parallel_overlap_checked "parallel-composed processes with overlapping alphabets checked for deadlock"
    sequential_safe        "sequential composition is inherently safe — no deadlock check needed"
    extends_e034           "process-level deadlock extends E034 with process context"
    isolated_safe          "processes with disjoint alphabets pass silently"
  }

  features [fa_process_modeling]

  verify unit "parallel processes with overlapping alphabets flagged for deadlock"
  verify unit "sequential composition passes without deadlock check"
  verify unit "process deadlock extends E034 with process context"
  verify unit "processes with disjoint alphabets pass silently"
}
