// @specforge/software analyze commands — CLI commands for formal analysis

use types/zero-entity-core
use extensions/software/types

behavior se_analyze_contracts "Analyze Contracts Command" {
  category command
  types [RequiresBlock, EnsuresBlock, ContractCondition]

  contract """
    The specforge analyze contracts command MUST run the contract_check
    pass in isolation and report results. Output MUST include: count
    of behaviors with contracts, precondition satisfiability results,
    postcondition reachability results, Liskov compliance results.
  """

  requires {
    graph_available        "compiled entity graph is available"
  }

  ensures {
    contract_report        "report shows contract analysis for all behaviors with requires/ensures"
    json_output            "with --json flag, output is machine-readable JSON"
    strict_exit_code       "with --strict flag, any violation exits non-zero"
  }

  verify unit "analyze contracts reports all behaviors with requires/ensures"
  verify unit "analyze contracts --json produces JSON output"
  verify unit "analyze contracts --strict exits non-zero on violation"

}

behavior se_analyze_refinement "Analyze Refinement Command" {
  category command
  types [RefinementChain]

  contract """
    The specforge analyze refinement command MUST run the
    refinement_verify pass in isolation and report results.
  """

  ensures {
    refinement_report      "report shows all refinement chains with completeness status"
    json_output            "with --json flag, output is machine-readable JSON"
  }

  verify unit "analyze refinement reports all abstract/concrete chains"
  verify unit "analyze refinement --json produces JSON output"

}

behavior se_analyze_concurrency "Analyze Concurrency Command" {
  category command
  types [SoftwareEvent, SyncBlock]

  contract """
    The specforge analyze concurrency command MUST run the
    process_analyze pass in isolation and report results.
  """

  ensures {
    concurrency_report     "report shows deadlocks, livelocks, unmatched producers, channel mismatches"
    json_output            "with --json flag, output is machine-readable JSON"
  }

  verify unit "analyze concurrency reports deadlocks and livelocks"
  verify unit "analyze concurrency --json produces JSON output"

}

behavior se_analyze_all "Analyze All Command" {
  category command

  contract """
    The specforge analyze all command MUST run contract_check,
    refinement_verify, and process_analyze in pipeline order and
    produce a unified report.
  """

  requires {
    graph_available        "compiled entity graph is available"
  }

  ensures {
    unified_report         "report combines contract, refinement, and concurrency analysis"
    json_output            "with --json flag, output is machine-readable JSON"
    strict_exit_code       "with --strict flag, any violation exits non-zero (for CI)"
  }

  verify unit "analyze all runs all three passes in pipeline order"
  verify unit "analyze all --strict exits non-zero on any violation"
  verify unit "analyze all --json produces unified JSON report"

}
