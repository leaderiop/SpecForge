// @specforge/formal structured conditions — requires/ensures/maintains blocks + port contracts
//
// Moved from @specforge/software formal-contracts.spec per 10-expert panel.
// Terminology: "Design by Contract" -> "Structured Conditions"
// All behavior IDs renamed se_ -> fa_
//
// Conditions are inline blocks on behaviors that reference invariants.
// Inline:    requires { name "description" }
// Inline conditions produce ConditionEntry nodes in the AST.

use "extensions/formal/invariants"
use "extensions/formal/types"
use "types/zero-entity-core"

// ── Structured Conditions ───────────────────────────────────

behavior fa_parse_requires_block "Parse Requires Block" {
  category command
  types    [RequiresBlock, ConditionEntry]
  contract """
    Recognize the requires field on behavior entities. Parses inline
    blocks into ordered ConditionEntry lists:
    - Inline block: requires { name "description" } — parsed as ordered
      ConditionEntry list (inline conditions, not graph nodes)
  """
  requires {
    entity_is_behavior     "the enclosing entity has kind=behavior"
  }
  ensures  {
    inline_parsed          "inline requires block parsed into RequiresBlock AST node with ordered ConditionEntry list"
    named_conditions       "each inline condition has a name (identifier) and description (string)"
    empty_permitted        "empty requires block produces empty ConditionEntry list"
    non_behavior_warned    "requires block on non-behavior entity produces warning"
  }

  features [fa_structured_conditions]

  verify unit "inline requires block with named conditions parsed"
  verify unit "empty requires block produces empty ConditionEntry list"
  verify unit "requires block on non-behavior entity produces warning"
}

behavior fa_parse_ensures_block "Parse Ensures Block" {
  category command
  types    [EnsuresBlock, ConditionEntry]
  contract """
    Recognize the ensures field on behavior entities. Parses inline
    blocks into ordered ConditionEntry lists:
    - Inline block: ensures { name "description" } — parsed as ordered
      ConditionEntry list (inline conditions, not graph nodes)
  """
  requires {
    entity_is_behavior     "the enclosing entity has kind=behavior"
  }
  ensures  {
    inline_parsed          "inline ensures block parsed into EnsuresBlock AST node with ordered ConditionEntry list"
    named_conditions       "each inline condition has a name (identifier) and description (string)"
    empty_permitted        "empty ensures block produces empty ConditionEntry list"
    standalone_info        "ensures without requires produces info diagnostic"
  }

  features [fa_structured_conditions]

  verify unit "inline ensures block with named conditions parsed"
  verify unit "empty ensures block produces empty ConditionEntry list"
  verify unit "ensures without requires produces info diagnostic"
}

behavior fa_parse_maintains_block "Parse Maintains Block" {
  category command
  types    [MaintainsBlock, ConditionEntry]
  contract """
    Recognize the maintains field on behavior and invariant entities as
    frame invariants (must hold before AND after). Parses inline blocks:
    - Inline block: maintains { name "description" }
  """
  ensures  {
    behavior_permitted     "maintains block on behavior entity parsed"
    invariant_permitted    "maintains block on invariant entity parsed"
    other_warned           "maintains block on feature/event/type/port produces warning"
    named_conditions       "each inline condition has a name and description"
  }

  features [fa_structured_conditions]

  verify unit "maintains block on behavior parsed"
  verify unit "maintains block on invariant parsed"
  verify unit "maintains block on feature produces warning"
}

behavior fa_validate_condition_consistency "Validate Condition Consistency" {
  category   query
  invariants [fa_condition_consistency]
  types      [RequiresBlock, EnsuresBlock, MaintainsBlock, ConditionEntry]
  contract   """
    Cross-check requires, ensures, and maintains blocks for structural
    consistency of condition names and scopes. This is a heuristic
    structural check, not formal semantic analysis.
  """
  requires   {
    blocks_parsed          "requires, ensures, maintains blocks are parsed into AST"
  }
  ensures    {
    scope_checked          "postconditions referencing undefined state produce warning"
    maintains_consistent   "maintains conditions consistent with requires and ensures"
    missing_requires_info  "ensures without requires produces I011 info"
  }

  features [fa_structured_conditions]

  verify unit "consistent requires and ensures passes"
  verify unit "ensures referencing undefined state produces warning"
  verify unit "maintains consistent with requires and ensures passes"
  verify unit "ensures without requires produces I011 info"
}

behavior fa_condition_check_pass "Condition Check Compiler Pass" {
  category   command
  invariants [fa_condition_consistency]
  types      [RequiresBlock, EnsuresBlock, ConditionEntry]
  produces  [fa_condition_check_complete]
  contract   """
    The condition_check compiler pass validates all behaviors with
    requires/ensures blocks after graph construction. Note: this
    performs heuristic structural checks on named conditions, not
    formal semantic analysis. Satisfiability and reachability checks
    operate on condition name patterns and scope relationships.

    E030 pattern catalog: X/not_X contradiction, tautological false,
    empty domain intersection between conditions.
    E031 pattern catalog: ensures names set inclusion check —
    refined ensures MUST be superset of abstract ensures names.
  """
  requires   {
    graph_constructed      "entity graph is fully built"
    conditions_parsed      "all requires/ensures blocks are parsed"
  }
  ensures    {
    satisfiability_checked "preconditions checked for structural satisfiability (not always false)"
    reachability_checked   "postconditions checked for structural reachability from preconditions"
    invariant_consistency  "conditions cross-checked with referenced invariants"
    layering_compliance    "refined behaviors checked: no precondition strengthening, no postcondition weakening (named-condition set inclusion, not logical entailment)"
    e030_on_contradiction  "structurally contradictory precondition produces E030 (patterns: X/not_X, tautological false, empty domain intersection)"
    e031_on_layering       "layering condition mismatch (named-condition set violation) produces E031 (pattern: ensures names must be superset of abstract ensures names)"
  }

  features [fa_structured_conditions]

  verify unit "satisfiable precondition passes"
  verify unit "structurally contradictory precondition produces E030"
  verify unit "precondition strengthening in layering produces E031"
  verify unit "postcondition weakening in layering produces E031"
  verify unit "pass runs after graph construction"
}

behavior fa_detect_unverifiable_condition "W037: Unverifiable Condition" {
  category query
  types    [ConditionEntry, RequiresBlock, EnsuresBlock]
  contract """
    Detect conditions (in requires or ensures) that cannot
    be verified because they reference external state, use ambiguous
    language, or are tautologically trivial.
  """
  ensures  {
    unverifiable_warned    "condition referencing external state produces W037"
    verifiable_passes      "condition with clear, checkable predicate passes"
    suggestion             "W037 includes suggestion for how to make the condition verifiable"
  }

  features [fa_structured_conditions]

  verify unit "condition referencing unknown state produces W037"
  verify unit "condition with clear predicate passes"
}

behavior fa_detect_unreachable_postcondition "W038: Unreachable Postcondition" {
  category query
  types    [EnsuresBlock, RequiresBlock, ConditionEntry]
  contract """
    Detect postconditions that can never be true given the
    preconditions. A postcondition contradicting a precondition
    indicates a condition error.
  """
  ensures  {
    unreachable_warned     "postcondition contradicting precondition produces W038"
    reachable_passes       "postcondition consistent with preconditions passes"
    suggestion             "W038 includes suggestion to fix the contradictory condition"
  }

  features [fa_structured_conditions]

  verify unit "contradictory postcondition produces W038"
  verify unit "consistent postcondition passes"
}

behavior fa_detect_redundant_precondition "W039: Redundant Precondition" {
  category query
  types    [RequiresBlock, ConditionEntry]
  contract """
    Detect preconditions that are implied by other preconditions
    in the same requires block or by the entity's type constraints.
  """
  ensures  {
    redundant_warned       "precondition implied by another produces W039"
    non_redundant_passes   "independent precondition passes"
    suggestion             "W039 includes suggestion to remove the redundant condition"
  }

  features [fa_structured_conditions]

  verify unit "precondition implied by sibling produces W039"
  verify unit "independent precondition passes"
}

behavior fa_detect_invariant_without_property "W040: Invariant Without Formal Property" {
  category query
  types    [MaintainsBlock]
  contract """
    Detect invariant entities that have a prose guarantee but no
    structured maintains block. Formal properties enable automated
    checking; prose-only invariants rely on manual review.
  """
  ensures  {
    prose_only_warned      "invariant with guarantee but no maintains block produces W040"
    formal_passes          "invariant with maintains block passes"
    suggestion             "W040 includes suggestion to add maintains block for automated checking"
  }

  features [fa_structured_conditions]

  verify unit "invariant with prose-only guarantee produces W040"
  verify unit "invariant with maintains block passes"
}

// ── Port Conditions ──────────────────────────────────────────

behavior fa_parse_port_operation_conditions "Parse Port Operation Conditions" {
  category command
  types    [RequiresBlock, EnsuresBlock]
  contract """
    Recognize requires/ensures blocks on individual port operations
    inside a port entity's methods block.
  """
  ensures  {
    conditions_parsed      "port operation with requires/ensures blocks parsed"
    consistency_validated  "port operation conditions validated for internal consistency"
  }

  features [fa_structured_conditions]

  verify unit "port operation with requires/ensures parsed"
  verify unit "port operation conditions validated for consistency"
}

behavior fa_validate_port_behavior_compatibility "W036: Port-Behavior Condition Compatibility" {
  category query
  types    [RequiresBlock, EnsuresBlock]
  contract """
    Check that port operation conditions are compatible with the
    conditions of behaviors that use the port.
  """
  requires {
    port_conditions_parsed    "port operation conditions are parsed"
    behavior_conditions_parsed "behavior requires/ensures blocks are parsed"
  }
  ensures  {
    compatible_passes      "compatible port and behavior conditions produce no diagnostic"
    strict_precond_warned  "port precondition stricter than behavior precondition produces W036"
    weak_postcond_warned   "port postcondition weaker than behavior postcondition produces W036"
    suggestion             "W036 includes suggestion to align port and behavior conditions"
  }

  features [fa_structured_conditions]

  verify unit "compatible port and behavior conditions pass"
  verify unit "stricter port precondition produces W036"
  verify unit "weaker port postcondition produces W036"
}

// ── Warn on conditions without formal verify ─────────────────

behavior fa_validate_conditions_without_verify "W028: Conditions Without Formal Verify" {
  category query
  types    [RequiresBlock, EnsuresBlock]
  contract """
    Detect behaviors that have requires/ensures blocks but no verify
    statement with kind contract or property. Structured conditions
    without corresponding formal verification are untested specifications.
  """
  requires {
    conditions_parsed      "requires/ensures blocks are parsed"
  }
  ensures  {
    missing_verify_warned  "behavior with conditions but no contract/property verify produces W028"
    formal_verify_passes   "behavior with conditions and contract or property verify passes"
    no_conditions_exempt   "behavior without conditions never produces W028"
    suggestion             "W028 includes suggestion to add verify contract or verify property"
  }

  features [fa_structured_conditions]

  verify unit "behavior with conditions but no contract/property verify produces W028"
  verify unit "behavior with conditions and contract verify passes"
  verify unit "behavior with conditions and property verify passes"
  verify unit "behavior without conditions never produces W028"
}
