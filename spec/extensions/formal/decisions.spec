// @specforge/formal Architecture Decision Records

decision formal_extension_split "Split Formal Analysis from Software Extension" {
  status       accepted
  date         2026-03-10
  context      """
    Two 10-expert panel reviews found that formal methods content bundled
    with @specforge/software overwhelms 90% of users. Formal vocabulary
    (Design by Contract, B-Method, CSP) oversells string-matching
    capabilities. E034 fires on all cycles (false positives on normal
    feedback loops). E033 has no implementable algorithm for semantic
    requirement satisfaction. ManifestV2 lacks passes, feature_flags,
    and field_provenance. New users see 30+ unfamiliar warnings from
    formal analysis they haven't opted into.
  """
  decision     """
    Extract all formal analysis content from @specforge/software into a
    new @specforge/formal extension. The formal extension operates through
    entity_enhancements on @specforge/software entities for inline
    condition blocks (requires/ensures/maintains). This follows the
    "extensions over built-ins" principle (P5) and keeps the software
    extension focused on core entity registration. Users opt into formal
    analysis by installing @specforge/formal and setting warning_level=strict.
  """
  consequences [
    "@specforge/software is simpler and focused on 5 entity kinds + 11 edges",
    "Formal analysis is opt-in via extension installation",
    "Progressive warning levels prevent overwhelming new users",
    "ManifestV2 gains passes and feature_flags for extension-contributed analysis",
    "Entity enhancement mechanism proves its value for cross-extension composition",
    "@specforge/formal requires peer_dependency on @specforge/software",
    "Inline condition fields (requires/ensures/maintains) reference invariants for reusable constraints",
  ]
}

decision formal_condition_entity "Condition as First-Class Entity Kind" {
  status       superseded
  date         2026-03-10
  context      """
    10-expert panel and decision matrix evaluated three options for
    representing reusable conditions:
    - Option A: Inline fields only (requires { name "desc" })
    - Option B: define mechanism (user-defined kinds)
    - Option C: Extension entity kind (condition)

    Option A scored 34/50 — pure-field approach prevents conditions from
    being addressable graph nodes (no edges, no traversal, no querying).
    Conditions duplicated across multiple behaviors cannot be traced.

    Option B scored 37/50 — define creates user-visible kinds but Wasm
    extensions cannot see define-created kinds (dead-on-arrival for
    extension-contributed validation passes).

    Option C scored 44/50 — extension entity kind makes conditions true
    graph citizens: addressable, traversable, queryable, with typed
    edges to behaviors. Supports dual-mode (inline + reference).

    SUPERSEDED (2026-04-18): After further analysis, the condition
    entity kind was removed. Conditions are inline fields
    (requires/ensures/maintains) on behaviors that reference invariants.
    The added complexity of a standalone condition entity kind was not
    justified: invariant entities already serve as reusable, addressable
    graph nodes for shared constraints. Inline ConditionEntry nodes in
    the AST provide sufficient structure for condition analysis without
    requiring a dedicated entity kind. This simplifies the extension
    from 6 to 5 entity kinds and removes 3 edge types
    (RequiresCondition, EnsuresCondition, MaintainsCondition).
  """
  decision     """
    REVERSED: Conditions remain as inline fields (requires/ensures/maintains)
    on behaviors, not as a standalone entity kind. Inline blocks produce
    ConditionEntry nodes in the AST. Shared, reusable constraints are
    modeled as invariant entities, which are already first-class graph
    nodes contributed by @specforge/software.
  """
  consequences [
    "Conditions are inline fields, not standalone entities — simpler model",
    "Invariant entities serve as the reusable, addressable graph nodes for shared constraints",
    "RequiresCondition/EnsuresCondition/MaintainsCondition edges removed",
    "Extension simplified from 6 to 5 entity kinds, 11 to 8 edge types",
    "condition-graph view removed; invariant-graph view serves the same purpose",
  ]
}

decision formal_entity_expansion "Expand Formal Extension to 4 Entity Kinds" {
  status       accepted
  date         2026-03-10
  context      """
    @specforge/formal initially contributed structured conditions as
    inline fields with a hard budget cap of 20 total entity kinds.
    Three formal methods concepts were modeled only as inline fields
    or types but deserve first-class entity status:

    - property: temporal/behavioral assertions (safety/liveness/fairness)
      are distinct from conditions (point-in-time state vs behavior over
      TIME). Without a property entity, temporal assertions cannot be
      queried or traced.
    - axiom: assumed-true foundations that conditions depend on. Without
      explicit axioms, these dependencies are invisible.
    - protocol: shared synchronization contracts across events. Currently
      duplicated as inline sync blocks on individual events.

    Additionally, the 20-entity budget cap was an arbitrary limit that
    prevented quality improvements. The formal extension's W042/W043/W044
    diagnostic codes collided with @specforge/product's codes, requiring
    renumbering to W058-W068.
  """
  decision     """
    Expand @specforge/formal to 3 entity kinds: property, axiom,
    protocol. All 3 are testable=false, supports_verify=false.
    Add 3 new edge types: AssumedBy (invariant → axiom), Satisfies
    (behavior → property), FollowsProtocol (event → protocol).
    PropertyDependsOn (property → invariant) added for property-invariant
    dependency tracking.

    Remove the arbitrary 20-entity budget cap. Quality and readability
    over artificial limits. New entity total: 2 structural + 20 domain
    = 22.

    Renumber formal diagnostic codes from W042-W044 to W058-W060 to
    resolve collision with @specforge/product. New diagnostic range:
    W058-W068 for the formal extension.
  """
  consequences [
    "property/axiom/protocol are first-class graph nodes — addressable, traversable, queryable",
    "Temporal properties (safety/liveness/fairness) can be queried and traced via Satisfies edges",
    "Axiom dependencies are explicit — invariants declare their assumed-true foundations",
    "Protocol entities replace duplicated sync blocks — single definition, multiple event references",
    "Axioms generate no coverage tracking items (assumed-true by definition)",
    "Entity budget cap removed — 21 total entity kinds with no artificial limit",
    "Formal diagnostic codes renumbered to W058-W068 (no collision with product W042-W044)",
    "Entity enhancements add assumes to invariant, satisfies to behavior, follows_protocol to event",
  ]
}

decision formal_refinement_process_entities "Add Refinement and Process Entity Kinds" {
  status       accepted
  date         2026-03-10
  context      """
    @specforge/formal has 3 entity kinds covering 2 of 3 disciplines:
    - Structured Conditions: property, axiom (2 entities, plus inline fields)
    - Event Graph Linting: protocol (1 entity)
    - Specification Layering: NO dedicated entity — only abstract/refines
      field annotations on behaviors

    Additionally, Event Graph Linting lacks a process-level entity. Deadlock
    analysis operates only at event-behavior bipartite level, not process level.
  """
  decision     """
    Add refinement and process as entity kinds 4 and 5. Both testable=false,
    supports_verify=false. 4 new edge types: RefinesTo, RefinementChainLink,
    ParticipatesIn, ProcessComposition. 2 new error codes: E041 (refinement
    chain cycle), E042 (process composition cycle). 6 new warnings: W069-W074.
    Entity enhancements: behavior gets refinement field, event gets process field.
    Total: 5 entity kinds, 8 edge types, 23 project entities.
  """
  consequences [
    "Each formal discipline has representative entity kinds",
    "Specification layering is a first-class graph concept",
    "Process-level deadlock analysis extends event graph linting",
    "Dual-mode: new entities coexist with field-based mechanisms",
    "4 new edge types enable new graph traversal patterns",
    "Coverage tracking extended with process_coverage",
    "Total project entities: 2 structural + 21 domain = 23",
  ]
}

decision formal_terminology_rename "Rename Formal Methods Terminology" {
  status       accepted
  date         2026-03-10
  context      """
    Expert panels identified that academic formal methods terminology
    (Design by Contract, B-Method Refinement, CSP Event Flow Analysis,
    Verification Obligations) oversells the capabilities of what are
    actually heuristic structural checks on string-based conditions.
    Users expect mathematical proof when they see "verification obligation"
    but get name-matching heuristics. The terminology gap between
    promise and delivery erodes trust.
  """
  decision     """
    Rename all formal methods vocabulary to accurately describe the
    structural analysis being performed:
    - "Design by Contract" -> "Structured Conditions"
    - "B-Method Refinement" -> "Specification Layering"
    - "CSP Event Flow Analysis" -> "Event Graph Linting"
    - "Verification Obligations" -> "Coverage Tracking Items"
    - "FormalityLevel" -> "SpecificationDepthLevel"
    - "Deadlock detection" -> "Unmitigated cycle detection"
    - "Livelock risk" -> "Unmitigated retry cycle"
    - "Starvation risk" -> "Asymmetric connectivity warning"
    - E033 downgraded to W058 (structural check, not semantic verification)
    - E034 now checks for mitigations before firing
  """
  consequences [
    "Terminology accurately describes capabilities — no overselling",
    "Users have correct expectations about analysis depth",
    "Academic vocabulary preserved in documentation for cross-referencing",
    "All behavior IDs change from se_ to fa_ prefix",
    "Existing spec files referencing old IDs need migration",
  ]
}
