requirement INVARIANT-080 "Module DAG" {
  category "Invariant"
  tier     v2
  text     """
    The depends_on edges between module entities MUST form a directed
    acyclic graph. Cycles are detected by E007 and rejected.
  """
}

requirement INVARIANT-077 "Milestone DAG" {
  category "Invariant"
  tier     v2
  text     """
    The depends_on edges between milestone entities MUST form a directed
    acyclic graph. Cycles are detected by E015 and rejected.
  """
}

requirement INVARIANT-041 "Feature DAG" {
  category "Invariant"
  tier     v2
  text     """
    The depends_on edges between feature entities MUST form a directed
    acyclic graph. Cycles produce a W045 warning.
  """
}

requirement INVARIANT-014 "Deliverable DAG" {
  category "Invariant"
  tier     v2
  text     """
    The depends_on edges between deliverable entities MUST form a
    directed acyclic graph. Cycles are detected by E052 and rejected.
  """
}

requirement INVARIANT-122 "Release DAG" {
  category "Invariant"
  tier     v2
  text     """
    The depends_on edges between release entities MUST form a directed
    acyclic graph. Cycles produce a W092 warning.
  """
}

requirement INVARIANT-090 "Feature Non-Testable" {
  category "Invariant"
  tier     v2
  text     """
    Feature entities MUST have testable=false in the manifest. Features
    are planning constructs tested indirectly through behavior chains.
  """
}

requirement INVARIANT-099 "Persona Non-Testable" {
  category "Invariant"
  tier     v2
  text     """
    Persona entities MUST have testable=false in the manifest. Personas
    are reference entities describing user roles, not testable contracts.
  """
}

requirement INVARIANT-085 "Channel Non-Testable" {
  category "Invariant"
  tier     v2
  text     """
    Channel entities MUST have testable=false in the manifest. Channels
    are reference entities describing interaction mediums.
  """
}

requirement INVARIANT-103 "Release Non-Testable" {
  category "Invariant"
  tier     v2
  text     """
    Release entities MUST have testable=false in the manifest. Releases
    are coordination constructs, not testable contracts.
  """
}

requirement INVARIANT-100 "Product Verify Support" {
  category "Invariant"
  tier     v2
  text     """
    No product entity kind supports verify statements: all 9 kinds have
    supportsVerify=false and declare no verify kinds.
  """
}

requirement INVARIANT-016 "Deliverable Lifecycle Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Deliverable status transitions MUST follow the declared state machine.
    Shipped deliverables MUST have completed milestones. Deprecated
    deliverables MUST have a documented reason.
  """
}

requirement INVARIANT-113 "Persona Lifecycle Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Deprecated personas MUST have a documented reason field. Persona
    status transitions MUST follow the declared state machine
    (active->deprecated, deprecated is terminal).
  """
}

requirement INVARIANT-004 "Channel Lifecycle Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Deprecated channels MUST have a documented reason field. Channel
    status transitions MUST follow the declared state machine
    (active->deprecated, deprecated is terminal).
  """
}

requirement INVARIANT-112 "Persona and Channel Lifecycle" {
  category "Invariant"
  tier     v2
  text     """
    Persona and channel entities MUST have valid lifecycle states.
    The status field, when present, MUST be a valid enum value.
  """
}

requirement INVARIANT-079 "Milestone Status Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Milestone status MUST be a valid MilestoneStatus enum value.
    Completed milestones MUST have exit criteria. Blocked milestones
    SHOULD have dependencies.
  """
}

requirement INVARIANT-102 "Release Lifecycle Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Released releases MUST have all deliverables in shipped status.
    Recalled releases MUST have a documented reason.
  """
}

requirement INVARIANT-104 "Release Status Transition" {
  category "Invariant"
  tier     v2
  text     """
    Release status transitions MUST follow the declared state machine:
    planned->in_progress, in_progress->released, released->recalled.
    Recalled is terminal.
  """
}

requirement INVARIANT-015 "Deliverable Journey-Module Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Features referenced by a deliverable's journeys MUST be a subset
    of features assigned to the deliverable's modules.
  """
}

requirement INVARIANT-078 "Milestone Feature-Module Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Features scheduled in a milestone MUST be reachable from the
    milestone's modules via ModuleContainsFeature edges.
  """
}

requirement INVARIANT-081 "Module-Feature Reachability" {
  category "Invariant"
  tier     v2
  text     """
    Every module SHOULD have at least one feature assigned via
    ModuleContainsFeature edges.
  """
}

requirement INVARIANT-087 "Deliverable Chain Integrity" {
  category "Invariant"
  tier     v2
  text     """
    The full deliverable->milestone->feature->module chain MUST be
    consistent. Features scheduled in milestones that belong to a
    deliverable MUST be covered by the deliverable's modules.
  """
}

requirement INVARIANT-097 "Ownership Field Awareness" {
  category "Invariant"
  tier     v2
  text     """
    Owner and contributors fields are available on feature, milestone,
    deliverable, and release entity kinds. Other entity kinds
    (module, term, journey, persona, channel) intentionally lack
    ownership fields.
  """
}

requirement INVARIANT-096 "Owner String Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Owner strings across entities SHOULD be consistent. Similar owner
    strings within Levenshtein distance 2 produce I085 diagnostics
    suggesting normalization.
  """
}

requirement INVARIANT-108 "Tags Per Entity Kind" {
  text     "Every product entity kind gets the shared tags field (string[] @optional)."
  category "Invariant"
  tier     v2
}

requirement INVARIANT-136 "Tag Cross-Kind Awareness" {
  category "Invariant"
  tier     v2
  text     """
    Tags used on entities of 3 or more different kinds produce I084
    info diagnostics suggesting kind-specific prefixes.
  """
}

requirement INVARIANT-139 "Term Alias Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    Term aliases MUST be unique across all terms (case-insensitive).
    Conflicts between aliases or between an alias and a term ID
    produce W086 warnings.
  """
}

requirement INVARIANT-089 "Effort-Weighted Completion" {
  category "Invariant"
  tier     v2
  text     """
    Weighted milestone completion MUST use configured effort weights.
    Default weights: xs=1, s=2, m=3, l=5, xl=8. Features without
    effort default to the weight of m.
  """
}

requirement INVARIANT-088 "Effort Enum Validity" {
  category "Invariant"
  tier     v2
  text     """
    The effort field on features MUST contain a valid Effort enum
    value (xs, s, m, l, xl). Another value is E061.
  """
}

requirement INVARIANT-107 "Surface Error Consistency" {
  category "Invariant"
  tier     v2
  text     """
    All product commands MUST use exactly two error codes:
    ENTITY_NOT_FOUND (exit 1) and INVALID_INPUT (exit 2), written to
    stderr with nothing on stdout.
  """
}

requirement INVARIANT-091 "List Pagination Correctness" {
  category "Invariant"
  tier     v2
  text     """
    All paged commands (the lists and the matrix queries) MUST return
    correct pagination: total is the count after filters and before
    paging, has_more is total > offset + returned count, limit defaults
    to 100 (clamped to [1, 1000]), offset defaults to 0.
  """
}

requirement INVARIANT-086 "Cross-Extension Query Isolation" {
  category "Invariant"
  tier     v2
  text     """
    Product queries MUST NOT traverse edges owned by other extensions.
    Query results MUST be identical regardless of which other
    extensions are installed.
  """
}

requirement INVARIANT-101 "Queries Are Derived, Not Standard" {
  category "Invariant"
  tier     v2
  text     """
    Product queries are a derived convenience layer, NOT part of the
    Graph Protocol standard. Alternative compilers need not implement
    these queries to be compliant.
  """
}

requirement INVARIANT-098 "Partial Graph Traversability" {
  category "Invariant"
  tier     v2
  text     """
    Product queries MUST operate on the structural graph regardless
    of validation state. Entities with validation errors MUST still
    be traversable and appear in query results.
  """
}

requirement INVARIANT-106 "Rendering Completeness" {
  category "Invariant"
  tier     v2
  text     """
    All 9 product entity kinds MUST render as standard graph nodes
    in the Graph Protocol JSON output. The core emitter handles all
    entity kinds uniformly.
  """
}

requirement INVARIANT-095 "Milestone Temporal Consistency" {
  category "Invariant"
  tier     v2
  text     """
    When both start_date and target_date are present on a milestone,
    start_date MUST be on or before target_date. Both fields MUST
    be date fields: E061 otherwise.
  """
}

requirement INVARIANT-084 "Blocker-Status Consistency" {
  category "Invariant"
  tier     v2
  text     """
    Blocked milestones SHOULD list blockers entries that document the
    cause of the block; depends_on entries do not replace them. A blocked
    milestone with no blockers produces I060.
  """
}

requirement INVARIANT-094 "Migration Backward Compatibility" {
  category "Invariant"
  tier     v2
  text     """
    Minor version bumps (1.x) MUST be backward compatible. New fields
    are always optional. Breaking changes require a major version bump
    with a migration hook.
  """
}

requirement INVARIANT-109 "Validation Deterministic" {
  category "Invariant"
  tier     v2
  text     """
    Product validation MUST be deterministic: identical inputs (spec
    files + HEAD) always produce identical diagnostic
    output. No time-dependent diagnostics during specforge check.
  """
}

requirement INVARIANT-105 "Release Version Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    Release version strings MUST be unique across all release entities.
    Duplicate versions produce I091 info diagnostics.
  """
}
