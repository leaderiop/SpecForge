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

decision D-023 "Gherkin Bridge for Traceability" {
  settled true
  origin  project
  text    """
    The @specforge/software extension declares a gherkin field with type
    string_list and file_reference=true on behavior entities. This field
    references external .feature files. Feature files use
    `@specforge:entity_id` tags to bind to spec entities. `verify` remains
    as a core grammar construct for unit/property/contract test intent
    declarations. `specforge collect cucumber` command consumes Cucumber
    JSON reports for traceability proof. Two traceability paths: gherkin
    field for behavior entities, convention-based matching for
    invariant/event/constraint entities.
  """
}

decision D-016 "Federation as Extension" {
  settled true
  origin  project
  text    """
    Federation functionality extracted from core into @specforge/federation
    extension. Federation behaviors reference federation-local invariants
    (federated_graph_traversal_integrity, federated_schema_completeness,
    cross_project_reference_safety) instead of core invariants. Generic
    qualified-reference hooks remain in core for any extension to use.
    Core compiler has zero federation knowledge.
  """
}
