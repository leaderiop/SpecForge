decision D-019 "Split Formal Analysis from Software Extension" {
  settled true
  origin  project
  text    """
    Extract all formal analysis content from @specforge/software into a
    new @specforge/formal extension. The formal extension operates through
    entity_enhancements on @specforge/software entities for inline
    condition blocks (requires/ensures/maintains). This follows the
    "extensions over built-ins" principle (P5) and keeps the software
    extension focused on core entity registration. Users opt into formal
    analysis by installing @specforge/formal and setting warning_level=strict.
  """
}

decision D-018 "Expand Formal Extension to 4 Entity Kinds" {
  settled true
  origin  project
  text    """
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

    Later (ADR 0040): the range was not kept. W059-W060 were removed
    with the condition entity kind, W058 (feature coverage mismatch)
    was trimmed for want of a sound algorithm, and W060-W062 are core
    codes now. Formal's entity validation uses W123-W136.
  """
}

decision D-020 "Add Refinement and Process Entity Kinds" {
  settled true
  origin  project
  text    """
    Add refinement and process as entity kinds 4 and 5. Both testable=false,
    supports_verify=false. 4 new edge types: RefinesTo, RefinementChainLink,
    ParticipatesIn, ProcessComposition. 2 new error codes: E041 (refinement
    chain cycle), E042 (process composition cycle). 6 new warnings: W069-W074,
    since renumbered W131-W136 (refinement W131-W133, process W134-W136).
    Entity enhancements: behavior gets refinement field, event gets process field.
    Total: 5 entity kinds, 8 edge types, 23 project entities.
  """
}

decision D-021 "Rename Formal Methods Terminology" {
  settled true
  origin  project
  text    """
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
    - E033 downgraded to W058 (structural check, not semantic verification);
      W058 was later trimmed (ADR 0040)
    - E034 now checks for mitigations before firing; the mitigation is a
      declared sync on a member of the cycle (ADR 0040)
  """
}
