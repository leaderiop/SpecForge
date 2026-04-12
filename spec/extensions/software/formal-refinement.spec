// @specforge/software formal refinement — B-Method

use "types/zero-entity-core"
use "extensions/software/events"
use "extensions/software/types"
use "extensions/software/invariants"
behavior se_parse_abstract_annotation "Parse Abstract Annotation" {
  category command
  features [se_formal_refinement]

  contract """
    Recognize the abstract field with value true on behavior entities.
  """

  ensures {
    abstract_parsed        "abstract true on behavior parsed as specification-only marker"
    empty_verify_ok        "abstract behavior with no verify permitted"
    non_behavior_warned    "abstract on non-behavior entity produces warning"
  }

  verify unit "abstract true on behavior parsed"
  verify unit "abstract behavior with no verify permitted"
  verify unit "abstract on non-behavior produces warning"

}

behavior se_parse_refines_annotation "Parse Refines Annotation" {
  category command
  features [se_formal_refinement]

  contract """
    Recognize the refines field on behavior entities as a reference
    to an abstract behavior.
  """

  ensures {
    edge_created           "refines creates directed edge from concrete to abstract behavior"
    multiple_permitted     "multiple behaviors refining same abstract permitted"
    non_abstract_warned    "refines referencing non-abstract behavior produces warning"
  }

  verify unit "refines referencing abstract behavior parsed"
  verify unit "refines creates directed edge in graph"
  verify unit "refines referencing non-abstract produces warning"
  verify unit "multiple behaviors refining same abstract permitted"

}

behavior se_build_refinement_chain "Build Refinement Chain" {
  category command
  invariants [se_refinement_dag]
  types [RefinementChain]
  features [se_formal_refinement]

  contract """
    Build refinement chains from all refines edges after graph
    construction.
  """

  requires {
    refines_edges_exist    "at least one refines edge exists in the graph"
  }

  ensures {
    chains_built           "each chain starts at abstract behavior, follows refines edges to concretes"
    dag_enforced           "cycles in refinement produce E032"
    depth_recorded         "each chain records its depth (number of refinement levels)"
    deep_chain_warned      "chain depth > 4 produces W031"
  }

  verify unit "refinement chain built from abstract to concrete"
  verify unit "cycle in refinement produces E032"
  verify unit "chain depth recorded correctly"
  verify unit "chain depth > 4 produces W031"

}

behavior se_validate_refinement_completeness "Validate Refinement Completeness" {
  category query
  types [RefinementChain]
  features [se_formal_refinement]

  contract """
    Check that every abstract behavior has at least one concrete
    refinement.
  """

  requires {
    chains_built           "refinement DAG is fully built"
  }

  ensures {
    complete_passes        "abstract with concrete refinement passes"
    incomplete_warned      "abstract with no refinement produces W030"
  }

  verify unit "abstract with concrete refinement passes"
  verify unit "abstract with no refinement produces W030"

}

behavior se_refinement_verify_pass "Refinement Verify Compiler Pass" {
  category command
  types [RefinementChain]
  features [se_formal_refinement]
  produces [se_refinement_check_complete]

  contract """
    The refinement_verify compiler pass validates all refinement chains
    after the contract check pass.
  """

  requires {
    contract_check_done    "contract_check pass has completed"
    chains_built           "refinement DAG is fully built"
  }

  ensures {
    completeness_checked   "every abstract has at least one concrete refinement"
    postconditions_held    "refined behaviors maintain the abstract's postconditions"
    dag_verified           "refinement DAG has no cycles"
    e033_on_mismatch       "behavior not satisfying feature requirements produces E033"
    incomplete_warned      "incomplete refinement chain produces W030 (via completeness check)"
  }

  verify unit "complete refinement chain passes"
  verify unit "behavior not satisfying feature produces E033"
  verify unit "incomplete chain produces W030"
  verify unit "pass runs after contract_check pass"

}
