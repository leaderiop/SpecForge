// @specforge/product features — capability groupings

// Note: behavior imports removed to break circular dependency.
// Behaviors are resolved globally — use imports not needed for reference resolution.

feature pe_core_entity_kinds "Product Entity Kind Registration" {
  problem   """
    The @specforge/product extension must register 9 entity kinds with
    full metadata, 16 edge types, field definitions, and validation rules.
    Without this registration, the compiler has zero knowledge of product
    planning concepts including the domain-neutral feature entity, who
    uses the system (persona), through which medium (channel), or how
    deliverables ship together (release).
  """
  solution  """
    A comprehensive manifest declaration provides all entity kinds with
    testability flags, LSP metadata (semantic tokens, icons), DOT shapes,
    typed field definitions with edge mappings, and declarative validation
    rules. All 16 edge types produce traversable graph edges — no
    reference field is left unwired. Registration follows the zero-entity
    core protocol defined in ManifestV2. Feature is a domain-neutral hub:
    any extension's entities can reference features via their own fields
    and peer dependencies. Persona and channel are first-class entity
    kinds referenced by journeys via JourneyPersona and JourneyChannel
    edges. Release coordinates multi-deliverable shipping via
    ReleaseDeliverable and ReleaseMilestone edges.
  """
}

feature pe_query_dependency_analysis "Product Dependency Analysis Queries" {
  problem   """
    Product entities form dependency chains (FeatureDependsOn, ModuleDependsOn,
    MilestoneDependsOn) but there is no way to compute topological ordering,
    detect cycles, find reverse dependents, or measure dependency depth. Without
    these queries, agents cannot answer "what order should features be built?"
    or "what is the critical path through milestones?"
  """
  solution  """
    Dependency-focused query behaviors: feature ordering (topological sort
    via FeatureDependsOn), feature dependents (reverse FeatureDependsOn),
    critical path (longest incomplete milestone chain with slack analysis),
    module dependency depth (longest ModuleDependsOn chain), and module
    coupling (shared feature count between modules). Entity-not-found errors
    include fuzzy-match suggestions.
  """
}

feature pe_query_traceability "Product Traceability Queries" {
  problem   """
    Product entities are richly connected across kinds — features belong to
    milestones and deliverables, journeys reference personas and channels,
    modules contain features — but there is no way to trace from any entity
    to its related entities. Without traceability queries, agents cannot answer
    "which deliverables contain this feature?" or "what features does this
    persona need?"
  """
  solution  """
    Traceability query behaviors that traverse the product graph to find
    related entities: deliverable traceability (transitive features via
    journeys and modules), feature deliverables (reverse traversal from
    feature to containing deliverables), feature milestones (reverse
    MilestoneFeature), persona journeys (reverse JourneyPersona), channel
    journeys (reverse JourneyChannel), module deliverables (reverse
    DeliverableModule), journey deliverables (reverse DeliverableJourney),
    deliverable personas (multi-hop deliverable->journey->persona traversal),
    milestone deliverables (reverse DeliverableMilestone), module features
    (forward ModuleFeature), persona channels (multi-hop persona->journey->
    channel traversal), persona features (multi-hop persona->journey->feature
    traversal), channel features (multi-hop channel->journey->feature
    traversal), term graph (N-hop TermSeeAlso traversal), term clusters
    (connected components via TermSeeAlso), and term density (terms per
    entity kind ratio). Entity-not-found errors include fuzzy-match
    suggestions.
  """
}

feature pe_query_coverage_analysis "Product Coverage Analysis Queries" {
  problem   """
    Product entities should form a well-connected graph, but there is no way
    to measure how well entities are connected: which features lack milestone
    scheduling, which features appear in multiple deliverables, or how
    thoroughly personas and channels are covered by journeys. Without coverage
    queries, quality gaps in the product specification go undetected.
  """
  solution  """
    Coverage-focused query behaviors: journey coverage (features with
    status=done per journey), unscheduled features (features with zero
    MilestoneFeature edges), feature overlap (features reachable from 2+
    deliverables), persona coverage matrix (per-persona reachability with
    coverage ratios), and channel coverage matrix (per-channel reachability
    with coverage ratios). Entity-not-found errors include fuzzy-match
    suggestions.
  """
}

feature pe_query_lifecycle_metrics "Product Lifecycle Metrics Queries" {
  problem   """
    Product managers need to track progress and health across milestones,
    deliverables, and the overall product graph: completion ratios, timeline
    status, velocity trends, and priority derivation. Without lifecycle
    queries, agents cannot answer "how complete is this milestone?" or
    "which milestones are overdue?"
  """
  solution  """
    Lifecycle-focused query behaviors: milestone completion (ratio of features
    with status=done), deliverable completion (aggregate milestone completion
    across DeliverableMilestone edges), milestone timeline (chronological
    sort with query-time overdue detection via as_of_date parameter —
    not emitted during specforge check), milestone velocity (feature counts
    by status with days elapsed/remaining and features per day, using
    explicit as_of_date for determinism), deliverable
    priority (derived from constituent milestones and journeys via
    max-priority algorithm), and deliverable dependents (reverse
    DeliverableDependsOn). Entity-not-found errors include fuzzy-match
    suggestions.
  """
}

feature pe_surface_contributions "Product Surface Contributions" {
  problem   """
    The @specforge/product extension has query behaviors and validation rules
    but no declared CLI commands or MCP tools to expose them to users and agents.
    Without typed input/output schemas, consumers cannot discover or validate
    the surface contract at compile time.
  """
  solution  """
    Surface contributions in the manifest declare 21 CLI commands for listing,
    querying, and managing product entities. Each CLI command is auto-promoted
    to an MCP tool for agent consumption. Commands cover entity listing (9
    kinds including release), planning queries (milestone-completion,
    journey-coverage, feature-ordering, milestone-timeline,
    milestone-deliverables, module-features), planning insights
    (unscheduled-features, coverage-matrix, critical-path,
    feature-overlap), and v1.1 commands (release-deliverables,
    release-completion, owner-workload, weighted-milestone-completion).
    Remaining query-port methods (deliverable-traceability,
    feature-deliverables, feature-milestones, persona-journeys,
    channel-journeys, module-deliverables, milestone-deliverables,
    module-features, term-graph, deliverable-completion,
    persona-channels, journey-deliverables, feature-dependents,
    deliverable-dependents, deliverable-priority, persona-features,
    milestone-velocity, deliverable-personas) are accessible via the
    programmatic ProductQueryPort API and as MCP resources.

    Every surface has a typed schema:
    - CLI list commands accept ProductListFilter (--status, --priority, --tags,
      --limit, --offset, --sort-by, --sort-order) and return per-kind list
      result types (FeatureListResult, JourneyListResult, etc.) with pagination.
    - CLI query commands accept typed input (positional entity ID or flags)
      and return the corresponding query payload type.
    - MCP resources return a ProductSurfaceResponse envelope wrapping the
      query payload, with consistent error handling via ProductSurfaceError.
    - All surfaces support --format=json|table|brief for CLI, JSON-only for MCP.
    - Consistent error codes: ENTITY_NOT_FOUND, GRAPH_NOT_READY, INVALID_INPUT.
  """
}

feature pe_validation_suite "Product Validation Suite" {
  problem   """
    Without domain-specific validation rules, the compiler cannot detect
    product-level quality issues: orphan journeys, deliverables without
    journeys, empty milestone phases, or unused modules.
  """
  solution  """
    Declarative validation rules (E007-E009, E015-E016, W041-W046, W049,
    W057, W075-W086, I010, I046-I075) detect common product specification
    quality issues.

    Validation operates in two diagnostic profiles:
    - default: Only E-codes (errors) and W-codes (warnings) are emitted.
      A minimal spec file produces at most structural warnings — never
      informational suggestions about missing optional fields.
    - pedantic: All E-codes, W-codes, AND I-codes are emitted. Enabled
      via --lint=pedantic or warning_level=pedantic in specforge.json.

    Rules by severity: module dependency cycles (E007), invalid persona
    references (E008), invalid channel references (E009), milestone
    dependency cycles (E015), orphan features without journeys (W041),
    orphan journeys without deliverables (W042), deliverables without
    journeys (W043), orphan modules (W044), feature dependency cycles
    (W045), deliverables without modules (W046), empty milestones with
    no features and no modules (W049), completed milestones without
    exit criteria (W057), journey referencing deprecated persona (W075),
    journey referencing deprecated channel (W076), orphan terms (I010),
    orphan personas not referenced by journeys (I046), orphan channels
    not referenced by journeys (I047), features without acceptance
    criteria (I048), deliverable journey-module feature gap (I049),
    journeys with empty flow (I050), milestone feature-module gap (I051),
    singleton tags suggesting typos (I052), invalid milestone target_date
    format (I053), journeys without persona (I054), journeys without
    channels (I055), term see_also referencing non-term (I056), blocked
    milestones without dependencies (I057), overdue milestones (I058,
    query-time only — not emitted during specforge check),
    deferred features without reason (I059), blocked milestones without
    reason (I060), invalid deliverable version format (I061),
    non-standard module family (I062), done features with incomplete
    dependencies (I063), milestone temporal inconsistency (I064),
    deliverable dependency cycles (E016), invalid persona status (W083),
    invalid channel status (W084), invalid deliverable status (W085),
    shipped deliverable with incomplete milestones (I065), deprecated
    deliverable without reason (I066), modules with no features (I067),
    non-conforming tag format (I068), deprecated persona without reason
    (I069), deprecated channel without reason (I070), cross-kind tag
    namespace collision (I071), journey flow step structure (I072),
    transitive deprecated persona reference (I073), transitive deprecated
    channel reference (I074), unanchored exit criteria (I075), and term
    alias conflicts (W086).
    Enum field validation covers FeatureStatus (W077), Priority (W078),
    MilestoneStatus (W079), ArtifactType (W080), TechnicalLevel (W081),
    InteractionModel (W082), PersonaStatus (W083), ChannelStatus (W084),
    and DeliverableStatus (W085).
    Each rule uses the declarative pattern engine.
  """
}

feature pe_graph_rendering "Product Graph Rendering" {
  problem   """
    Product entities are registered in the KindRegistry and appear in the
    entity graph, but there is no specification for how they render in the
    Graph Protocol JSON output produced by specforge export. Without this,
    consumers cannot predict the shape of product entities in exported output.
  """
  solution  """
    Product entities render as standard graph nodes following the core Graph
    Protocol schema. The core emitter handles all entity kinds uniformly —
    no product-specific renderer is needed (contributes.renderers = false).
    Each product entity appears with its kind, id, fields (as declared in
    FieldRegistry), and edges (as declared in EdgeTypeRegistry) in all three
    export formats:
    - context: full entity with all fields and resolved edges
    - graph: entity node with edge list (no field bodies)
    - brief: entity id and kind only
    Field ordering in JSON output follows FieldRegistry declaration order.
    Edge ordering follows source entity's field declaration order. Entities
    with validation errors still appear in output with an _diagnostics array.
  """
}

feature pe_cross_extension_cooperation "Cross-Extension Cooperation" {
  problem   """
    @specforge/product declares no peer_dependencies and operates standalone,
    but @specforge/software declares a peer_dependency on product and
    contributes the Implements edge (behavior->feature) and MilestoneBehavior
    entity_enhancement. There is no specification for how cross-extension
    cooperation is validated end-to-end.
  """
  solution  """
    Cross-extension integration is validated through: (1) product queries
    respect the 16-edge-type allowlist and never follow foreign edges,
    (2) entity_enhancements from peer extensions add fields to product
    entity kinds without modifying the product manifest, (3) the Implements
    edge from @specforge/software creates a traversable link from behaviors
    to features without product extension awareness. Integration testing
    verifies the product+software combination produces correct traceability.
  """
}

feature pe_partial_graph_queries "Partial Graph Query Behavior" {
  problem   """
    When the entity graph contains validation errors (e.g., orphan entities,
    broken references), the behavior of product queries is unspecified.
    Consumers cannot predict whether queries return partial results, fail
    entirely, or silently omit invalid entities.
  """
  solution  """
    Product queries operate on the structural graph, not the validation
    result. Entities with validation errors are still traversable and
    appear in query results. Queries never filter entities based on
    diagnostic state. Consumers can cross-reference query results with
    validation diagnostics to identify which returned entities have
    quality issues.
  """
}

feature pe_migration_strategy "Extension Migration Strategy" {
  problem   """
    The v1 manifest declares migration_hook=null (no prior version), but
    there is no specification for what happens when the product extension
    version bumps. Schema evolution, field additions, and diagnostic code
    changes need a documented migration path.
  """
  solution  """
    The product extension follows additive-only schema evolution for minor
    versions: new fields are always optional, new diagnostic codes use
    reserved ranges, new edge types require manifest version bump. Breaking
    changes (field removal, kind removal, edge type removal) require a major
    version bump with a migration hook that transforms persisted graph state.
    The migration hook receives the old manifest version and returns a list
    of graph transformations. Until v2, migration_hook remains null.
  """
}

feature pe_planning_insights "Advanced Planning Insights" {
  problem   """
    Product managers and agents cannot answer key planning questions
    without manual graph traversal: which features are unscheduled,
    which features overlap across deliverables, what is each persona's
    feature coverage, and what is the critical path through milestones.
    These questions require multi-hop traversal that no existing query
    provides.
  """
  solution  """
    Four new query methods covering the planning blind spots:
    (1) queryUnscheduledFeatures returns features with zero MilestoneFeature
    edges, (2) queryFeatureOverlap returns features reachable from 2+
    deliverables, (3) queryPersonaCoverageMatrix computes per-persona
    reachability with coverage ratios, (4) queryCriticalPath computes
    the longest incomplete milestone chain with slack analysis. All
    queries are exposed as CLI commands and MCP resources.
  """
}

feature pe_chain_validation "End-to-End Chain Validation" {
  problem   """
    Existing validation checks journey-module gaps (I049) and milestone-
    module gaps (I051) independently, but no check validates the full
    deliverable→milestone→feature→module chain. A deliverable can have
    milestones scheduling features that no deliverable module implements,
    with no diagnostic. Additionally, features scheduled in multiple
    milestones, priority mismatches between features and milestones,
    and implicit temporal ordering conflicts go undetected.
  """
  solution  """
    Four new validation rules: I076 detects deliverable end-to-end chain
    gaps, I077 flags features in multiple milestones (informational),
    I078 detects priority escalation gaps (high-priority feature in
    low-priority milestone), I079 detects implicit milestone ordering
    conflicts (shared features with conflicting dates but no depends_on).
  """
}

// ---------------------------------------------------------------------------
// v1.1 features — ownership, effort, release, temporal, blockers, flow
// ---------------------------------------------------------------------------

feature pe_ownership_tracking "Ownership Tracking" {
  problem  """
    Product entities have no concept of who is responsible. Product managers
    cannot answer "what features does Alice own?" or "which milestones have
    no owner?" — basic PM queries that every planning tool supports.
  """
  solution """
    Add owner (string @optional) and contributors (string[] @optional) fields
    to feature, milestone, deliverable, and release entities. Provide an
    owner-workload query that aggregates ownership across all product entities.
    I080 info diagnostic encourages ownership assignment without requiring it.
  """
  tags ["ownership", "planning", "v1-1"]
}

feature pe_effort_estimation "Effort Estimation" {
  problem  """
    Milestone completion is feature-count-based: 3/5 done = 60%. But a
    trivial feature and a month-long epic both count as 1. This makes
    completion ratios misleading for capacity planning.
  """
  solution """
    Add effort field (t-shirt size: xs, s, m, l, xl) to features with
    configurable weights. Default weights follow a Fibonacci-inspired
    scale (1, 2, 3, 5, 8) but teams MAY override via effort_weights in
    specforge.json. Provide a weighted milestone completion query. Features
    without effort default to m weight. I081 info diagnostic (pedantic
    profile only) encourages effort estimation.
  """
  tags ["estimation", "planning", "v1-1"]
}

feature pe_release_coordination "Release Coordination" {
  problem  """
    Deliverables have individual versions and statuses, but multiple
    deliverables often ship together. There is no way to answer "what
    ships together?" or track coordinated release readiness.
  """
  solution """
    Add release as the 9th product entity kind with fields: version,
    status (planned->in_progress->released->recalled), deliverables,
    milestones, release_date, changelog, depends_on, owner, contributors.
    Two new edge types: ReleaseDeliverable and ReleaseMilestone.
  """
  tags ["release", "coordination", "v1-1"]
}

feature pe_temporal_planning "Temporal Planning" {
  problem  """
    Milestones have only target_date — no start date. Duration cannot be
    computed, and milestone dependency ordering cannot validate temporal
    consistency of start dates.
  """
  solution """
    Add start_date (string @optional, ISO 8601) to milestones. Extend
    temporal consistency validation to check start_date vs target_date
    ordering. I087 validates format.
  """
  tags ["temporal", "planning", "v1-1"]
}

feature pe_external_blockers "External Blocker Tracking" {
  problem  """
    Blocked milestones only reference internal dependencies (depends_on).
    External factors (regulatory, third-party APIs, hiring) that block
    progress cannot be documented structurally.
  """
  solution """
    Add blockers (string[] @optional) to milestones. I084 detects blocked
    milestones with neither depends_on nor blockers.
  """
  tags ["blockers", "planning", "v1-1"]
}

feature pe_journey_flow_validation "Journey Flow Validation" {
  problem  """
    Journey flow steps are free-text with no structural validation.
    Steps may reference features via [brackets] but there is no check
    that referenced features exist in the journey's features list.
  """
  solution """
    Validate bracketed references in flow steps against the journey's
    declared features. I090 warns when a flow step references an
    undeclared feature.
  """
  tags ["validation", "journeys", "v1-1"]
}

