requirement PERFORMANCE-14 "Product Validation Latency" {
  category "Performance"
  tier     v2
  text     """
    All product validation rules must complete within 50ms for projects with up to 500 entities across all 9 kinds.

    Product-specific validation rules (E007, E015, E052, W041-W046, W049,
    W057, W092, W093, I010, I046-I048, I050,
    I054, I055, I057, I059-I061, I066-I070, I080-I083, I089)
    MUST complete in under
    50ms for a project with up to 500 product entities across all 9 kinds.
  """
}

requirement RELIABILITY-08 "Product Cycle Detection Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Cycle detection for modules, milestones, features, and deliverables must have zero false negatives and zero false positives.

    Cycle detection for modules (E007), milestones (E015), and features
    (W045) MUST have zero false negatives — every cycle in the graph MUST
    be detected. Zero false positives — acyclic graphs MUST never produce
    cycle diagnostics. Verified by property-based testing with randomly
    generated dependency graphs including transitive cycles.
  """
}

requirement RELIABILITY-29 "Product Unreferenced Detection Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Unreferenced detection must fire if and only if an entity has no incoming edge (I010: no edge at all).

    Unreferenced detection for features (W041), journeys (W042), modules (W044),
    personas (I046) and channels (I047) MUST fire if and only if the entity
    has no incoming edge of any type. Unreferenced-term detection (I010)
    MUST fire if and only if the term has no incoming edge and names nothing.
    Zero false positives when entities have valid references. Zero false
    negatives when entities are truly unreferenced.
  """
}

requirement RELIABILITY-20 "Product Query Correctness" {
  category "Reliability"
  tier     v2
  text     """
    All product graph queries must return correct results for all graph topologies, including transitive traversals and reverse queries.

    All product graph queries (milestone completion, deliverable traceability,
    journey coverage, feature ordering, milestone timeline, feature
    deliverables, term graph, deliverable completion, persona channels,
    feature dependents, deliverable dependents, deliverable priority, and
    the analytics queries)
    MUST return correct results for all graph topologies. Milestone completion
    ratio MUST be mathematically correct (done_count / total). Deliverable
    traceability MUST include all transitive features via both journey and
    module paths. Dependents queries MUST return the exact inverse of the
    corresponding depends_on references.
  """
}

requirement RELIABILITY-22 "Product Standalone Operation" {
  category "Reliability"
  tier     v2
  text     """
    All product queries must operate purely on product entity kinds and edge types without requiring peer extensions.

    All product queries operate purely on product entity kinds and edge
    types. No query requires peer extensions. Feature coverage uses the
    FeatureStatus field. No crashes, no false diagnostics, no panics.
  """
}

requirement RELIABILITY-13 "Product Entity Registration Determinism" {
  category "Reliability"
  tier     v2
  text     """
    Registration of entity kinds, edge types, and field definitions must be deterministic regardless of registration order.

    Registration of 9 entity kinds, 20 edge types, and all field
    definitions MUST be deterministic — same manifest input always
    produces same KindRegistry and FieldRegistry state. Registration
    order MUST NOT affect validation outcomes.
  """
}

requirement RELIABILITY-11 "Product Diagnostic Severity Correctness" {
  category "Reliability"
  tier     v2
  text     """
    All product diagnostics must fire at their declared severity level matching their E/W/I code prefix.

    All product diagnostics MUST fire at their declared severity level:
    E-codes (E007, E015, E052) are errors, W-codes (W041-W046, W049, W057,
    W092, W093) are warnings, I-codes (I010,
    I046-I048, I050, I054, I055, I057, I059-I061, I066-I070, I080-I083,
    I089) are info. No
    diagnostic may fire at a different severity than its code prefix declares.
  """
}

requirement RELIABILITY-26 "Product Tag Detection Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Singleton tag detection and tag format validation must fire correctly without false positives, completing under 10ms for 1000 tags.

    Singleton tag detection (I052) MUST fire for tags on exactly one entity
    and suppress for tags on 2+ entities. Levenshtein suggestions MUST NOT
    produce false positives for intentionally short distinct tags. Tag format
    validation (I068) MUST fire for tags not matching [a-z0-9][a-z0-9-]*[a-z0-9].
    Combined tag detection MUST complete in under 10ms for up to 1000 tags
    across 500 entities.
  """
}

requirement PERFORMANCE-05 "Product Large-Scale Validation" {
  category "Performance"
  tier     v2
  text     """
    Validation rules must complete under 500ms for 5000 entities and under 2s beyond that, with O(V+E) cycle and unreferenced detection.

    Product-specific validation rules SHOULD complete in under 500ms for
    a project with up to 5000 product entities across all 9 kinds. For
    projects exceeding 5000 entities, validation SHOULD complete in under
    2 seconds. Cycle detection algorithms MUST remain O(V+E) regardless
    of graph size. Unreferenced detection MUST remain O(V+E) by leveraging
    pre-computed incoming-edge indexes.
  """
}

requirement PERFORMANCE-04 "Product Large-Scale Query Latency" {
  category "Performance"
  tier     v2
  text     """
    All product graph queries must complete under 1s for 5000 entities, with feature ordering under 2s.

    All product graph queries SHOULD complete in under 1 second for a
    project with up to 5000 product entities across all 9 kinds.
    Feature ordering (global topological sort) SHOULD complete in under
    2 seconds for 5000 features. Queries on projects exceeding 5000
    entities SHOULD return partial results or a timeout error rather
    than blocking indefinitely.
  """
}

requirement RELIABILITY-09 "Product Deliverable Lifecycle Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Deliverable status, shipped-incomplete-milestone, and deprecated-without-reason validation must fire precisely per their diagnostic rules.

    Deliverable status validation MUST refuse (E061) exactly the status
    values outside DeliverableStatus. Shipped
    deliverable incomplete milestone detection (I065) MUST fire for each
    non-completed milestone in a shipped deliverable. Deprecated deliverable
    reason detection (I066) MUST fire for deprecated deliverables with
    empty or missing reason fields.
  """
}

requirement RELIABILITY-25 "Product Surface Schema Completeness" {
  category "Reliability"
  tier     v2
  text     """
    Every product CLI command must have a typed input and a typed json payload, with no untyped JSON allowed.

    Every one of the 40 product CLI commands MUST have typed args
    (ProductListFilter for list commands, the entity id positional for
    entity-scoped queries) and a typed json payload: the per-kind list
    result type for list commands, the query payload type types.spec
    names for each query, including bulk_status and health. MCP serves
    the same payload as the auto-promoted tool's result; product declares
    no MCP resources (ADR 0011). No command may return untyped JSON.
  """
}

requirement RELIABILITY-24 "Product Surface Error Correctness" {
  category "Reliability"
  tier     v2
  text     """
    All commands must use exactly two error codes and must never panic on any input including null, empty, or malformed JSON.

    All product commands MUST use exactly two error codes:
    ENTITY_NOT_FOUND (exit 1) and INVALID_INPUT (exit 2). The error is
    written to stderr (the ProductSurfaceError object under --format json)
    with nothing on stdout. Over MCP a non-zero exit is an isError tool
    result carrying the McpError the object maps to (entity_not_found or
    invalid_input, its entity_id and suggestion kept). No command may panic on any input
    including null, empty string, or malformed JSON.
  """
}

requirement RELIABILITY-15 "Product List Pagination Correctness" {
  category "Reliability"
  tier     v2
  text     """
    All 9 list commands and the 5 matrix queries must return correct pagination with proper total counts, has_more flags, and input validation.

    All 9 list commands and the 5 matrix queries (coverage_matrix,
    channel_coverage_matrix, feature_overlap, owner_workload,
    module_coupling) MUST return correct pagination: total is the count
    after filtering and before paging, has_more is total > offset +
    returned count, limit defaults to 100, offset defaults to 0. A negative
    offset MUST produce INVALID_INPUT (exit 2). Limit is clamped to
    [1, 1000]. There are no cursors (ADR 0011).
  """
}

requirement PERFORMANCE-13 "Product Surface Latency" {
  category "Performance"
  tier     v2
  text     """
    CLI commands must complete under 200ms for 500 entities, with surface overhead under 50ms.

    All 40 CLI commands SHOULD complete in under 200ms for a project with
    up to 500 product entities (includes graph query + JSON serialization
    + stdout write); their auto-promoted MCP tools run the same export.
    Surface overhead (serialization + dispatch) MUST
    NOT exceed 50ms beyond the underlying query latency.
  """
}

requirement PERFORMANCE-09 "Product Query Latency" {
  category "Performance"
  tier     v2
  text     """
    All 12 core product graph queries must complete under 100ms for projects with up to 500 entities.

    All 12 core product graph queries (milestone-completion,
    deliverable-traceability, journey-coverage, feature-ordering,
    milestone-timeline, feature-deliverables, term-graph,
    deliverable-completion, persona-channels, feature-dependents,
    deliverable-dependents, deliverable-priority) SHOULD complete in under 100ms
    for a project with up to 500 product entities across all 9 kinds.
  """
}

requirement RELIABILITY-19 "Product Persona/Channel Lifecycle Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Deprecated persona and channel reason detection must fire only when reason is empty or missing, and suppress otherwise.

    Deprecated persona reason detection (I069) MUST fire for deprecated
    personas with empty or missing reason fields. Deprecated channel reason
    detection (I070) MUST fire for deprecated channels with empty or missing
    reason fields. Both diagnostics MUST suppress when the reason field is
    present and non-empty. Both diagnostics MUST suppress for non-deprecated
    entities and entities without a status field.
  """
}

requirement RELIABILITY-14 "Product Impact Query Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Feature impact and persona feature queries must return complete transitive closures with order-independent results.

    Feature impact analysis (pe_query_feature_impact) MUST return the
    complete transitive closure of all affected entities. Persona feature
    traversal (pe_query_persona_features) MUST return the exact union of
    features across all persona journeys. Both queries MUST produce
    identical results regardless of edge traversal order.
  """
}

requirement RELIABILITY-27 "Product Term Alias Validation Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Term alias conflict detection must fire for case-insensitive duplicate aliases with zero false positives or negatives.

    Term alias conflict detection (W086) MUST fire if and only if two
    terms share the same alias (case-insensitive) or a term's alias
    matches another term's entity ID (case-insensitive). Zero false
    positives for distinct aliases. Zero false negatives for conflicting
    aliases.
  """
}

requirement RELIABILITY-10 "Product Deliverable-Persona Query Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Deliverable persona traversal must return deduplicated personas via journey edges with deterministic results.

    Deliverable persona traversal (pe_query_deliverable_personas) MUST return
    the exact deduplicated set of personas reachable via deliverable->journey->
    persona edges. via_journey_ids MUST include all intermediate journeys. The
    result MUST be deterministic and identical regardless of edge traversal order.
  """
}

requirement RELIABILITY-18 "Pagination Sort Stability" {
  category "Reliability"
  tier     v2
  text     """
    Paginated list queries must produce stable, deterministic sort order with entity ID as tie-breaker.

    List queries with pagination MUST produce stable sort order across
    consecutive paginated requests when the graph has not been rebuilt.
    Specifically: concatenating pages 0..N MUST produce the same entity
    sequence as a single unpaginated query. Sort order MUST be deterministic
    — entities with identical sort keys MUST be tie-broken by entity ID
    (alphabetical ascending). Cursor-based pagination is not supported in v1;
    offset-based pagination is the only mechanism.
  """
}

requirement RELIABILITY-07 "Cross-Extension Integration Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Product queries must return identical results with or without peer extensions, enforced by the 20-edge-type allowlist.

    When @specforge/software is installed alongside @specforge/product:
    (1) product queries MUST return identical results to standalone operation
    (no foreign edges followed), (2) entity_enhancements from software MUST
    add fields to product entity kinds without affecting query behavior,
    (3) the BehaviorImplementsFeature edge MUST be traversable by software queries but
    invisible to product status results; delivery evidence alone reads it,
    in fields that are absent without recorded evidence (ADR 0039). The
    20-edge-type allowlist MUST be the sole enforcement mechanism for
    status results.
  """
}

requirement RELIABILITY-23 "Product Status Transition Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Status transition validation must catch all invalid transitions and suppress when there is no git baseline.

    Status transition validation (W087-W091) MUST fire if and only if
    the current status cannot be reached from the prior status via a
    valid transition. Zero false negatives (every invalid transition is
    caught). Zero false positives when a git baseline exists and
    transitions are valid. All transition checks MUST be suppressed
    when there is no git baseline. Terminal
    states MUST have zero outbound transitions.
  """
}

requirement RELIABILITY-16 "Product New Query Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Unscheduled features, feature overlap, coverage matrix, and critical path queries must return correct results for all topologies.

    All 4 new query methods (queryUnscheduledFeatures, queryFeatureOverlap,
    queryPersonaCoverageMatrix, queryCriticalPath) MUST return correct
    results for all graph topologies. Unscheduled features MUST have zero
    MilestoneDeliversFeature edges. Feature overlap MUST be detected via both
    journey and module paths. Coverage matrix ratios MUST be mathematically
    correct. Critical path MUST be the longest incomplete milestone chain.
  """
}

requirement PERFORMANCE-06 "Product New Query Latency" {
  category "Performance"
  tier     v2
  text     """
    New query methods must complete under 100ms for 500 entities, with critical path under 200ms and coverage matrix under 500ms at 5000 entities.

    All 4 new query methods SHOULD complete in under 100ms for a project
    with up to 500 product entities. Critical path computation (topological
    sort + longest path) SHOULD complete in under 200ms for up to 5000
    milestones. Coverage matrix computation SHOULD complete in under 500ms
    for up to 5000 entities (it traverses persona→journey→feature for
    every persona).
  """
}

requirement RELIABILITY-06 "Product Chain Validation Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Chain gap, multi-milestone, priority escalation, and implicit ordering diagnostics must fire precisely per their trigger conditions.

    End-to-end chain validation (I076) MUST fire for every uncovered
    feature in the deliverable→milestone→feature→module chain. Feature
    multi-milestone detection (I077) MUST fire for features in 2+
    milestones. Priority escalation (I078) MUST fire only when both
    feature and milestone have explicit priority and the feature's is
    higher. Implicit ordering (I079) MUST fire only for milestones
    with target_date fields.
  """
}

requirement RELIABILITY-28 "Product Transitive Deprecated Reference Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Transitive deprecated persona and channel detection must fire on deliverables referencing journeys with deprecated entities.

    Transitive deprecated persona detection (I073) and transitive deprecated
    channel detection (I074) SHOULD fire on deliverables that reference
    journeys using deprecated personas or channels. SHOULD suppress when all
    journey personas and channels are active or have no status.
  """
}

requirement PERFORMANCE-11 "Release Validation Latency" {
  category "Performance"
  tier     v2
  text     """
    Release entity validation including cycle detection and version format checks must complete under 50ms for 500 entities.

    Release entity validation (cycle detection, lifecycle consistency, version
    format, date format) MUST complete in under 50ms for 500 entities.
  """
}

requirement PERFORMANCE-07 "Owner Workload Query Latency" {
  category "Performance"
  tier     v2
  text     """
    The owner workload query scanning features, milestones, deliverables, and releases must complete under 100ms for 500 entities.

    The owner workload query MUST complete in under 100ms for 500 entities.
    The query scans all features, milestones, deliverables, and releases.
  """
}

requirement PERFORMANCE-15 "Weighted Completion Query Latency" {
  category "Performance"
  tier     v2
  text     """
    The weighted milestone completion query must complete under 100ms for milestones with up to 200 features.

    The weighted milestone completion query MUST complete in under 100ms
    for milestones with up to 200 features.
  """
}

requirement PERFORMANCE-10 "Release Query Latency" {
  category "Performance"
  tier     v2
  text     """
    The release completion query must complete under 100ms for 500 entities.

    The release completion query MUST complete in under 100ms for 500
    entities. A release's deliverables and milestones are core's
    `specforge query <release> --depth 1` (ADR 0011).
  """
}

requirement RELIABILITY-21 "Release Cycle Detection Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Release dependency cycle detection must have zero false negatives and zero false positives using Tarjan's algorithm.

    Release dependency cycle detection MUST have zero false negatives
    and zero false positives. Uses same Tarjan algorithm as other DAGs.
  """
}

requirement RELIABILITY-17 "Ownership Field Consistency" {
  category "Reliability"
  tier     v2
  text     """
    Missing owner detection must fire only for ownable entity kinds and never for journeys, modules, terms, personas, or channels.

    I080 MUST fire for every feature, milestone, deliverable, and release
    without an owner field. I080 MUST NOT fire for entity kinds that do
    not support owner (journey, module, term, persona, channel).
  """
}

requirement RELIABILITY-12 "Effort Weight Correctness" {
  category "Reliability"
  tier     v2
  text     """
    Weighted milestone completion must use the fixed Fibonacci effort weights with features defaulting to medium when unspecified.

    Weighted milestone completion MUST use the effort scale's weights:
    xs=1, s=2, m=3, l=5, xl=8 (Fibonacci-inspired). The weights define
    the scale and are not configurable (ADR 0011). Features without
    effort MUST default to the weight of m (3).
  """
}

requirement PERFORMANCE-08 "Product Query Complexity Bounds" {
  category "Performance"
  tier     v2
  text     """
    Multi-hop traversal queries must complete under 500ms for 5000 entities, bounded by max_hops to prevent unbounded traversal.

    Multi-hop traversal queries (queryPersonaCoverageMatrix,
    queryFeatureImpact, queryCriticalPath) SHOULD complete in under 500ms
    for projects with up to 5000 entities. Queries SHOULD be bounded by
    max_hops (default 5) to prevent unbounded traversal. The complexity
    of queryPersonaCoverageMatrix is O(P*J*F) where P=personas, J=journeys,
    F=features.
  """
}

requirement PERFORMANCE-12 "Product Scalability Tiers" {
  category "Performance"
  tier     v2
  text     """
    Product operations must degrade gracefully across three entity-count tiers with no superlinear degradation beyond 10000 entities.

    Product extension operations MUST degrade gracefully as entity count
    increases. Performance expectations are defined in three tiers:

    Tier 1 (<=500 entities): validation <50ms, queries <100ms.
    Tier 2 (501-2000 entities): validation <200ms, queries <500ms.
    Tier 3 (2001-10000 entities): validation <1s, queries <2s.

    Beyond 10,000 entities, no hard guarantees apply but the extension MUST
    NOT exhibit superlinear (O(n^2) or worse) degradation for any operation.
    Pagination becomes critical at Tier 3 — queries returning result sets
    >1000 items MUST support pagination.

    Cycle detection (Tarjan's) is O(V+E) at all tiers. Coverage matrix
    (O(P*J*F)) is the highest-complexity query and MAY exceed 2s at Tier 3.
    Critical path computation is O(V+E) via topological sort and does not
    degrade significantly.
  """
}

requirement USABILITY-05 "Query Pagination for Large Result Sets" {
  category "Usability"
  tier     v2
  text     """
    Matrix and analytics queries must page with the list commands' offset and limit to enable incremental consumption by AI agents.

    Matrix and analytics queries (coverage_matrix, module_coupling,
    owner_workload, feature_overlap, channel_coverage_matrix) MUST page
    like the list commands: --limit (default 100, clamped to [1, 1000])
    and --offset (default 0), with total, offset, limit and has_more in
    the payload. There are no cursors: with no graph revision a cursor
    detects nothing an offset does not (ADR 0011).

    This enables AI agents to consume large result sets incrementally
    without exceeding token budgets.
  """
}
