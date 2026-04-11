// @specforge/product — Query behaviors
//
// All product graph query behaviors: core queries (Phase 1), extended queries
// (Phase 2), persona/feature traversal, feature impact, milestone velocity,
// and partial graph queries.

use "extensions/product/types"
use "product/features"
use "types/diagnostics"

behavior pe_query_milestone_completion "Query Milestone Completion" {
  category   query
  types      [MilestoneCompletionPayload, ProductFeature, FeatureStatus]
  produces  [pe_milestone_completion_queried]
  contract   """
    The @specforge/product extension MUST compute the completion ratio
    for a milestone by counting features with status=done vs total features
    in the milestone. completion_ratio = done_count / total_features.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    ratio_computed     "completion_ratio is done_count / total_features, finite float in [0.0, 1.0]"
    empty_milestone    "milestone with zero features returns completion_ratio 0.0"
    status_based       "completion is based on feature status field, not cross-extension edges"
    no_division_error  "milestone with zero features does not produce division-by-zero"
  }

  features [pe_query_lifecycle_metrics]

  verify unit "milestone with all features status=done returns ratio 1.0"
  verify unit "milestone with no done features returns ratio 0.0"
  verify unit "empty milestone returns ratio 0.0 with zero features"
  verify unit "milestone with mix of done and non-done features returns partial ratio"
  verify unit "milestone completion is deterministic across repeated queries"
}

behavior pe_query_deliverable_traceability "Query Deliverable Traceability" {
  category   query
  types      [DeliverableTraceabilityPayload, ProductDeliverable, ProductTraceabilityPayload]
  produces  [pe_deliverable_traceability_queried, pe_traceability_computed]
  contract   """
    The @specforge/product extension MUST enumerate all transitive features
    reachable from a deliverable via two paths: journeys (DeliverableJourney
    -> JourneyFeature) and modules (DeliverableModule -> ModuleFeature).
    The union of both path sets gives the deliverable's full feature scope.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    both_paths_traced  "features reachable via journeys and modules are both included"
    deduplication      "features reachable via both paths appear once in the result"
    empty_deliverable  "deliverable with no journeys and no modules returns empty transitive_features"
    path_counts        "journey_path_count and module_path_count reflect actual reachable features per path"
  }

  features [pe_query_traceability]

  verify unit "deliverable with journeys and modules returns union of features"
  verify unit "deliverable with only journeys returns journey features"
  verify unit "deliverable with no journeys or modules returns empty feature set"
  verify unit "deliverable with overlapping journey and module features deduplicates"
  verify unit "deliverable traceability is deterministic across repeated queries"
}

behavior pe_query_journey_coverage "Query Journey Coverage" {
  category   query
  types      [JourneyCoveragePayload, ProductJourney, FeatureStatus]
  produces  [pe_journey_coverage_queried]
  contract   """
    The @specforge/product extension MUST check whether features referenced
    by a journey have status=done. Features without status=done are uncovered.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    coverage_computed      "covered_features count reflects features with status=done"
    uncovered_listed       "uncovered feature IDs are returned in the result"
    empty_journey          "journey with zero features returns total_features 0"
    no_status_default      "features without status field are treated as uncovered"
  }

  features [pe_query_coverage_analysis]

  verify unit "journey with all features status=done returns full coverage"
  verify unit "journey with uncovered features lists them"
  verify unit "journey with zero features returns empty coverage"
  verify unit "features without status field are treated as uncovered"
  verify unit "journey coverage is deterministic across repeated queries"
}

behavior pe_query_feature_ordering "Query Feature Ordering" {
  category   query
  types      [FeatureOrderingPayload, ProductFeature, Priority, ProductListSortOrder]
  produces  [pe_feature_ordering_queried]
  contract   """
    The @specforge/product extension MUST produce a topological sort of
    features based on FeatureDependsOn edges. Within each topological
    level (features with identical dependency depth), features are
    secondarily sorted by priority (critical > high > medium > low).
    Features without a priority field default to medium. If the feature
    dependency graph contains cycles, has_cycles MUST be true and
    cycle_members MUST list the features involved in the cycle.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    topological_order  "sorted_features is in dependency order (dependencies before dependents)"
    priority_tiebreak  "features at the same topological level are sorted by priority descending"
    default_priority   "features without a priority field are treated as medium"
    cycle_detection    "cycles are detected and reported via has_cycles and cycle_members"
    empty_graph        "zero features returns empty sorted_features and has_cycles=false"
    stable_ordering    "features with no dependencies and equal priority are sorted alphabetically by entity ID for deterministic stable ordering"
    cycle_members_set  "cycle_members contains the union of all features participating in any cycle; each feature appears at most once"
  }

  features [pe_query_dependency_analysis]

  verify unit "acyclic feature graph returns topological sort"
  verify unit "features at same level sorted by priority descending"
  verify unit "features without priority default to medium"
  verify unit "cyclic feature graph returns has_cycles=true with cycle members"
  verify unit "features with no dependencies return stable ordering"
  verify unit "empty feature graph returns empty sorted list and has_cycles=false"
  verify unit "feature ordering is deterministic across repeated queries"
}

behavior pe_query_milestone_timeline "Query Milestone Timeline" {
  category   query
  types      [MilestoneTimelinePayload, MilestoneTimelineEntry, ProductMilestone]
  produces  [pe_milestone_timeline_queried]
  contract   """
    The @specforge/product extension MUST produce a sorted timeline of all
    milestones. Milestones with target_date are sorted chronologically;
    milestones without target_date appear last (sorted by ID for stability).
    Overdue milestones (target_date < as_of_date AND status != completed)
    are flagged with is_overdue=true in the query result.

    The as_of_date parameter MUST be explicitly provided or default to the
    build timestamp (captured once at build start). This ensures
    deterministic results: the same spec files + the same as_of_date always
    produce the same timeline.

    IMPORTANT: Overdue detection is a QUERY-TIME operation, not a
    compile-time validation. I058 markers appear only in the timeline
    query result (is_overdue field and overdue_count), NOT as diagnostics
    emitted during specforge check. This preserves deterministic
    compilation — specforge check never depends on wall-clock time.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    date_sorted          "milestones with target_date are sorted chronologically (earliest first)"
    undated_last         "milestones without target_date appear after all dated milestones"
    overdue_detected     "milestones with target_date < as_of_date and status != completed have is_overdue=true"
    completed_not_overdue "milestones with status=completed are never marked overdue regardless of target_date"
    empty_timeline       "zero milestones returns empty milestones list and overdue_count=0"
    no_compile_diagnostic "overdue detection does NOT emit I058 during specforge check — query-time only"
    as_of_date_explicit  "as_of_date defaults to build timestamp, not wall-clock time"
    deterministic_tiebreaker "entities at the same topological level and priority are sorted alphabetically by entity ID for deterministic stable ordering; undated milestones and milestones sharing a target_date use entity ID as tiebreaker"
  }

  features [pe_query_lifecycle_metrics]

  verify unit "milestones sorted by target_date ascending"
  verify unit "undated milestones appear after dated milestones"
  verify unit "overdue milestone with status=in_progress is flagged in query result"
  verify unit "completed milestone past target_date is not flagged overdue"
  verify unit "empty milestone set returns empty timeline"
  verify unit "same as_of_date produces identical timeline across repeated queries"
  verify unit "specforge check emits no I058 diagnostics (query-time only)"
  verify unit "milestone timeline is deterministic across repeated queries"
}

behavior pe_query_feature_deliverables "Query Feature Deliverables" {
  category   query
  types      [FeatureDeliverablePayload, ProductFeature, ProductDeliverable]
  produces  [pe_feature_deliverables_queried]
  contract   """
    The @specforge/product extension MUST compute which deliverables
    transitively contain a given feature by traversing reverse paths:
    feature <- JourneyFeature <- journey <- DeliverableJourney <- deliverable
    and feature <- ModuleFeature <- module <- DeliverableModule <- deliverable.
    The union of both path sets gives the feature's full deliverable scope.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    both_paths_traversed "deliverables reachable via reverse journey and module paths are both included"
    deduplication        "deliverables reachable via both paths appear once in the result"
    empty_feature        "feature with no incoming journey or module edges returns empty deliverables"
    path_counts          "via_journey_count and via_module_count reflect actual reachable deliverables per path"
    deterministic        "same graph input always produces same deliverable list in sorted order"
  }

  features [pe_query_traceability]

  verify unit "feature reachable via journey path returns deliverable"
  verify unit "feature reachable via module path returns deliverable"
  verify unit "feature reachable via both paths deduplicates deliverables"
  verify unit "feature with no incoming edges returns empty deliverables"
  verify unit "feature deliverable query is deterministic across repeated queries"
}

behavior pe_query_feature_milestones "Query Feature Milestones" {
  category   query
  types      [FeatureMilestonePayload, ProductFeature, ProductMilestone]
  produces  [pe_feature_milestones_queried]
  contract   """
    The @specforge/product extension MUST compute which milestones
    schedule a given feature by traversing reverse MilestoneFeature edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "milestones found via reverse MilestoneFeature edge traversal"
    empty_feature      "feature not in any milestone returns empty milestones list"
    sorted_by_id       "milestones are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "feature in milestone returns that milestone"
  verify unit "feature in multiple milestones returns all sorted by ID"
  verify unit "feature not in any milestone returns empty list"
}

behavior pe_query_persona_journeys "Query Persona Journeys" {
  category   query
  types      [PersonaJourneyPayload, ProductPersona, ProductJourney]
  produces  [pe_persona_journeys_queried]
  contract   """
    The @specforge/product extension MUST compute which journeys
    reference a given persona by traversing reverse JourneyPersona edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "journeys found via reverse JourneyPersona edge traversal"
    empty_persona      "persona not in any journey returns empty journeys list"
    sorted_by_id       "journeys are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "persona referenced by journey returns that journey"
  verify unit "persona referenced by multiple journeys returns all sorted by ID"
  verify unit "persona not referenced by any journey returns empty list"
}

behavior pe_query_channel_journeys "Query Channel Journeys" {
  category   query
  types      [ChannelJourneyPayload, ProductChannel, ProductJourney]
  produces  [pe_channel_journeys_queried]
  contract   """
    The @specforge/product extension MUST compute which journeys
    reference a given channel by traversing reverse JourneyChannel edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "journeys found via reverse JourneyChannel edge traversal"
    empty_channel      "channel not in any journey returns empty journeys list"
    sorted_by_id       "journeys are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "channel referenced by journey returns that journey"
  verify unit "channel referenced by multiple journeys returns all sorted by ID"
  verify unit "channel not referenced by any journey returns empty list"
}

behavior pe_query_module_deliverables "Query Module Deliverables" {
  category   query
  types      [ModuleDeliverablePayload, ProductModule, ProductDeliverable]
  produces  [pe_module_deliverables_queried]
  contract   """
    The @specforge/product extension MUST compute which deliverables
    contain a given module by traversing reverse DeliverableModule edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "deliverables found via reverse DeliverableModule edge traversal"
    empty_module       "module not in any deliverable returns empty deliverables list"
    sorted_by_id       "deliverables are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "module in deliverable returns that deliverable"
  verify unit "module in multiple deliverables returns all sorted by ID"
  verify unit "module not in any deliverable returns empty list"
}

behavior pe_query_term_graph "Query Term Graph" {
  category   query
  types      [TermGraphPayload, ProductTerm]
  produces  [pe_term_graph_queried]
  contract   """
    The @specforge/product extension MUST compute related terms reachable
    from a given term via N-hop TermSeeAlso traversal. The maxHops
    parameter limits traversal depth (default 1).
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    n_hop_traversal    "related_terms includes terms reachable within maxHops"
    default_one_hop    "maxHops defaults to 1 when not specified"
    max_hops_capped    "maxHops is capped at 5 to prevent unbounded traversal; values > 5 are clamped"
    max_hops_bounded   "maxHops is capped at 5 to prevent combinatorial explosion in densely-connected term graphs; BFS with visited set ensures O(V+E) traversal within the hop limit; values above 5 are silently clamped rather than rejected to preserve agent ergonomics"
    no_self_include    "the source term is not included in related_terms"
    sorted_by_id       "related_terms are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "term with see_also returns related terms at hop 1"
  verify unit "term with 2-hop chain returns transitive terms when maxHops=2"
  verify unit "term with no see_also returns empty related_terms"
  verify unit "source term is excluded from related_terms"
  verify unit "omitted maxHops defaults to 1"
  verify unit "maxHops=10 is clamped to 5"
  verify unit "maxHops=0 returns empty related_terms"
}

behavior pe_query_milestone_deliverables "Query Milestone Deliverables" {
  category   query
  types      [MilestoneDeliverablePayload, ProductMilestone, ProductDeliverable]
  produces  [pe_milestone_deliverables_queried]
  contract   """
    The @specforge/product extension MUST compute which deliverables
    include a given milestone by traversing reverse DeliverableMilestone edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "deliverables found via reverse DeliverableMilestone edge traversal"
    empty_milestone    "milestone not in any deliverable returns empty deliverables list"
    sorted_by_id       "deliverables are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "milestone in deliverable returns that deliverable"
  verify unit "milestone in multiple deliverables returns all sorted by ID"
  verify unit "milestone not in any deliverable returns empty list"
}

behavior pe_query_module_features "Query Module Features" {
  category   query
  types      [ModuleFeaturePayload, ProductModule, ProductFeature]
  produces  [pe_module_features_queried]
  contract   """
    The @specforge/product extension MUST compute which features a given
    module implements by traversing outgoing ModuleFeature edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    forward_traversal  "features found via outgoing ModuleFeature edge traversal"
    empty_module       "module with no features returns empty features list"
    sorted_by_id       "features are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "module with features returns those features"
  verify unit "module with multiple features returns all sorted by ID"
  verify unit "module with no features returns empty list"
}

behavior pe_query_entity_not_found "Query Entity Not Found" {
  category   query
  types      [ProductQueryError, ProductQueryFailedPayload]
  contract   """
    Product graph queries receiving a non-existent entity ID MUST return
    a ProductQueryError with a descriptive message and optionally a
    fuzzy-match suggestion (Levenshtein distance ≤2) for the closest
    entity of the expected kind. Queries MUST NOT panic on missing IDs.
  """
  ensures    {
    error_returned     "non-existent entity ID returns ProductQueryError, not panic"
    suggestion_present "when a close match exists (Levenshtein ≤2), the error includes a suggestion field"
    no_suggestion      "when no close match exists, the error message has no suggestion"
  }

  features [pe_query_dependency_analysis, pe_query_traceability, pe_query_coverage_analysis, pe_query_lifecycle_metrics, product_graph_queries]

  verify unit "query with missing entity ID returns error"
  verify unit "query with missing ID and close match includes suggestion"
  verify unit "query with missing ID and no close match omits suggestion"
}

behavior pe_query_deliverable_completion "Query Deliverable Completion" {
  category   query
  types      [DeliverableCompletionPayload, MilestoneCompletionPayload, ProductDeliverable, ProductMilestone, MilestoneStatus]
  produces  [pe_deliverable_completion_queried]
  contract   """
    The @specforge/product extension MUST compute the aggregate milestone
    completion for a deliverable by traversing all DeliverableMilestone
    edges. completion_ratio = completed_count (milestones with
    status=completed) / milestone_count. Deliverable with zero milestones
    returns completion_ratio 0.0.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    ratio_computed     "completion_ratio is completed_count / milestone_count, finite float in [0.0, 1.0]"
    empty_deliverable  "deliverable with zero milestones returns completion_ratio 0.0"
    details_optional   "milestone_details includes per-milestone completion when requested"
    no_division_error  "deliverable with zero milestones does not produce division-by-zero"
  }

  features [pe_query_lifecycle_metrics, product_health_metric]

  verify unit "deliverable with all completed milestones returns ratio 1.0"
  verify unit "deliverable with no completed milestones returns ratio 0.0"
  verify unit "deliverable with zero milestones returns ratio 0.0"
  verify unit "deliverable with mix of completed and non-completed milestones returns partial ratio"
  verify unit "deliverable completion is deterministic across repeated queries"
}

behavior pe_query_milestone_velocity "Query Milestone Velocity" {
  category   query
  types      [MilestoneVelocityPayload, ProductQueryError]
  produces  [pe_milestone_velocity_queried]
  contract   """
    The @specforge/product extension MUST provide a velocity query for
    milestones that computes: total/done/in_progress/remaining feature
    counts, completion ratio, days elapsed since start_date (or
    target_date if start_date absent) relative to as_of_date, estimated
    days remaining (based on current velocity), and features per day.

    The as_of_date parameter MUST be explicitly provided or default to
    the build timestamp (captured once at build start). This ensures
    deterministic results: the same spec files + the same as_of_date
    always produce the same velocity metrics.

    When no features are done, features_per_day is null. When neither
    start_date nor target_date is present, days_elapsed and
    days_remaining are null.
  """
  requires   {
    graph_ready       "product graph is in ready state"
    milestone_exists  "milestone with given ID exists in the graph"
  }
  ensures    {
    counts_correct      "total = done + in_progress + remaining"
    ratio_correct       "completion_ratio = done / total when total > 0"
    velocity_correct    "features_per_day = done / days_elapsed when both > 0"
    null_when_unknown   "features_per_day is null when done_features == 0"
    days_null_no_date   "days_elapsed and days_remaining are null when no date fields present"
    as_of_date_explicit "as_of_date defaults to build timestamp, not wall-clock time"
    deterministic       "same inputs + same as_of_date = same velocity result"
    not_found_suggests  "missing milestone ID returns ENTITY_NOT_FOUND with suggestion"
  }

  features [pe_query_lifecycle_metrics]

  verify unit "milestone with 3 done and 2 remaining returns correct counts"
  verify unit "milestone with no done features returns null velocity"
  verify unit "milestone without target_date returns null days_elapsed"
  verify unit "velocity calculation is mathematically correct"
}

// -- Persona-Feature Traversal -----------------------------------------------

behavior pe_query_persona_features "Query Persona Features" {
  category   query
  types      [PersonaFeaturePayload, ProductQueryError]
  produces  [pe_persona_features_queried]
  contract   """
    The @specforge/product extension MUST provide a multi-hop query that
    traverses persona -> JourneyPersona -> journey -> JourneyFeature ->
    feature to return all features reachable from a given persona. The
    result includes the intermediate journey IDs for traceability. This
    answers the question: "What features does this persona need?"
  """
  requires   {
    graph_ready  "product graph is in ready state"
    persona_exists "persona with given ID exists in the graph"
  }
  ensures    {
    multi_hop_traversal  "traverses persona->journey->feature via JourneyPersona and JourneyFeature edges"
    deduplicates         "features appearing in multiple journeys are listed once"
    includes_journeys    "via_journey_ids contains all intermediate journeys traversed"
    count_matches        "count equals features array length"
    not_found_suggests   "missing persona ID returns ENTITY_NOT_FOUND with fuzzy-match suggestions"
  }

  features [pe_query_traceability]

  verify unit "persona with one journey returns that journey's features"
  verify unit "persona with multiple journeys returns deduplicated features"
  verify unit "persona with no journeys returns empty features"
  verify unit "nonexistent persona returns ENTITY_NOT_FOUND with suggestion"
}

// -- Feature Impact Analysis -------------------------------------------------

behavior pe_query_feature_impact "Query Feature Impact" {
  category   query
  types      [FeatureImpactPayload, ProductQueryError]
  produces  [pe_feature_impact_queried]
  contract   """
    The @specforge/product extension MUST provide a transitive impact
    analysis query that, given a feature ID, returns all entities that
    would be affected if the feature were deferred or removed. The query
    traverses: reverse JourneyFeature -> affected journeys, reverse
    MilestoneFeature -> affected milestones, reverse ModuleFeature ->
    affected modules, then DeliverableJourney/DeliverableModule ->
    affected deliverables, and forward FeatureDependsOn -> dependent
    features. total_affected_entities is the count of all unique affected
    entities across all categories.
  """
  requires   {
    graph_ready     "product graph is in ready state"
    feature_exists  "feature with given ID exists in the graph"
  }
  ensures    {
    transitive_traversal    "follows all reverse edge paths from feature"
    includes_dependents     "dependent_features includes transitive FeatureDependsOn reverse"
    deliverables_via_both   "affected_deliverables found via both journey and module paths"
    total_is_union          "total_affected_entities is the deduplicated union of all arrays"
    not_found_suggests      "missing feature ID returns ENTITY_NOT_FOUND with suggestion"
  }

  features [product_impact_and_whatif]

  verify unit "feature in one journey and one milestone returns both as affected"
  verify unit "feature with dependent features includes transitive dependents"
  verify unit "feature with no references returns zero affected entities"
  verify unit "affected deliverables found via both journey and module paths"
  verify unit "total_affected_entities is deduplicated count"
}

// -- Unscheduled Features ----------------------------------------------------

behavior pe_query_unscheduled_features "Query Unscheduled Features" {
  category   query
  types      [UnscheduledFeaturesPayload]
  produces  [pe_unscheduled_features_queried]
  contract   """
    The @specforge/product extension MUST provide a query that returns
    all features not scheduled in any milestone. A feature is unscheduled
    if it has zero incoming MilestoneFeature edges. The result MUST include
    total_features (all features in graph), scheduled_count, and the
    unscheduled feature IDs. This enables planners and agents to identify
    features that exist but have not been committed to any release phase.
  """
  requires   {
    graph_ready "product graph is in ready state"
  }
  ensures    {
    correct_set       "returned features have zero MilestoneFeature incoming edges"
    exhaustive        "no unscheduled feature is omitted"
    count_consistent  "count == features.length AND count + scheduled_count == total_features"
  }

  features [pe_query_coverage_analysis, pe_planning_insights]

  verify unit "feature in no milestone appears in unscheduled list"
  verify unit "feature in one milestone is excluded from unscheduled list"
  verify unit "empty graph returns empty list"
  verify unit "count + scheduled_count == total_features"
}

behavior pe_query_feature_overlap "Query Cross-Deliverable Feature Overlap" {
  category   query
  types      [FeatureOverlapPayload, FeatureOverlapEntry]
  produces  [pe_feature_overlap_queried]
  contract   """
    The @specforge/product extension MUST provide a query that returns
    features shared across multiple deliverables. A feature overlaps if
    it is reachable from 2+ deliverables via DeliverableJourney→JourneyFeature
    or DeliverableModule→ModuleFeature paths. The result MUST list each
    overlapping feature with its containing deliverable IDs. This enables
    release planners to identify shared dependencies across delivery streams.
  """
  requires   {
    graph_ready "product graph is in ready state"
  }
  ensures    {
    correct_overlap    "each returned feature is reachable from 2+ deliverables"
    exhaustive         "no overlapping feature is omitted"
    deliverable_ids    "each entry lists all deliverable IDs containing the feature"
    dedup              "each feature appears at most once in the result"
  }

  features [pe_query_coverage_analysis]

  verify unit "feature in two deliverables appears in overlap list"
  verify unit "feature in one deliverable is excluded"
  verify unit "feature reachable via journey path and module path from same deliverable counts once"
  verify unit "diamond topology correctly deduplicates"
}

behavior pe_query_persona_coverage_matrix "Query Persona Coverage Matrix" {
  category   query
  types      [PersonaCoverageMatrixPayload, PersonaCoverageEntry]
  produces  [pe_persona_coverage_matrix_queried]
  contract   """
    The @specforge/product extension MUST provide a query that computes
    a coverage matrix showing which features each persona can reach via
    their journeys. For each persona, the result MUST include: reachable
    features (via JourneyPersona→journey→JourneyFeature traversal),
    unreachable features (all features minus reachable), coverage_ratio
    (reachable/total), and journey count. The overall_coverage is the
    mean of all persona coverage_ratios. Personas with zero journeys
    get coverage_ratio=0.0.
  """
  requires   {
    graph_ready "product graph is in ready state"
  }
  ensures    {
    per_persona         "one entry per persona in the graph"
    reachable_correct   "reachable features match persona→journey→feature traversal"
    unreachable_correct "unreachable = total_features - reachable"
    ratio_correct       "coverage_ratio = reachable.length / total_features"
    zero_journey        "persona with zero journeys has coverage_ratio=0.0"
    overall_mean        "overall_coverage = mean of all persona coverage_ratios"
  }

  features [pe_query_coverage_analysis]

  verify unit "persona with journeys covering all features has coverage_ratio=1.0"
  verify unit "persona with no journeys has coverage_ratio=0.0"
  verify unit "overall_coverage is arithmetic mean of persona ratios"
  verify unit "diamond topology deduplicates shared features"
}

behavior pe_query_channel_coverage_matrix "Query Channel Coverage Matrix" {
  category   query
  types      [ChannelCoverageMatrixPayload, ChannelCoverageEntry, PaginatedQueryInput, PaginationMetadata]
  ports      [ProductQueryPort, GraphQueryPort]
  produces  [pe_channel_coverage_matrix_queried]
  contract   """
    The @specforge/product extension MUST provide a query that computes
    a coverage matrix showing which features each channel can reach via
    its journeys. For each channel, the result MUST include: reachable
    features (via JourneyChannel→journey→JourneyFeature traversal),
    unreachable features (all features minus reachable), coverage_ratio
    (reachable/total), and journey count. The overall_coverage is the
    mean of all channel coverage_ratios. Channels with zero journeys
    get coverage_ratio=0.0. This is the symmetric counterpart to
    pe_query_persona_coverage_matrix.
  """
  requires   {
    graph_ready "product graph is in ready state"
  }
  ensures    {
    per_channel         "one entry per channel in the graph"
    reachable_correct   "reachable features match channel→journey→feature traversal"
    unreachable_correct "unreachable = total_features - reachable"
    ratio_correct       "coverage_ratio = reachable.length / total_features"
    zero_journey        "channel with zero journeys has coverage_ratio=0.0"
    overall_mean        "overall_coverage = mean of all channel coverage_ratios"
    deduplicated        "shared features across journeys are counted once per channel"
    sorted_by_id        "channels sorted alphabetically by entity ID"
    deterministic       "same graph always produces identical result"
    product_edges_only  "traversal uses only JourneyChannel and JourneyFeature edges"
  }

  features [pe_query_coverage_analysis]

  verify unit "channel with journeys covering all features has coverage_ratio=1.0"
  verify unit "channel with no journeys has coverage_ratio=0.0"
  verify unit "overall_coverage is arithmetic mean of channel ratios"
  verify unit "diamond topology deduplicates shared features"
  verify unit "empty channel graph returns empty matrix"
}

behavior pe_query_critical_path "Query Critical Path" {
  category   query
  types      [CriticalPathPayload, CriticalPathNode]
  produces  [pe_critical_path_queried]
  contract   """
    The @specforge/product extension MUST provide a query that computes
    the critical path through the milestone dependency graph. The critical
    path is the longest chain of milestones (by MilestoneDependsOn edges)
    where each milestone is not yet completed. Nodes on the critical path
    have zero slack (any delay pushes the overall completion date). The
    result MUST include the ordered path, earliest/latest completion dates
    (derived from target_date fields), and bottleneck IDs (blocked or
    in_progress milestones on the critical path). Milestones without
    target_date are included but contribute no date information.
  """
  requires   {
    graph_ready "product graph is in ready state"
    no_cycles   "milestone dependency graph is acyclic (E015 not fired)"
  }
  ensures    {
    longest_path        "critical path is the longest dependency chain"
    zero_slack          "all nodes on the path have slack_days=0 or null"
    ordered             "path is ordered from earliest to latest"
    bottleneck_correct  "bottleneck_ids are blocked/in_progress milestones on the path"
    date_derived        "earliest_completion from first milestone, latest from last"
    cycle_safe          "returns empty path if cycles exist (E015 fired)"
  }

  features [pe_query_dependency_analysis]

  verify unit "linear chain of 3 milestones returns all 3 as critical path"
  verify unit "parallel chains return the longer one"
  verify unit "completed milestones are excluded from critical path"
  verify unit "milestones without target_date still appear on path"
  verify unit "graph with cycles returns empty path"
}

// -- Persona Channels --------------------------------------------------------

behavior pe_query_persona_channels "Query Persona Channels" {
  category   query
  types      [PersonaChannelPayload, ProductPersona, ProductJourney, ProductChannel]
  produces  [pe_persona_channels_queried]
  contract   """
    The @specforge/product extension MUST compute which channels a given
    persona uses by traversing the multi-hop path: persona <- JourneyPersona
    <- journey -> JourneyChannel -> channel. Returns a deduplicated channel
    list for a persona.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    multi_hop_traversal  "channels found via persona <- JourneyPersona <- journey -> JourneyChannel -> channel"
    deduplication        "channels reachable via multiple journeys appear once in the result"
    empty_persona        "persona with no journeys returns empty channels list"
    sorted_by_id         "channels are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic        "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "persona with one journey returns that journey's channels"
  verify unit "persona with multiple journeys returns deduplicated channels"
  verify unit "persona with no journeys returns empty channel list"
  verify unit "persona channels query is deterministic across repeated queries"
}

// -- Journey Deliverables ----------------------------------------------------

behavior pe_query_journey_deliverables "Query Journey Deliverables" {
  category   query
  types      [JourneyDeliverablePayload, ProductJourney, ProductDeliverable]
  produces  [pe_journey_deliverables_queried]
  contract   """
    The @specforge/product extension MUST compute which deliverables
    contain a given journey by traversing reverse DeliverableJourney edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "deliverables found via reverse DeliverableJourney edge traversal"
    empty_journey      "journey not in any deliverable returns empty deliverables list"
    sorted_by_id       "deliverables are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "journey in deliverable returns that deliverable"
  verify unit "journey in multiple deliverables returns all sorted by ID"
  verify unit "journey not in any deliverable returns empty list"
}

// -- Feature Dependents ------------------------------------------------------

behavior pe_query_feature_dependents "Query Feature Dependents" {
  invariants [pe_queries_derived_not_standard]
  category   query
  types      [FeatureDependentPayload, ProductFeature]
  produces  [pe_feature_dependents_queried]
  contract   """
    The @specforge/product extension MUST compute which features depend
    on a given feature by traversing reverse FeatureDependsOn edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "dependents found via reverse FeatureDependsOn edge traversal"
    empty_feature      "feature with no dependents returns empty dependents list"
    sorted_by_id       "dependents are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_dependency_analysis]

  verify unit "feature with dependent returns that dependent"
  verify unit "feature with multiple dependents returns all sorted by ID"
  verify unit "feature with no dependents returns empty list"
}

// -- Deliverable Dependents --------------------------------------------------

behavior pe_query_deliverable_dependents "Query Deliverable Dependents" {
  category   query
  types      [DeliverableDependentPayload, ProductDeliverable]
  produces  [pe_deliverable_dependents_queried]
  contract   """
    The @specforge/product extension MUST compute which deliverables depend
    on a given deliverable by traversing reverse DeliverableDependsOn edges.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    reverse_traversal  "dependents found via reverse DeliverableDependsOn edge traversal"
    empty_deliverable  "deliverable with no dependents returns empty dependents list"
    sorted_by_id       "dependents are returned sorted alphabetically by entity ID for deterministic ordering"
    deterministic      "same graph input always produces same result"
  }

  features [pe_query_traceability]

  verify unit "deliverable with dependent returns that dependent"
  verify unit "deliverable with multiple dependents returns all sorted by ID"
  verify unit "deliverable with no dependents returns empty list"
}

// -- Deliverable Priority ----------------------------------------------------

behavior pe_query_deliverable_priority "Query Deliverable Priority" {
  category   query
  types      [DeliverablePriorityPayload, ProductDeliverable, ProductMilestone, ProductJourney, Priority]
  produces  [pe_deliverable_priority_queried]
  contract   """
    The @specforge/product extension MUST derive deliverable priority from
    constituent milestones and journeys. Algorithm: highest priority among
    all milestones (via DeliverableMilestone) and journeys (via
    DeliverableJourney) referenced by the deliverable that have an
    explicit priority field. Entities without a priority field are excluded
    from the derivation (not treated as medium).
    critical > high > medium > low. Deliverable returns null priority when:
    (a) no milestones and no journeys exist, or (b) all constituent
    milestones and journeys have null/absent priority.
    source_count reflects only entities WITH explicit priority, not total.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    highest_priority       "derived priority is the highest among constituents with explicit priority"
    priority_ordering      "priority ordering: critical > high > medium > low"
    null_when_empty        "deliverable with no milestones and no journeys returns null priority"
    null_when_all_absent   "deliverable where all constituents have null priority returns null priority"
    excludes_null_priority "constituents without an explicit priority field are excluded from derivation"
    source_counted         "source_count reflects only entities with explicit priority, not total entities"
    deterministic          "same graph input always produces same result"
  }

  features [pe_query_lifecycle_metrics]

  verify unit "deliverable with critical milestone returns critical priority"
  verify unit "deliverable with high journey and medium milestone returns high priority"
  verify unit "deliverable with no milestones and no journeys returns null priority"
  verify unit "deliverable where all constituents have null priority returns null priority"
  verify unit "deliverable with one prioritized and five unprioritized returns the one priority"
  verify unit "source_count counts only entities with explicit priority"
  verify unit "deliverable priority is deterministic across repeated queries"
}

// -- Deliverable-Persona Composite Query -------------------------------------

behavior pe_query_deliverable_personas "Query Deliverable Personas" {
  category   query
  types      [DeliverablePersonaPayload, ProductDeliverable, ProductJourney, ProductPersona]
  ports      [ProductQueryPort, GraphQueryPort]
  produces  [pe_deliverable_personas_queried]
  contract   """
    The @specforge/product extension MUST compute which personas a
    deliverable serves by traversing: deliverable -> DeliverableJourney ->
    journey -> JourneyPersona -> persona. Returns the deduplicated set of
    personas with the intermediate journey IDs that connect them. This is
    a convenience query avoiding the two-hop traversal that would otherwise
    be required to answer "which personas does this deliverable serve?"
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    two_hop_traversal     "personas found via deliverable->journey->persona edge traversal"
    deduplicated          "each persona appears at most once in the result"
    includes_via_journeys "via_journey_ids lists all intermediate journeys"
    empty_deliverable     "deliverable with no journeys returns empty personas list"
    empty_journeys        "deliverable with journeys that have no persona returns empty personas list"
    sorted_by_id          "personas are returned sorted alphabetically by entity ID"
    deterministic         "same graph input always produces same result"
    product_edges_only    "traversal uses only DeliverableJourney and JourneyPersona edges"
  }

  features [pe_query_traceability]

  verify unit "deliverable with one journey and one persona returns that persona"
  verify unit "deliverable with multiple journeys sharing a persona deduplicates"
  verify unit "deliverable with no journeys returns empty list"
  verify unit "deliverable with journeys without personas returns empty list"
  verify unit "result includes via_journey_ids for traceability"
}

// -- Partial Graph Queries ---------------------------------------------------

behavior pe_query_partial_graph "Query Behavior on Graphs with Validation Errors" {
  invariants [pe_partial_graph_traversability]
  category   query
  types     [ProductEntitySummary]
  contract   """
    Product queries MUST operate on the structural graph regardless of
    validation state. Entities with validation errors (orphans, broken
    references, invalid field values) MUST still be traversable and appear
    in query results. Queries MUST NOT filter entities based on diagnostic
    state. The only condition that blocks queries is an active graph rebuild.
    Consumers can cross-reference query results
    with validation diagnostics to identify quality issues.
  """
  requires   {
    graph_built            "entity graph is built (not mid-rebuild)"
  }
  ensures    {
    errors_traversable     "entities with E-level diagnostics appear in query results"
    warnings_traversable   "entities with W-level diagnostics appear in query results"
    no_diagnostic_filter   "query results are identical whether validation has run or not"
    rebuild_blocks         "queries during active rebuild return GRAPH_NOT_READY error"
  }

  features [pe_partial_graph_queries, product_graph_diff]

  verify unit "milestone-completion includes milestones with E015 cycle diagnostic"
  verify unit "journey-coverage includes journeys with W042 orphan diagnostic"
  verify unit "feature-ordering includes features with W045 cycle warning"
  verify unit "query results identical before and after validation pass"
  verify unit "GRAPH_NOT_READY returned during active rebuild only"
}

// -- Channel-Feature Traversal -----------------------------------------------

behavior pe_query_channel_features "Query Channel Features" {
  category   query
  types      [ChannelFeaturePayload, ProductChannel, ProductJourney, ProductFeature]
  ports      [ProductQueryPort, GraphQueryPort]
  produces  [pe_channel_features_queried]
  contract   """
    The @specforge/product extension MUST compute which features are reachable
    from a channel by traversing: channel -> JourneyChannel (reverse) -> journey
    -> JourneyFeature -> feature. Returns the deduplicated set of features with
    the intermediate journey IDs that connect them. This is the symmetric
    counterpart to pe_query_persona_features (persona->journey->feature) and
    closes the channel→features query gap.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    two_hop_traversal     "features found via channel->journey->feature edge traversal"
    deduplicated          "each feature appears at most once in the result"
    includes_via_journeys "via_journey_ids lists all intermediate journeys"
    empty_channel         "channel with no journeys returns empty features list"
    empty_journeys        "channel with journeys that have no features returns empty features list"
    sorted_by_id          "features are returned sorted alphabetically by entity ID"
    deterministic         "same graph input always produces same result"
    product_edges_only    "traversal uses only JourneyChannel and JourneyFeature edges"
  }

  features [pe_query_traceability]

  verify unit "channel with one journey and features returns those features"
  verify unit "channel with multiple journeys sharing features deduplicates"
  verify unit "channel with no journeys returns empty list"
  verify unit "channel with journeys without features returns empty list"
  verify unit "result includes via_journey_ids for traceability"
}

// ── Term Analytics Queries ──────────────────────────────────

behavior pe_query_term_clusters "Query Term Clusters" {
  category   query
  types      [TermClusterPayload, TermCluster]
  ports      [ProductQueryPort, GraphQueryPort]
  produces  [pe_term_clusters_queried]
  contract   """
    The @specforge/product extension MUST compute connected components in
    the TermSeeAlso subgraph. Each cluster is a set of terms reachable
    from each other via TermSeeAlso edges (treated as undirected for
    clustering). Terms with zero TermSeeAlso edges are "isolated" and
    reported in isolated_count but not included in any cluster. Clusters
    are sorted by size descending (largest first), with ties broken by
    alphabetical order of the first term ID. This provides a global
    overview of glossary structure that pe_query_term_graph (single-root
    BFS) cannot offer.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    connected_components "each cluster is a maximal connected component via TermSeeAlso"
    undirected_treatment "TermSeeAlso edges are treated as undirected for clustering"
    isolated_excluded    "terms with zero TermSeeAlso edges are counted in isolated_count, not in clusters"
    no_overlap           "no term appears in more than one cluster"
    total_correct        "sum of all cluster term_counts + isolated_count == total_terms"
    sorted_by_size       "clusters sorted by term_count descending, ties broken alphabetically"
    deterministic        "same graph always produces identical cluster assignments"
    product_edges_only   "only TermSeeAlso edges are traversed"
  }

  features [pe_query_traceability]

  verify unit "three terms in a connected chain produce one cluster of size 3"
  verify unit "two disconnected pairs produce two clusters of size 2"
  verify unit "isolated term with no see_also edges is counted in isolated_count"
  verify unit "empty term graph returns zero clusters and zero isolated"
  verify unit "total_terms equals sum of cluster sizes plus isolated_count"
  verify unit "clusters sorted by size descending"
  verify unit "result is deterministic across repeated queries"
}

behavior pe_query_term_density "Query Term Density" {
  category   query
  types      [TermDensityPayload]
  ports      [ProductQueryPort, GraphQueryPort]
  produces  [pe_term_density_queried]
  contract   """
    The @specforge/product extension MUST compute connectivity statistics
    for the TermSeeAlso subgraph: total terms, total see_also edges,
    average connections per term, maximum connections, hub terms (terms
    with connections > 2× average, minimum 3 connections), and isolated
    terms (zero connections). This metric helps identify glossary health:
    too many isolated terms suggests poor linking, while hub terms are
    central vocabulary. avg_connections is null when total_terms is 0.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    avg_computed          "avg_connections = total_see_also / total_terms (null when total_terms=0)"
    max_computed          "max_connections is the highest TermSeeAlso degree across all terms"
    hub_threshold         "hub_terms have connections > 2 * avg_connections AND >= 3"
    isolated_zero_edges   "isolated_terms have exactly zero TermSeeAlso edges"
    counts_consistent     "hub_terms + isolated_terms <= total_terms"
    deterministic         "same graph always produces identical density metrics"
    product_edges_only    "only TermSeeAlso edges are counted"
  }

  features [pe_query_traceability]

  verify unit "5 terms with 8 edges computes correct average"
  verify unit "term with 6 connections in a graph averaging 2 is a hub"
  verify unit "term with zero connections is listed in isolated_terms"
  verify unit "empty term graph returns total_terms=0 and avg_connections=null"
  verify unit "max_connections reflects the most-connected term"
  verify unit "result is deterministic across repeated queries"
}

// ── Module Analytics Queries ────────────────────────────────

behavior pe_query_module_dependency_depth "Query Module Dependency Depth" {
  category   query
  types      [ModuleDependencyDepthPayload]
  ports      [ProductQueryPort, GraphQueryPort]
  produces  [pe_module_dependency_depth_queried]
  contract   """
    The @specforge/product extension MUST compute the longest dependency
    chain starting from a given module, following ModuleDependsOn edges.
    depth is the number of edges in the longest path (0 if the module
    has no dependencies). longest_chain lists the module IDs in order
    from the queried module to the leaf. If the module participates in
    a cycle (E007), depth is -1 and longest_chain contains the cycle
    members. This enables architects to identify deep dependency chains
    that increase build/test coupling.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    depth_computed        "depth is the number of edges in the longest ModuleDependsOn chain"
    chain_ordered         "longest_chain lists module IDs from queried module to leaf"
    zero_for_no_deps      "module with no depends_on returns depth=0 and chain=[moduleId]"
    cycle_returns_minus_1 "module in a cycle returns depth=-1 and chain=cycle members"
    deterministic         "same graph always produces identical result"
    product_edges_only    "only ModuleDependsOn edges are traversed"
  }

  features [pe_query_dependency_analysis]

  verify unit "module with no dependencies returns depth=0"
  verify unit "module depending on two layers returns depth=2"
  verify unit "module in a cycle returns depth=-1"
  verify unit "longest_chain includes all modules in the longest path"
  verify unit "non-existent module returns ENTITY_NOT_FOUND with suggestion"
  verify unit "result is deterministic across repeated queries"
}

behavior pe_query_module_coupling "Query Module Coupling" {
  category   query
  types      [ModuleCouplingPayload, ModuleCouplingEntry]
  ports      [ProductQueryPort, GraphQueryPort]
  produces  [pe_module_coupling_queried]
  contract   """
    The @specforge/product extension MUST compute coupling metrics for
    all modules in the product graph. For each module: fan_in is the
    number of incoming ModuleDependsOn edges (how many modules depend
    on it), fan_out is the number of outgoing ModuleDependsOn edges
    (how many modules it depends on), and coupling = fan_in + fan_out.
    The result includes averages and identifies the most-coupled module.
    This enables architects to spot over-coupled modules that may need
    decomposition or interface stabilization.
  """
  requires   {
    graph_ready "product graph is built and in ready state"
  }
  ensures    {
    fan_in_correct        "fan_in counts incoming ModuleDependsOn edges"
    fan_out_correct       "fan_out counts outgoing ModuleDependsOn edges"
    coupling_sum          "coupling = fan_in + fan_out for each module"
    avg_computed          "avg_fan_in and avg_fan_out are means across all modules (null when total=0)"
    most_coupled_correct  "most_coupled_id has the highest coupling value (alphabetical tiebreak)"
    all_modules_included  "every module entity appears in the result"
    sorted_by_coupling    "modules sorted by coupling descending, ties broken alphabetically"
    deterministic         "same graph always produces identical result"
    product_edges_only    "only ModuleDependsOn edges are counted"
  }

  features [pe_query_dependency_analysis]

  verify unit "module with 3 dependents and 1 dependency has fan_in=3 fan_out=1 coupling=4"
  verify unit "isolated module has fan_in=0 fan_out=0 coupling=0"
  verify unit "empty module graph returns empty modules array"
  verify unit "most_coupled_id identifies the highest-coupling module"
  verify unit "averages are computed correctly across all modules"
  verify unit "result is deterministic across repeated queries"
}
