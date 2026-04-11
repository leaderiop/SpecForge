// @specforge/product — v1.1 behaviors
//
// Release entity kind, release edges, ownership fields, effort field,
// temporal fields, and v1.1-specific query behaviors.

use "extensions/product/invariants"
use "extensions/product/types"
use "types/zero-entity-core"

behavior pe_register_release_kind "Register Release Entity Kind" {
  category   command
  invariants [pe_release_non_testable]
  types      [ManifestEntityKind, ProductRelease, ReleaseStatus]
  contract   """
    The @specforge/product extension MUST register the release entity kind
    with testable=false, supportsVerify=false, and appropriate LSP/DOT metadata.
  """
  requires   {
    manifest_loaded  "ManifestV2 is parsed and schema-validated"
  }
  ensures    {
    release_registered "KindRegistry contains release: testable=false, supportsVerify=false, semanticToken=class, lspIcon=Event, dotShape=octagon"
  }

  verify unit "release kind is registered with testable=false"
  verify unit "release kind has dotShape=octagon"
}

behavior pe_register_release_edges "Register Release Edge Types" {
  category   command
  types      [ManifestEdgeType]
  contract   """
    The @specforge/product extension MUST register 2 release-specific edge types:
    ReleaseDeliverable (release->deliverable) and ReleaseMilestone (release->milestone).
  """
  requires   {
    entity_kinds_registered "All 9 entity kinds are in the KindRegistry"
  }
  ensures    {
    release_deliverable_registered "EdgeTypeRegistry contains ReleaseDeliverable: source=release, target=deliverable, label=ships"
    release_milestone_registered   "EdgeTypeRegistry contains ReleaseMilestone: source=release, target=milestone, label=targets"
  }

  verify unit "ReleaseDeliverable edge type is registered"
  verify unit "ReleaseMilestone edge type is registered"
}

behavior pe_register_ownership_fields "Register Ownership Fields" {
  category   command
  features   [pe_ownership_tracking]
  contract   """
    The @specforge/product extension MUST register owner (string @optional)
    and contributors (string[] @optional) as shared fields on feature,
    milestone, deliverable, and release entity kinds.
  """
  requires   {
    entity_kinds_registered "All 9 entity kinds are in the KindRegistry"
  }
  ensures    {
    owner_on_four_kinds       "FieldRegistry contains owner:string @optional for kind=feature,milestone,deliverable,release"
    contributors_on_four_kinds "FieldRegistry contains contributors:string[] @optional for kind=feature,milestone,deliverable,release"
  }

  verify unit "owner field is registered on feature, milestone, deliverable, release"
  verify unit "contributors field is registered on feature, milestone, deliverable, release"
}

behavior pe_register_effort_field "Register Effort Field" {
  category   command
  types      [Effort]
  contract   """
    The @specforge/product extension MUST register effort (Effort @optional)
    on the feature entity kind. Valid values: xs, s, m, l, xl.
  """
  ensures    {
    effort_on_feature "FieldRegistry contains effort:Effort @optional for kind=feature"
  }

  verify unit "effort field is registered on feature kind"
}

behavior pe_register_temporal_fields "Register Temporal Fields" {
  category   command
  features   [pe_temporal_planning]
  contract   """
    The @specforge/product extension MUST register start_date (string @optional)
    on the milestone entity kind and blockers (string[] @optional) on the
    milestone entity kind.
  """
  ensures    {
    start_date_on_milestone "FieldRegistry contains start_date:string @optional for kind=milestone"
    blockers_on_milestone   "FieldRegistry contains blockers:string[] @optional for kind=milestone"
  }

  verify unit "start_date field is registered on milestone kind"
  verify unit "blockers field is registered on milestone kind"
}

behavior pe_query_owner_workload "Query Owner Workload" {
  category   query
  invariants [pe_ownership_field_awareness]
  types      [OwnerWorkloadPayload, OwnerWorkloadEntry, OwnerKindBreakdown]
  produces  [pe_owner_workload_queried]
  contract   """
    The product extension MUST provide a query that aggregates ownership
    across features, milestones, deliverables, and releases. The result
    groups entities by owner string, counts entities per kind, and reports
    the number of unowned entities.
  """
  requires   {
    graph_ready "Graph is built and queryable"
  }
  ensures    {
    grouped_by_owner  "Each unique owner string appears exactly once in the owners array"
    unowned_counted   "unowned_count equals the number of entities with no owner field"
    total_correct     "total_entities equals sum(owners[].entity_count) + unowned_count"
  }

  verify unit "single owner across multiple kinds returns correct breakdown"
  verify unit "entities without owner contribute to unowned_count"
  verify unit "empty graph returns zero totals"
}

behavior pe_query_weighted_milestone_completion "Query Weighted Milestone Completion" {
  category   query
  invariants [pe_effort_weighted_completion]
  types      [WeightedMilestoneCompletionPayload, EffortBreakdownEntry, Effort]
  produces  [pe_weighted_completion_queried]
  contract   """
    The product extension MUST provide a query that computes effort-weighted
    milestone completion. Default effort weights follow a Fibonacci-inspired
    scale: xs=1, s=2, m=3, l=5, xl=8. Teams MAY override weights via the
    effort_weights map in the @specforge/product extension configuration
    within specforge.json. Features without effort default to m (default
    weight 3). When custom weights are configured, the query uses those
    weights instead of the defaults.
  """
  requires   {
    graph_ready      "Graph is built and queryable"
    milestone_exists "Milestone ID resolves to a milestone entity"
  }
  ensures    {
    default_weights     "default weights xs=1, s=2, m=3, l=5, xl=8 are used when no custom weights configured"
    custom_weights      "custom weights from effort_weights config override defaults when present"
    default_to_medium   "Features without effort field are weighted as m (default weight 3)"
    null_when_empty     "Milestone with zero features returns completion_ratio=null"
  }

  verify unit "milestone with xs(done) + xl(pending) returns ratio 1/9"
  verify unit "feature without effort defaults to m=3 weight"
  verify unit "empty milestone returns null completion_ratio"
}

behavior pe_query_release_deliverables "Query Release Deliverables" {
  category   query
  types      [ReleaseDeliverablePayload]
  produces  [pe_release_deliverables_queried]
  contract   """
    The product extension MUST provide a query that returns all deliverables
    grouped under a release via ReleaseDeliverable edges.
  """
  requires   {
    graph_ready    "Graph is built and queryable"
    release_exists "Release ID resolves to a release entity"
  }
  ensures    {
    deliverables_listed "All ReleaseDeliverable targets are included"
    count_matches       "count == deliverables.length"
  }

  verify unit "release with 3 deliverables returns count=3"
  verify unit "release with no deliverables returns empty array and count=0"
}

behavior pe_query_release_milestones "Query Release Milestones" {
  category   query
  types      [ReleaseMilestonePayload]
  produces  [pe_release_milestones_queried]
  contract   """
    The product extension MUST provide a query that returns all milestones
    targeted by a release via ReleaseMilestone edges.
  """
  requires   {
    graph_ready    "Graph is built and queryable"
    release_exists "Release ID resolves to a release entity"
  }
  ensures    {
    milestones_listed "All ReleaseMilestone targets are included"
    count_matches     "count == milestones.length"
  }

  verify unit "release with 2 milestones returns count=2"
}

behavior pe_query_release_completion "Query Release Completion" {
  category   query
  types      [ReleaseCompletionPayload]
  produces  [pe_release_completion_queried]
  contract   """
    The product extension MUST provide a query that computes release
    completion as the ratio of shipped deliverables to total deliverables.
  """
  requires   {
    graph_ready    "Graph is built and queryable"
    release_exists "Release ID resolves to a release entity"
  }
  ensures    {
    shipped_counted   "shipped == count of deliverables with status=shipped"
    ratio_correct     "completion_ratio == shipped / total (null when total=0)"
  }

  verify unit "release with 2/3 shipped returns ratio 0.667"
  verify unit "release with no deliverables returns null ratio"
}

behavior pe_validate_journey_flow_features "Validate Journey Flow Feature References" {
  category   validation
  features   [pe_journey_flow_validation]
  contract   """
    Journey flow steps that contain bracketed entity references SHOULD
    resolve to features declared in the journey's features list.
    Unresolvable references produce I090.

    Extraction regex: \[([a-zA-Z_][a-zA-Z0-9_]{1,59})\]

    This matches a single opening bracket, captures an entity ID
    (2-60 chars: starts with letter or underscore, then letters, digits,
    or underscores), and a closing bracket. The entity ID character set
    matches the spec-wide entity ID format.

    Edge cases:
    - Nested brackets "[[foo]]" — the outer pair is NOT matched; the
      inner "[foo]" IS matched as a valid reference.
    - Empty brackets "[]" — not matched (minimum 2 chars in capture).
    - Escaped brackets "\[foo\]" — not treated as references (the
      backslash prevents the opening bracket from starting a match).
    - Multiple references in one step "Use [auth] then [billing]" —
      each is extracted and validated independently.
    - Non-ID content "[hello world]" — not matched (spaces are not
      valid in entity IDs).
    - Markdown links "[text](url)" — "[text]" is extracted as a
      candidate reference. If "text" is not in the journey's features
      list, I090 fires. Authors should use backticks for code references
      that are not entity IDs: `[not_a_ref]`.

    Each matched reference is checked against the journey's features[]
    field. If the reference is not in the features list, I090 is emitted
    with the unresolved ID and the flow step index.
  """
  ensures    {
    regex_extraction   "references extracted via \\[([a-zA-Z_][a-zA-Z0-9_]{1,59})\\] regex"
    references_checked "each extracted entity_id is checked against the journey's features list"
    unknown_warned     "I090 emitted per unresolved reference with entity_id and step index"
    empty_safe         "empty brackets [] produce no match and no I090"
    multi_match        "multiple references in one step are each validated independently"
    escaped_ignored    "backslash-escaped brackets are not treated as references"
  }

  verify unit "flow step referencing declared feature produces no I090"
  verify unit "flow step referencing undeclared feature produces I090"
  verify unit "flow step without bracketed references produces no I090"
  verify unit "empty brackets [] produce no I090"
  verify unit "nested brackets [[foo]] match inner [foo] only"
  verify unit "multiple references in one step are each validated"
  verify unit "escaped \\[foo\\] does not match as reference"
  verify unit "non-ID content [hello world] does not match as reference"
}
