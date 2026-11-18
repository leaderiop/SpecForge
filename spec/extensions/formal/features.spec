requirement FEATURE-041 "Structured Conditions" {
  category "Feature"
  tier     v2
  text     """
    Behavior contract fields are free-form prose. Preconditions and
    postconditions are mixed together in a single text block. There is
    no structured way to declare named conditions, check consistency,
    or generate coverage tracking items from conditions. AI agents
    cannot reliably extract preconditions from prose. Conditions that
    apply to multiple behaviors must be duplicated inline.

    Structured requires/ensures/maintains blocks with named conditions
    augment (not replace) prose contracts (progressive specification
    depth Level 2). Inline blocks produce ConditionEntry nodes in the
    AST. Shared, reusable constraints are modeled as invariant entities
    (contributed by @specforge/software), which are already first-class
    graph nodes. @specforge/formal contributes 5 entity kinds: property
    (temporal assertions), axiom (assumed-true foundations), protocol
    (sync contracts), refinement (abstract-to-concrete behavior mappings),
    and process (CSP-style communicating processes).

    The condition_check compiler pass (specforge analyze) checks which
    condition blocks are written, not what they say: a behavior that
    requires without ensuring (W096), one whose requires names a
    condition twice (W039), one that ensures without requiring (I011),
    and an invariant whose guarantee has no expression (W040).
    Named conditions map 1:1 to test assertions and coverage tracking
    items. A compile notes that specforge analyze would check the
    conditions it found (I015).
  """
}

requirement FEATURE-040 "Specification Layering" {
  category "Feature"
  tier     v2
  text     """
    There is no way to express that a behavior is an abstract
    specification refined by concrete implementations. The existing
    entity graph is flat — all behaviors are at the same level. This
    makes it impossible to model progressive elaboration from high-
    level requirements to detailed implementation behaviors.

    The abstract/refines mechanism creates layering chains between
    behaviors. Abstract behaviors serve as specification-only entries.
    Concrete behaviors declare refines to build a DAG. The layering
    verify pass checks completeness (every abstract has a concrete),
    correctness (refined preserves abstract's guarantees), and DAG
    structure (no cycles). RefinementChain uses a linked-list of
    RefinementStep entries preserving per-step condition deltas rather
    than a flat list. The refinement entity kind provides first-class
    graph nodes for abstract-to-concrete mappings with condition deltas
    and proof status. RefinementRefinesAbstract/RefinementRefinesConcrete edges connect
    refinements to behaviors; RefinementChainsToRefinement edges support
    multi-level refinement. Dual-mode:
    refinement entities coexist with abstract/refines field annotations.
    W131-W133 validation rules detect unreferenced refinements, empty
    descriptions, and missing invariant deltas.
  """
}

requirement FEATURE-034 "Event Graph Linting" {
  category "Feature"
  tier     v2
  text     """
    Event entities declare producers but the compiler performs no
    event flow analysis. Circular event dependencies, payload type
    mismatches, unmatched producers, and retry loops are invisible
    until runtime. Distributed system failures are the hardest bugs
    to reproduce.

    The event_graph_analyze compiler pass builds an event-behavior
    bipartite graph and performs structural event flow analysis.
    Tarjan's SCC algorithm detects unmitigated cycles (E034 — a cycle
    passes when any member declares a sync), and a cycle-free graph is
    noted (I009). Producer/consumer pairing detects lost side effects
    (W029). Retry cycle analysis flags a behavior re-producing an event
    it consumes with no sync (W032). Asymmetric connectivity warnings on
    ports (W033). Unbounded channel buffer detection: a produced event
    with no sync (W034). Sync blocks declare barrier, timeout, and delivery
    semantics constraints. The process entity kind models CSP-style
    communicating processes with alphabet, states, and composition
    operators. ParticipatesIn edges connect events to processes;
    ProcessComposition edges model hierarchical composition, whose
    cycles are E042. Dual-mode: process entities
    coexist with inline sync blocks. W134-W136 validation rules detect
    unreferenced processes, empty descriptions, and missing alphabets.
  """
}

requirement FEATURE-033 "Coverage Tracking" {
  category "Feature"
  tier     v2
  text     """
    There is no way to know whether formal properties (conditions,
    invariants, layering chains) are verified by existing tests.
    Coverage is binary (has test / no test) rather than graduated.
    Coverage tracking items are invisible — users cannot see what
    remains unverified.

    The coverage_tracking pass generates machine-readable items
    categorized as condition_coverage, invariant_coverage,
    layering_coverage, or process_coverage. Each item tracks discharge status across 5
    states: pending, test_written, test_failing, test_covered,
    heuristic_ok. W035 emits a single summary per compilation with
    breakdown by kind and drill-down command reference; I008 names the
    tests that prove an item. Coverage evolves from binary to graduated:
    I014 reports each behavior's specification depth, Level 4 (proofs)
    only when the coverage rule holds it proven.
  """
}

requirement FEATURE-031 "Formal Analysis CLI Commands" {
  category "Feature"
  tier     v2
  text     """
    Formal analysis results are only available as part of the full
    compilation pipeline. There is no way to run specific analyses
    in isolation, get machine-readable output for CI integration,
    or fail builds on formal violations.

    The specforge analyze subcommands (conditions, layering,
    event-graph, all) run individual or combined analysis passes.
    Each supports --format json for machine-readable output and --strict
    for CI gate integration (non-zero exit on any violation).
  """
}

requirement FEATURE-035 "Graph Annotations" {
  category "Feature"
  tier     v2
  text     """
    Analysis results are emitted as diagnostics but not attached to
    the entity graph. Agents cannot query per-node analysis results
    without parsing diagnostic output. There is no structured way to
    ask "what did the condition check find for this behavior?"

    Each analysis pass annotates graph nodes with structured results
    (FormalAnalysisAnnotation). Annotations are queryable via the
    graph protocol and included in export formats. Agent-optimized
    graph views (behavior-overview, type-graph, event-flow,
    traceability-chain, formal-analysis) provide pre-filtered
    subgraphs for common agent queries.
  """
}

requirement FEATURE-042 "Temporal Properties" {
  category "Feature"
  tier     v2
  text     """
    There is no way to declare temporal/behavioral assertions (safety,
    liveness, fairness) as first-class entities. Such properties are
    distinct from point-in-time conditions: they assert behavior over
    TIME. Without a property entity, temporal assertions are scattered
    across prose contracts and cannot be queried or traced.

    The property entity kind represents a temporal/behavioral assertion
    with a kind classifier (safety/liveness/fairness). Behaviors
    declare which properties they satisfy via the satisfies field,
    creating Satisfies edges. Properties can depend on conditions via
    PropertyDependsOn edges. Properties no behavior satisfies (W123, where behaviors load), empty
    descriptions (W124) are detected, and a property_type outside its
    vocabulary is the core's E061.
  """
}

requirement FEATURE-032 "Axiom Foundations" {
  category "Feature"
  tier     v2
  text     """
    Conditions implicitly depend on assumptions that are never stated.
    For example, a condition "database is available" rests on the
    unstated assumption that the network is reachable. Without explicit
    axioms, these foundations are invisible and cannot be traced.

    The axiom entity kind represents an assumed-true foundation that
    conditions depend on. Axioms require no proof and generate no
    coverage tracking items. Conditions reference axioms via the
    assumes field, creating AssumedBy edges. Unreferenced axioms (W126)
    and empty descriptions (W127) are detected.
  """
}

requirement FEATURE-038 "Protocol Contracts" {
  category "Feature"
  tier     v2
  text     """
    Synchronization constraints are declared as inline sync blocks on
    individual events. When multiple events share the same sync
    contract, the constraint is duplicated. There is no way to declare
    a shared protocol and have events reference it.

    The protocol entity kind represents a shared synchronization
    contract with ordering, timeout, and delivery semantics. Events
    reference protocols via the follows_protocol field, creating
    FollowsProtocol edges. Protocols no event follows (W128, where events load) and empty
    descriptions (W129) are detected. Protocols
    coexist with inline sync blocks — dual-mode like conditions.
  """
}

requirement FEATURE-039 "Refinement Entities" {
  category "Feature"
  tier     v2
  text     """
    Specification layering is expressed only through field annotations
    (abstract/refines) on behaviors. The abstract-to-concrete mapping has
    no first-class graph representation. Condition deltas are implicit.
    Multi-level refinement chains cannot be queried as connected subgraphs.

    The refinement entity kind captures abstract-to-concrete behavior
    mappings as first-class graph nodes with condition deltas and proof
    status. RefinementRefinesAbstract/RefinementRefinesConcrete edges connect
    refinements to target behaviors. RefinementChainsToRefinement edges
    support multi-level refinement. Dual-mode:
    coexists with abstract/refines field annotations. W131-W133 validation.
  """
}

requirement FEATURE-036 "Process Modeling" {
  category "Feature"
  tier     v2
  text     """
    Event graph linting operates at the event-behavior bipartite graph
    level but has no concept of communicating processes. Events participate
    in sync blocks but cannot be grouped into logical processes with
    alphabets, states, and composition operators. Deadlock analysis cannot
    reason about process-level composition.

    The process entity kind models CSP-style communicating processes with
    alphabet (event set), states, and composition operators. ParticipatesIn
    edges connect events to processes. ProcessComposition edges model
    hierarchical composition. Dual-mode: coexists with inline sync
    blocks. W134-W136 validation.
  """
}

requirement FEATURE-037 "Progressive Warning Levels" {
  category "Feature"
  tier     v2
  text     """
    All formal analysis warnings (W029-W035, W039, W040, W096, W110, W123-W129, W131-W136) fire at the same level.
    New users are overwhelmed by formal analysis warnings they cannot
    act on yet. There is no way to gradually increase warning
    strictness as the project matures.

    Three warning levels (onboarding, standard, strict) control which
    warnings are emitted. Formal analysis warnings (W029-W035, W039, W040, W096, W110, W123-W129, W131-W136) require
    warning_level=strict. Basic warnings (W001-W010) fire at all
    levels. The warning_level is set in specforge.json or
    CompilerConfig. Default is standard.
  """
}
