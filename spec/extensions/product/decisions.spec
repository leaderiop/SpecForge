decision D-056 "Nine Entity Kinds" {
  settled true
  origin  project
  text    """
    Register 9 entity kinds: feature, journey, deliverable, milestone, module,
    term, persona, channel, release. All have testable=false — they are planning
    constructs tested indirectly through behavior chains from domain extensions.
    Connected by 20 edge types forming a complete traceability graph from user
    need to shipped artifact. Release was added in v1.1 to coordinate
    multi-deliverable shipping with ReleaseIncludesDeliverable,
    ReleaseCompletesMilestone and ReleaseDependsOn edges.
  """
}

decision D-063 "Persona and Channel as First-Class Entities" {
  settled true
  origin  project
  text    """
    Declare persona and channel as first-class entity kinds in
    @specforge/product. Journeys reference them via JourneyTargetsPersona and
    JourneyUsesChannel edges. Both are testable=false. Unreferenced detection
    via I046 (persona) and I047 (channel) at info level to support
    incremental adoption.
  """
}

decision D-046 "Feature as Domain-Neutral Product Concept" {
  settled true
  origin  project
  text    """
    Feature entities use domain-neutral problem/solution framing only.
    No domain-specific fields in the base feature type. Domain extensions
    reference features via their own fields and peer dependencies, creating
    cross-extension edges. Product features work standalone.
  """
}

decision D-030 "All Product Entities Non-Testable" {
  settled true
  origin  project
  text    """
    All 9 product entity kinds declare testable=false in the manifest.
    Product entities are tested indirectly through the behavior chains
    from domain extensions that reference features. Direct testability
    would create false obligations on
    planning constructs that have no executable contract.
  """
}

decision D-083 "TermReferencesRelatedTerm Edge Restricted to Term-to-Term" {
  settled true
  origin  project
  text    """
    Only term-to-term references in see_also produce TermReferencesRelatedTerm edges.
    Cross-kind references are rejected (E022): see_also accepts only terms.
    Terms are navigation aids for
    vocabulary consistency, not dependency-tracking nodes. Cross-kind
    edges from terms would pollute dependency analysis, cycle detection,
    and graph traversal with non-structural relationships.
  """
}

decision D-028 "Acceptance Criteria as Free-Form Prose" {
  settled true
  origin  project
  text    """
    The acceptance field on features is free-form string[] — each entry
    is a prose acceptance criterion with no enforced structure. Machine
    verification of feature completeness belongs to behavior verify
    statements, not feature acceptance criteria. This preserves domain
    neutrality: any domain can express acceptance in its own vocabulary.
  """
}

decision D-048 "Interaction Model as Metadata" {
  settled true
  origin  project
  text    """
    The interaction_model field is metadata for tooling and AI agents, not
    a flow-step validation constraint. interaction_model is an open
    identifier with suggested words (ADR 0279), and the compiler does
    NOT enforce that journey flow steps are compatible with the
    referenced channel's interaction model. Flow-step compatibility is
    domain-specific and belongs to domain extensions or agent reasoning.
  """
}

decision D-035 "Milestone Completion via Feature Status" {
  settled true
  origin  project
  text    """
    Milestone completion uses the FeatureStatus field: done_count / total_features.
    Features with status=done are complete. No cross-extension edges are needed.
  """
}

decision D-057 "No Migration Hook in v1" {
  settled true
  origin  project
  text    """
    Omit migration_hook from the v1 manifest. Add migration hooks when
    the entity model evolves in a breaking way.
  """
}

decision D-080 "Tags Field on All Entity Kinds" {
  settled true
  origin  project
  text    """
    All 9 product entity kinds get the tags field (string[] @optional) as a
    shared field.
  """
}

decision D-049 "Journey Description Field" {
  settled true
  origin  project
  text    """
    Add an optional description field (string) to the ProductJourney type.
    This provides a summary of the journey's purpose without requiring
    agents to parse individual flow steps.
  """
}

decision D-041 "Intentional Diagnostic Code Gaps" {
  settled true
  origin  project
  text    """
    Diagnostic code gaps are intentional and reserved for future validation rules.
    No existing code will be reassigned. New validation rules receive the next
    available code in the appropriate range (E, W, or I).
  """
}

decision D-086 "Validation Behavior Naming Convention" {
  settled true
  origin  project
  text    """
    Adopt the detect_/validate_ prefix convention as a stable naming pattern:
    - detect_* behaviors check structural graph properties (edge presence, cycles,
      field emptiness, cross-entity consistency)
    - validate_* behaviors check individual field values against known enums,
      formats, or constraints
    Both prefixes use category=validation and produce diagnostics.
  """
}

decision D-047 "Feature Ordering Takes No Entity ID Argument" {
  settled true
  origin  project
  text    """
    queryFeatureOrdering() is a global query that returns all features in
    topological order. Scoping would require a subgraph extraction step that
    adds complexity without clear user value. Agents can filter the result
    by milestone or module membership after receiving the full ordering.
  """
}

decision D-076 "Product Extension Fully Standalone" {
  settled true
  origin  project
  text    """
    All product queries operate purely on product entity kinds, edge types,
    and the FeatureStatus field. No query requires peer extensions. Domain
    extensions that want to link to product features do so via their own
    manifests and peer_dependency declarations.
  """
}

decision D-084 "Term Weak Connectivity" {
  settled true
  origin  project
  text    """
    Terms are vocabulary aids, not dependency nodes. TermReferencesRelatedTerm edges are
    optional navigation links — a term is valid without any see_also
    references. I010 detects terms with no edges at all at info level for awareness,
    but term isolation is not a structural defect. Terms serve a fundamentally
    different role than deliverables, modules, or milestones: they define
    vocabulary, not dependencies.
  """
}

decision D-051 "Journey Flow Steps are Opaque" {
  settled true
  origin  project
  text    """
    Journey flow steps are intentionally opaque prose strings. Structured
    flow semantics (step typing, conditional branching, loop detection)
    belong to domain extensions that understand the interaction patterns
    of their specific domain. The base product extension only requires
    flow to be present (core E006: flow is a required field) and leaves
    content interpretation to
    consumers. This preserves domain neutrality.
  """
}

decision D-037 "Deliverable Lifecycle via Status Field" {
  settled true
  origin  project
  text    """
    Add DeliverableStatus (draft, in_progress, shipped, deprecated) and
    a reason field to ProductDeliverable. The core refuses another value (E061). I066 detects
    deprecated deliverables without a documented reason. Status defaults
    to absent (treated as draft) for incremental adoption.
  """
}

decision D-082 "Term Isolation as Design Boundary" {
  settled true
  origin  project
  text    """
    Term isolation is an intentional design boundary in v1. Terms are
    vocabulary aids consumed by humans and agents via text — they are NOT
    dependency nodes. Extensions that need term-to-entity graph connectivity
    (e.g., a glossary-powered documentation renderer) can add a `terms`
    reference field to any entity kind via entity_enhancements in their
    own manifest, creating extension-owned edges without polluting the
    base product model. The product extension intentionally does not
    pre-wire term references on all entity kinds because most domains
    do not need graph-level term connectivity.
  """
}

decision D-074 "Scalability Tiers" {
  settled true
  origin  project
  text    """
    Define three scalability tiers with corresponding performance targets:
    - Tier 1 (≤500 entities): validation <50ms, queries <100ms (MUST)
    - Tier 2 (≤5000 entities): validation <500ms, queries <1s (SHOULD)
    - Tier 3 (>5000 entities): validation <2s, queries <2s or timeout (SHOULD)
    Cycle detection and unreferenced detection MUST remain O(V+E) at all tiers.
    Feature ordering (global topological sort) is the most expensive query
    and gets a dedicated 2-second budget at Tier 2.
  """
}

decision D-058 "No Direct PersonaChannel Edge" {
  settled true
  origin  project
  text    """
    No PersonaChannel edge in v1. The journey is the authoritative source
    of persona-channel binding. A direct edge would duplicate information
    and create sync issues when journeys change. Extensions can add this
    via entity_enhancements if needed. A composite query
    (pe_query_persona_channels) provides the convenience of a direct edge
    without the denormalization risk.
  """
}

decision D-085 "Typed Surface Schemas" {
  settled true
  origin  project
  text    """
    Define per-surface input and output types in the spec DSL (Option C).
    List commands use ProductListFilter input and per-kind list result types
    (FeatureListResult, JourneyListResult, etc.) with pagination. Query commands
    use per-command input types and return existing query payload types.
    All commands share consistent error handling via ProductSurfaceError with
    two error codes: ENTITY_NOT_FOUND (exit 1) and INVALID_INPUT (exit 2).
    GRAPH_NOT_READY and the MCP resource envelope (ProductSurfaceResponse)
    were dropped by ADR 0011: a command only runs over a built graph, and
    product declares no MCP resources.
  """
}

decision D-052 "List Command Pagination" {
  settled true
  origin  project
  text    """
    Use offset/limit pagination (Option C). Default limit is 100, max is 1000.
    total reflects the filtered (pre-pagination) count. has_more indicates
    whether more results exist beyond the current page. Pagination is stateless
    — clients can request any page without server-side cursors. The matrix
    queries page the same way (ADR 0011).
  """
}

decision D-071 "Reason Field Content is Opaque" {
  settled true
  origin  project
  text    """
    Reason content is intentionally opaque prose. Quality validation
    belongs to domain extensions or human review. The product extension
    validates presence, not content, consistent with pe_acceptance_free_form
    and pe_journey_flow_opaque.
  """
}

decision D-029 "Cross-Extension Query Isolation" {
  settled true
  origin  project
  text    """
    Product queries MUST NOT traverse edges owned by other extensions. Product
    queries operate exclusively on the 20 product-owned edge types. The BehaviorImplementsFeature
    edge is owned by @specforge/software and is invisible to product queries.
    This ensures:
    1. Product extension is fully standalone — works without any peer extension
    2. Query results are deterministic regardless of which other extensions are installed
    3. No coupling between product query logic and other extensions' edge schemas
    4. Extensions that need cross-extension traversal implement it in their own queries

    Agents that want to combine product and software graph data should issue
    separate queries to each extension's surface and merge results client-side.
  """
}

decision D-036 "Cross-Extension Enhancement Receiving" {
  settled true
  origin  project
  text    """
    Product entity kinds MAY receive entity_enhancements from peer extensions.
    Enhanced fields are visible in the entity graph and queryable via the
    graph protocol, but product-owned queries and validation rules do NOT
    inspect enhancement fields. Enhancement fields are the responsibility of
    the enhancing extension. Product validation rules only check fields
    declared in the product manifest's field definitions.
  """
}

decision D-079 "Tag Naming Conventions" {
  settled true
  origin  project
  text    """
    Tags are case-sensitive, lowercase, hyphen-separated strings matching
    the pattern [a-z0-9][a-z0-9-]*[a-z0-9] (min 2 chars, max 50 chars).
    Tags that violate this pattern produce an I068 info diagnostic with
    a suggested normalized form. Empty strings in the tags array are silently
    ignored. Tags are NOT namespaced — there are no reserved prefixes.
    This convention is enforced at info level to support incremental adoption
    without blocking compilation.
  """
}

decision D-045 "Extension Versioning Strategy" {
  settled true
  origin  project
  text    """
    The extension version follows semver (major.minor.patch) starting at 1.0.0.
    Breaking changes (removing entity kinds, edge types, or fields; changing
    query return types; renaming diagnostic codes) require a major version bump.
    Additive changes (new entity kinds, edge types, fields, queries, diagnostics)
    require a minor version bump. Bug fixes and documentation changes require a
    patch version bump. The extension version is declared in the manifest and
    included in the graph protocol output. Peer dependencies reference extension
    versions using semver ranges (e.g., ^1.0).
  """
}

decision D-067 "Priority Algorithm Null Handling" {
  settled true
  origin  project
  text    """
    Null-priority entities are excluded from the derivation. Only milestones
    and journeys with an explicit priority field contribute to the derived
    deliverable priority. If ALL constituent entities have null priority,
    the deliverable priority is null. source_count reflects only entities
    WITH explicit priority, not total entities.
  """
}

decision D-039 "Description Field on Planning Entities" {
  settled true
  origin  project
  text    """
    Add an optional description field (string) to ProductFeature,
    ProductMilestone, and ProductDeliverable. For features, description
    complements problem/solution — it serves as a human/agent-readable
    summary when the problem/solution framing is too verbose. For milestones
    and deliverables, description fills a gap where no summary field existed.
    All three fields are optional to preserve incremental adoption.
  """
}

decision D-065 "Reason Field on Persona and Channel" {
  settled true
  origin  project
  text    """
    Add an optional reason field (string) to ProductPersona and
    ProductChannel. I069 detects deprecated personas without a reason.
    I070 detects deprecated channels without a reason. Both are info-level
    to support incremental adoption. This completes the lifecycle consistency
    pattern across all entity kinds with deprecation states.
  """
}

decision D-078 "Tag Cross-Kind Awareness" {
  settled true
  origin  project
  text    """
    Add I084 to detect tags used on entities of 3 or more different
    kinds. The threshold of 3 avoids noise for common cross-cutting tags
    used on 2 kinds (e.g., "mvp" on features and milestones). I084 is
    info-level — cross-kind tags are valid and often intentional for
    cross-cutting categorization. The diagnostic suggests kind-specific
    prefixes (e.g., "core-module", "core-feature") as an opt-in
    convention, not a requirement.
  """
}

decision D-068 "Priority Field Asymmetry" {
  settled true
  origin  project
  text    """
    Priority is only meaningful on planning-active entities that compete for
    scheduling attention: features (what to build first), journeys (which
    user flows to prioritize), and milestones (which phases are critical).
    Deliverables inherit priority from their milestones and journeys. Modules
    are structural — they exist or don't, independent of priority. Terms,
    personas, and channels are reference entities that describe vocabulary,
    roles, and mediums — they are not prioritized against each other.
  """
}

decision D-075 "Schema Evolution Strategy" {
  settled true
  origin  project
  text    """
    Adopt additive-only evolution for minor versions (1.x): new fields are
    always @optional, no fields are removed or renamed. Breaking changes
    (field removal, field rename, type change) require a major version bump
    (2.0) and a migration_hook that transforms existing entities. The
    migration_hook is a Wasm export (migrate__product) that receives the
    old entity JSON and returns the new entity JSON. Migration hooks run
    during specforge migrate and are tested with before/after fixture pairs.
    The extension_versioning decision (pe_extension_versioning) governs the
    version numbering; this decision governs the schema-level contract.
  """
}

decision D-069 "Priority Propagation Algorithm" {
  settled true
  origin  project
  text    """
    The priority propagation algorithm uses max-priority semantics:
    1. Collect priorities from all milestones (via DeliverableTrackedByMilestone)
       and journeys (via DeliverableSupportsJourney) referencing the deliverable.
    2. Filter out null priorities — only explicit priorities participate.
    3. If no explicit priorities exist, return null (priority unknown).
    4. Otherwise, return the highest priority: critical > high > medium > low.
    5. source_count is the number of entities with explicit (non-null) priority.
    This is max-priority, not average or majority-vote. The rationale is
    that a deliverable containing even one critical milestone is itself
    critical — the highest-urgency component dominates.
  """
}

decision D-054 "Module Reason Field" {
  settled true
  origin  project
  text    """
    Add reason as string @optional to ProductModule. No new validation
    rule is needed in v1 — the field is documentation-only, consistent
    with pe_reason_content_opaque. Future versions may add I-level
    diagnostics for deprecated modules without a reason.
  """
}

decision D-055 "Module and Term Lack Status Lifecycle" {
  settled true
  origin  project
  text    """
    Module and term intentionally have no status lifecycle field.

    Module rationale: modules map to code components (crates, packages).
    Their lifecycle is observable from the codebase itself — a module either
    exists in the dependency graph or it doesn't. Adding draft/active/deprecated
    would duplicate what the build system already knows. Module has a reason
    field for documenting deprecation intent, but status is the build artifact's
    responsibility, not the spec's.

    Term rationale: terms are vocabulary definitions. They are either defined
    or they are not. A term has no meaningful lifecycle stages between those
    two states. Deprecated terms should be removed and replaced with updated
    definitions — the see_also field handles term evolution by pointing to
    successor terms. Adding status to terms would create validation noise
    without improving planning outcomes.

    Future versions may add status if a concrete use case emerges (e.g.,
    "proposed" terms in a review workflow), but the bar is high: the status
    must enable a query or validation rule that cannot be achieved otherwise.
  """
}

decision D-034 "Channel URL Field" {
  settled true
  origin  project
  text    """
    Add url as string @optional to ProductChannel. This field links to
    the canonical documentation, API endpoint, or configuration URL for
    the channel. It is informational only — no validation of URL format
    in v1 to avoid false positives on internal URIs and custom schemes.
  """
}

decision D-043 "Entity Kind Addition Migration" {
  settled true
  origin  project
  text    """
    New entity kinds are a minor version addition (1.x), NOT a breaking change.
    The protocol:
      1. New kind is added to entity_kinds[] in the manifest (e.g., 9 kinds)
      2. New edge types connecting the kind are added to edge_types[]
      3. New validation rules and diagnostic codes are allocated from reserved gaps
      4. The manifest version bumps to 1.x (minor)
      5. Existing .spec files are unaffected — no migration needed
      6. The invariant pe_manifest_nine_entity_kinds covers the
         pe_manifest_entity_kinds_match_manifest (count matches manifest declaration)

    Removal of an existing entity kind is a major version change (2.0) requiring
    a migration_hook. Renaming a kind is also a major change. Only addition
    is minor-compatible.

    Edge type additions follow the same pattern: new edges are minor, removal
    is major.
  """
}

decision D-038 "Deliverable-Persona Composite Query" {
  settled true
  origin  project
  text    """
    Add pe_query_deliverable_personas as a composite query on ProductQueryPort.
    The query traverses deliverable->journey->persona edges, deduplicates
    personas, and includes via_journey_ids for traceability. This mirrors
    the pe_query_persona_channels pattern: composite queries provide
    convenience without denormalizing edges. No new edge type is needed.
  """
}

decision D-060 "Observability Events Are Fire-and-Forget" {
  settled true
  origin  project
  text    """
    Observability events are intentionally fire-and-forget. They exist for external
    consumers: CI dashboards, monitoring systems, extension analytics, and debugging tools.
    The product extension does NOT consume its own observability events — this would create
    circular dependencies. Consumers subscribe at the channel level (product.*) and filter
    by event type. The events carry structured payloads (typed via ProductXxxPayload types)
    so consumers can deserialize without parsing. No runtime cost when no consumers
    subscribe — events are lazy (emitted only when a listener exists on the channel).
    Future extensions (e.g., @specforge/analytics) can declare consumers for these events.
  """
}

decision D-064 "Persona and Channel Entities Are Intentionally Shallow" {
  settled true
  origin  project
  text    """
    Personas and channels are intentionally shallow leaf entities. Their purpose is
    to provide identity and classification for journey references, not to carry
    complex graph structure. Adding priority/weighting to personas would duplicate
    journey priority (which already captures persona-specific importance). Adding
    throughput to channels would conflate specification (what the system does) with
    infrastructure (how it performs) — violating the domain-neutral principle.
    Depth is added through journeys: a persona's importance is measured by the
    number and priority of journeys referencing it (core's
    `specforge query <persona> --depth 1 --kind journey`). A channel's
    utilization is measured by journey count and coverage. If a project needs richer persona modeling
    (e.g., weighted scoring, persona segments), an extension can use
    entity_enhancements to add fields to the persona kind without modifying
    the product manifest.
  """
}

decision D-044 "Only Deliverables Track Versions" {
  settled true
  origin  project
  text    """
    Only deliverables track versions because they are shippable artifacts with
    external consumers who depend on version numbers. Milestones, modules, and
    journeys are internal planning constructs — their evolution is tracked by
    version control (git diff on .spec files). Adding version fields to
    internal entities would create maintenance burden without consumer value.
    Temporal tracking (when status changed) is deliberately excluded from v1 —
    it requires either a changelog field or event sourcing, both of which add
    significant complexity. If per-entity temporal tracking is needed, an
    extension can add a changelog field via entity_enhancements.
  """
}

decision D-053 "Additive-Only Schema Evolution" {
  settled true
  origin  project
  text    """
    Minor versions (1.x) follow additive-only schema evolution:
    - New optional fields can be added to any entity kind
    - New diagnostic codes are allocated in the core catalog (a
      third-party code is namespaced) and added to the diagnostic catalog
    - New edge types require manifest version field bump but not major version
    - New entity kinds require major version bump
    - Field removal, kind removal, or edge type removal require major version
      bump with a migration hook
    The migration hook (introduced in v2) receives the old manifest version
    and returns a list of graph transformations (add_field, remove_field,
    rename_field, add_edge_type, remove_edge_type). Until v2, migration_hook
    remains null. Spec files authored for v1.0 parse without error under v1.x.
  """
}

decision D-059 "Product Uses Core Emitter, No Custom Renderer" {
  settled true
  origin  project
  text    """
    Product entities render through the core emitter's uniform node rendering.
    All entity kinds registered in KindRegistry are rendered identically: kind,
    id, fields (per FieldRegistry order), edges (per EdgeTypeRegistry order).
    No product-specific rendering is needed because:
    (1) The graph protocol is kind-agnostic — consumers use the kind field to
        interpret entities, not format-specific rendering.
    (2) Product entities have no structured body content (unlike behaviors with
        Design-by-Contract blocks) — all data is in typed fields.
    (3) The three export formats (context, graph, brief) apply uniformly to
        all entity kinds.
    Entities with validation errors include a _diagnostics array in context
    format. This is a core emitter feature, not product-specific.
  """
}

decision D-066 "Advanced Planning Queries" {
  settled true
  origin  project
  text    """
    Add four new query methods to ProductQueryPort:
    1. queryUnscheduledFeatures — features with zero MilestoneDeliversFeature edges
    2. queryFeatureOverlap — features reachable from 2+ deliverables
    3. queryPersonaCoverageMatrix — per-persona feature reachability matrix
    4. queryCriticalPath — longest incomplete milestone chain with slack

    All four are specforge product commands (unscheduled-features,
    feature-overlap, coverage-matrix, critical-path), auto-promoted to MCP
    tools; product declares no MCP resources (ADR 0011).
  """
}

decision D-033 "End-to-End Chain Validation Rules" {
  settled true
  origin  project
  text    """
    Add four new I-level diagnostics:
    - I076: Deliverable end-to-end chain gap (milestone features not in modules)
    - I077: Feature in multiple milestones (informational awareness)
    - I078: Priority escalation gap (critical feature in low milestone)
    - I079: Implicit milestone ordering (shared features, conflicting dates)

    All are info-level to support incremental adoption. I077 is intentionally
    informational — multi-milestone scheduling is valid for phased delivery.
    I078 requires explicit priority on both feature and milestone (absent
    priority suppresses the check). I079 requires target_date on both
    milestones.
  """
}

decision D-061 "Ownership as String Fields" {
  settled true
  origin  project
  text    """
    Owner is a free-form string field (@optional) on feature, milestone,
    deliverable, and release. Contributors is a string list field. No entity
    references — supports any naming convention.
  """
}

decision D-042 "Effort as T-Shirt Sizes" {
  settled true
  origin  project
  text    """
    T-shirt sizes (xs, s, m, l, xl) mapped to Fibonacci weights (1, 2, 3, 5, 8).
    Applied to features only. Features without effort default to m=3 in queries.
  """
}

decision D-072 "Release as Ninth Entity Kind" {
  settled true
  origin  project
  text    """
    Add release as the 9th product entity kind. Releases group deliverables
    via ReleaseIncludesDeliverable edges, complete milestones via
    ReleaseCompletesMilestone edges, and order releases via ReleaseDependsOn.
    Lifecycle: planned->in_progress->released->recalled. Three new edge
    types (total 20). Previous 8-kinds invariant updated to 9.
  """
}

decision D-081 "Start Date on Milestones" {
  settled true
  origin  project
  text    """
    Add start_date (date @optional, ISO 8601) to milestones. E061 reports
    a start_date that is no calendar day; no rule checks its order.
  """
}

decision D-032 "External Blockers on Milestones" {
  settled true
  origin  project
  text    """
    Add blockers (string[] @optional) to milestones. Free-text descriptions
    of external impediments. I060 reports a blocked milestone with no
    blockers (depends_on doesn't count).
  """
}

decision D-050 "Journey Flow Feature Linking" {
  settled true
  origin  project
  text    """
    Validate bracketed references in flow steps against the journey's declared
    features list. I090 for unresolvable references. Lightweight structure
    without requiring a flow DSL.
  """
}

decision D-073 "Release Entity v1.1 Migration Path" {
  settled true
  origin  project
  text    """
    Release entity adoption is fully additive and requires zero migration:
    1. The release kind is registered during pe_register_entity_kinds alongside
       the other 8 kinds — no conditional registration path.
    2. Every release field except version is optional; a project with zero
       release blocks parses and validates identically to v1.0.
    3. ReleaseIncludesDeliverable, ReleaseCompletesMilestone and
       ReleaseDependsOn edge types are registered but
       produce zero edges when no release entities exist — no phantom edges.
    4. Validation rules targeting releases (W092, W093) fire only
       when release entities are present in the graph. Zero releases = zero
       release diagnostics.
    5. CLI commands (specforge product releases, release-completion, etc.)
       return empty lists when no releases exist — consistent with other
       entity listing behavior.
    6. pe_query_release_completion returns null when no releases exist.
    No migration_hook is needed. No spec file changes are required. The
    pe_migration_additive_only decision governs: v1.1 is a minor version
    bump with additive-only changes.
  """
}

decision D-062 "Ownership Field Exclusion Rationale" {
  settled true
  origin  project
  text    """
    Ownership fields are intentionally limited to the 4 entity kinds that
    represent actionable work items with individual accountability:

    - feature: someone owns the capability being built
    - milestone: someone owns the phase deadline and exit criteria
    - deliverable: someone owns the shippable artifact
    - release: someone owns the coordinated shipping event

    The 5 excluded kinds are excluded for specific reasons:

    - module: structural grouping, not a work item. Module ownership is
      implicit — the owner of the features within the module owns the module.
      Adding owner to modules would create redundant data that drifts from
      feature-level ownership.
    - term: vocabulary definitions are shared knowledge, not owned work.
      Terms evolve through consensus, not individual accountability.
    - journey: cross-functional flows spanning personas and channels.
      Journey ownership is ambiguous — is it the UX designer, the PM,
      or the engineer implementing the flow? The persona and feature
      owners are the accountable parties.
    - persona: user role archetypes are research artifacts, not work items.
      Persona definitions are owned by the product team collectively.
    - channel: interaction mediums are infrastructure concerns, not work
      items. Channel ownership is an ops/platform concern outside the
      product planning scope.

    Extensions can add ownership fields to any kind via entity_enhancements
    if a specific domain requires it. The product extension does not
    pre-wire ownership on kinds where it creates noise.
  """
}

decision D-070 "Product Queries as Derived Convenience Layer" {
  settled true
  origin  project
  text    """
    Product queries are explicitly a derived convenience layer, NOT part of
    the Graph Protocol standard. Each query behavior contract documents its
    exact traversal algorithm — the edge types followed, the traversal
    direction, the aggregation logic — enabling any Graph Protocol consumer
    to independently derive the same results from the exported graph.

    The Graph Protocol output contains all raw entity fields and edges
    required for independent derivation. No query result depends on data
    absent from the graph export. Alternative compilers that produce a
    compliant Graph Protocol JSON are NOT required to implement these
    queries — they are extension-provided conveniences, not standard
    obligations.

    Each query method's contract serves as a reference algorithm
    specification. A different compiler, an agent, or a downstream tool
    can implement any query by following the documented traversal pattern
    against the standard graph export.
  """
}

decision D-077 "Status Transitions via the Git Baseline" {
  settled true
  origin  project
  text    """
    Status transition validation reads the git baseline (ADR 0287): the
    project compiled at HEAD, handed to a pass as `previous`. It records
    entity statuses from the previous commit. When there is a baseline,
    W087-W091/W094 compare current status against the baseline status.
    When there is none, transition rules are suppressed and only
    vocabulary validity (E061) is checked.

    The baseline is a deterministic function of the commit and the
    loaded extensions. It is NOT hidden compiler state — it is what git
    already holds. This preserves deterministic compilation: identical
    inputs (spec files + HEAD) always produce identical outputs.

    CI pipelines that want transition checking fetch enough history to
    name HEAD or the merge-base. Pipelines that want stateless builds
    run outside a repository. Both modes are first-class supported.
  """
}

decision D-031 "Analytics Namespace Separate from Graph Protocol" {
  settled true
  origin  project
  text    """
    Computed analytics are NEVER included in the Graph Protocol standard
    output. The graph export contains raw entities and edges only. Analytics
    are available exclusively through extension query endpoints (CLI commands,
    auto-promoted MCP tools, ProductQueryPort). This keeps the Graph Protocol
    clean and portable while making analytics available to consumers who
    want them.
  """
}

decision D-040 "Deterministic Time-Dependent Queries" {
  settled true
  origin  project
  text    """
    All time-dependent commands accept an explicit --as-of date that
    defaults to the date the host passes in CommandInput (today, UTC,
    YYYY-MM-DD; ADR 0011). The same spec files + the same --as-of always
    produce the same results.

    Overdue detection (I058) is moved from compile-time validation to
    query-time only. It fires when the milestone timeline query is
    executed (either via CLI command or MCP tool), NOT during
    specforge check. This means specforge check is fully deterministic —
    it never depends on wall-clock time. Overdue markers appear only in
    query results, not in the validation diagnostic set.

    The milestone velocity query likewise accepts as_of_date, making
    velocity calculations reproducible for testing and CI.
  """
}

decision D-087 "Product Vocabularies Are Enum Fields" {
  settled true
  origin  project
  text    """
    Each vocabulary is one constant (ADR 0071). Its field is declared an
    enum of it, with the value an absent status counts as as its default;
    a value outside it is the core's E061. The list filters, their help,
    the sorts, the lifecycle pass and the rollups read the same constant.
    W077-W080, W083-W085 and W095 are retired to E061.
  """
}
