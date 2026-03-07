// @specforge/software formal contracts — DbC + port contracts

use types/zero-entity-core
use extensions/software/types
use extensions/software/invariants

// ── Design by Contract (DbC) ────────────────────────────────

behavior se_parse_requires_block "Parse Requires Block" {
  category command
  types [RequiresBlock, ContractCondition]

  contract """
    Recognize the requires { } block on behavior entities as a
    collection of named preconditions.
  """

  requires {
    entity_is_behavior     "the enclosing entity has kind=behavior"
  }

  ensures {
    block_parsed           "requires block parsed into RequiresBlock AST node with ordered ContractCondition list"
    named_conditions       "each condition has a name (identifier) and description (string)"
    empty_permitted        "empty requires block produces empty ContractCondition list"
    non_behavior_warned    "requires block on non-behavior entity produces warning"
  }

  verify unit "requires block with named conditions parsed"
  verify unit "empty requires block produces empty ContractCondition list"
  verify unit "requires block on non-behavior entity produces warning"

}

behavior se_parse_ensures_block "Parse Ensures Block" {
  category command
  types [EnsuresBlock, ContractCondition]

  contract """
    Recognize the ensures { } block on behavior entities as a
    collection of named postconditions.
  """

  requires {
    entity_is_behavior     "the enclosing entity has kind=behavior"
  }

  ensures {
    block_parsed           "ensures block parsed into EnsuresBlock AST node with ordered ContractCondition list"
    named_conditions       "each condition has a name (identifier) and description (string)"
    empty_permitted        "empty ensures block produces empty ContractCondition list"
    standalone_info        "ensures without requires produces info diagnostic"
  }

  verify unit "ensures block with named conditions parsed"
  verify unit "empty ensures block produces empty ContractCondition list"
  verify unit "ensures without requires produces info diagnostic"

}

behavior se_parse_maintains_block "Parse Maintains Block" {
  category command
  types [MaintainsBlock, ContractCondition]

  contract """
    Recognize the maintains { } block on behavior and invariant
    entities as frame invariants (must hold before AND after).
  """

  ensures {
    behavior_permitted     "maintains block on behavior entity parsed"
    invariant_permitted    "maintains block on invariant entity parsed"
    other_warned           "maintains block on feature/event/type/port produces warning"
    named_conditions       "each condition has a name and description"
  }

  verify unit "maintains block on behavior parsed"
  verify unit "maintains block on invariant parsed"
  verify unit "maintains block on feature produces warning"

}

behavior se_validate_contract_consistency "Validate Contract Consistency" {
  category query
  invariants [se_formal_contract_consistency]
  types [RequiresBlock, EnsuresBlock, MaintainsBlock, ContractCondition]

  contract """
    Cross-check requires, ensures, and maintains blocks for
    internal consistency.
  """

  requires {
    blocks_parsed          "requires, ensures, maintains blocks are parsed into AST"
  }

  ensures {
    scope_checked          "postconditions referencing undefined state produce warning"
    maintains_consistent   "maintains conditions consistent with requires and ensures"
    missing_requires_info  "ensures without requires produces I011 info"
  }

  verify unit "consistent requires and ensures passes"
  verify unit "ensures referencing undefined state produces warning"
  verify unit "maintains consistent with requires and ensures passes"
  verify unit "ensures without requires produces I011 info"

}

behavior se_contract_check_pass "Contract Check Compiler Pass" {
  category command
  invariants [se_formal_contract_consistency]
  types [RequiresBlock, EnsuresBlock, ContractCondition]

  contract """
    The contract_check compiler pass validates all behaviors with
    requires/ensures blocks after graph construction.
  """

  requires {
    graph_constructed      "entity graph is fully built"
    contracts_parsed       "all requires/ensures blocks are parsed"
  }

  ensures {
    satisfiability_checked "preconditions checked for satisfiability (not always false)"
    reachability_checked   "postconditions checked for reachability from preconditions"
    invariant_consistency  "contracts cross-checked with referenced invariants"
    liskov_compliance      "refined behaviors checked: no precondition strengthening, no postcondition weakening"
    e030_on_unverifiable   "always-false precondition produces E030"
    e031_on_liskov         "precondition strengthening or postcondition weakening in refinement produces E031"
  }

  verify unit "satisfiable precondition passes"
  verify unit "always-false precondition produces E030"
  verify unit "precondition strengthening in refinement produces E031"
  verify unit "postcondition weakening in refinement produces E031"
  verify unit "pass runs after graph construction"

}

behavior se_detect_unverifiable_condition "W037: Unverifiable Contract Condition" {
  category query
  types [ContractCondition, RequiresBlock, EnsuresBlock]

  contract """
    Detect contract conditions (in requires or ensures) that cannot
    be verified because they reference external state, use ambiguous
    language, or are tautologically trivial.
  """

  ensures {
    unverifiable_warned    "condition referencing external state produces W037"
    verifiable_passes      "condition with clear, checkable predicate passes"
  }

  verify unit "condition referencing unknown state produces W037"
  verify unit "condition with clear predicate passes"

}

behavior se_detect_unreachable_postcondition "W038: Unreachable Postcondition" {
  category query
  types [EnsuresBlock, RequiresBlock, ContractCondition]

  contract """
    Detect postconditions that can never be true given the
    preconditions. A postcondition contradicting a precondition
    indicates a contract error.
  """

  ensures {
    unreachable_warned     "postcondition contradicting precondition produces W038"
    reachable_passes       "postcondition consistent with preconditions passes"
  }

  verify unit "contradictory postcondition produces W038"
  verify unit "consistent postcondition passes"

}

behavior se_detect_redundant_precondition "W039: Redundant Precondition" {
  category query
  types [RequiresBlock, ContractCondition]

  contract """
    Detect preconditions that are implied by other preconditions
    in the same requires block or by the entity's type constraints.
  """

  ensures {
    redundant_warned       "precondition implied by another produces W039"
    non_redundant_passes   "independent precondition passes"
  }

  verify unit "precondition implied by sibling produces W039"
  verify unit "independent precondition passes"

}

behavior se_detect_invariant_without_property "W040: Invariant Without Formal Property" {
  category query
  types [SoftwareInvariant, MaintainsBlock]

  contract """
    Detect invariant entities that have a prose guarantee but no
    structured maintains block. Formal properties enable automated
    checking; prose-only invariants rely on manual review.
  """

  ensures {
    prose_only_warned      "invariant with guarantee but no maintains block produces W040"
    formal_passes          "invariant with maintains block passes"
  }

  verify unit "invariant with prose-only guarantee produces W040"
  verify unit "invariant with maintains block passes"

}

// ── Port Contracts ───────────────────────────────────────────

behavior se_parse_port_operation_contracts "Parse Port Operation Contracts" {
  category command
  types [PortOperation, RequiresBlock, EnsuresBlock, SoftwarePort]

  contract """
    Recognize requires/ensures blocks on individual port operations
    inside a port entity's methods block.
  """

  ensures {
    contracts_parsed       "port operation with requires/ensures blocks parsed"
    consistency_validated  "port operation contracts validated for internal consistency"
  }

  verify unit "port operation with requires/ensures parsed"
  verify unit "port operation contracts validated for consistency"

}

behavior se_validate_port_behavior_compatibility "W036: Port-Behavior Contract Compatibility" {
  category query
  types [SoftwarePort, PortOperation, SoftwareBehavior, RequiresBlock, EnsuresBlock]

  contract """
    Check that port operation contracts are compatible with the
    contracts of behaviors that use the port.
  """

  requires {
    port_contracts_parsed  "port operation contracts are parsed"
    behavior_contracts_parsed "behavior requires/ensures blocks are parsed"
  }

  ensures {
    compatible_passes      "compatible port and behavior contracts produce no diagnostic"
    strict_precond_warned  "port precondition stricter than behavior precondition produces W036"
    weak_postcond_warned   "port postcondition weaker than behavior postcondition produces W036"
  }

  verify unit "compatible port and behavior contracts pass"
  verify unit "stricter port precondition produces W036"
  verify unit "weaker port postcondition produces W036"

}

// ── R7: Warn on contracts without formal verify ─────────────

behavior se_validate_contracts_without_verify "W028: Contracts Without Formal Verify" {
  category query
  types [SoftwareBehavior, RequiresBlock, EnsuresBlock]

  contract """
    Detect behaviors that have requires/ensures blocks but no verify
    statement with kind contract or property. Formal contracts without
    corresponding formal verification are untested specifications.
  """

  requires {
    contracts_parsed       "requires/ensures blocks are parsed"
  }

  ensures {
    missing_verify_warned  "behavior with contracts but no contract/property verify produces W028"
    formal_verify_passes   "behavior with contracts and contract or property verify passes"
    no_contracts_exempt    "behavior without contracts never produces W028"
  }

  verify unit "behavior with contracts but no contract/property verify produces W028"
  verify unit "behavior with contracts and contract verify passes"
  verify unit "behavior with contracts and property verify passes"
  verify unit "behavior without contracts never produces W028"

}
