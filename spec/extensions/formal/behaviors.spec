// @specforge/formal behaviors — graph annotations + graph views
//
// New behaviors for attaching analysis results to graph nodes and
// declaring named graph views for agent consumption.

use "extensions/formal/types"
use "types/zero-entity-core"

behavior fa_annotate_graph_with_results "Annotate Graph with Analysis Results" {
  category command
  types    [FormalAnalysisAnnotation, ConditionAnalysisResult, LayeringAnalysisResult, CycleAnalysisResult, CoverageAnalysisResult]
  contract """
    After each analysis pass completes, annotate the relevant graph
    nodes with structured analysis results (FormalAnalysisAnnotation).
    Annotations are queryable via the graph protocol and included in
    export formats when --include-annotations is set.
  """
  requires {
    pass_completed         "at least one analysis pass has completed"
    graph_available        "entity graph is available for annotation"
  }
  ensures  {
    condition_annotations  "condition_check pass results annotated on behavior nodes"
    condition_entity_ann   "condition entities annotated with referencing behavior count and edge types"
    layering_annotations   "layering_verify pass results annotated on behavior nodes"
    refinement_annotations "layering_verify pass results annotated on refinement nodes"
    cycle_annotations      "event_graph_analyze pass results annotated on event nodes"
    process_annotations    "event_graph_analyze pass results annotated on process nodes"
    coverage_annotations   "coverage_tracking pass results annotated on all analyzed nodes"
    queryable              "annotations queryable via query_graph host function"
    export_included        "annotations included in export when --include-annotations flag is set"
  }

  features [fa_graph_annotations]

  verify unit "condition analysis results annotated on behavior nodes"
  verify unit "condition entity nodes annotated with referencing behaviors"
  verify unit "layering analysis results annotated on behavior nodes"
  verify unit "cycle analysis results annotated on event nodes"
  verify unit "coverage results annotated on all analyzed nodes"
  verify unit "annotations included in export with --include-annotations"
}

behavior fa_declare_graph_views "Declare Named Graph Views" {
  category command
  contract """
    Declare named subgraph extractions that agents can request for
    focused context. Each view filters the full graph to a relevant
    subset. Views are registered in the graph protocol and available
    via specforge export --view=<name>.
  """
  ensures  {
    behavior_overview      "behavior-overview view: all behaviors with their features, invariants, and verify statements"
    type_graph             "type-graph view: all types with ExtendsType and UsesType edges"
    event_flow             "event-flow view: all events with Produces/Consumes edges and sync blocks"
    traceability_chain     "traceability-chain view: verify statements -> tests -> coverage items"
    formal_analysis        "formal-analysis view: all nodes with FormalAnalysisAnnotation attached"
    condition_graph        "condition-graph view: all condition entities with RequiresCondition/EnsuresCondition/MaintainsCondition edges to behaviors"
    property_graph         "property-graph view: all property entities with Satisfies edges from behaviors and PropertyDependsOn edges to conditions"
    axiom_graph            "axiom-graph view: all axiom entities with AssumedBy edges from conditions"
    protocol_graph         "protocol-graph view: all protocol entities with FollowsProtocol edges from events"
    refinement_graph       "refinement-graph view: all refinement entities with RefinesTo edges to behaviors and RefinementChainLink edges"
    process_graph          "process-graph view: all process entities with ParticipatesIn edges from events and ProcessComposition edges"
  }

  features [fa_graph_annotations]

  verify unit "behavior-overview view contains behaviors, features, invariants"
  verify unit "type-graph view contains types with ExtendsType edges"
  verify unit "event-flow view contains events with Produces/Consumes edges"
  verify unit "traceability-chain view follows verify -> tests -> coverage"
  verify unit "formal-analysis view contains only annotated nodes"
  verify unit "condition-graph view contains condition entities with edges to behaviors"
  verify unit "property-graph view contains property entities with Satisfies edges"
  verify unit "axiom-graph view contains axiom entities with AssumedBy edges"
  verify unit "protocol-graph view contains protocol entities with FollowsProtocol edges"
  verify unit "refinement-graph view contains refinement entities with RefinesTo edges"
  verify unit "process-graph view contains process entities with ParticipatesIn edges"
}
