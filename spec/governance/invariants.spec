// Governance invariants — formal claims entailed by the declared
// constraint budgets (RES-25 prove pass). The expression groups are
// verified with z3: `specforge analyze --prove` proves these claims from
// the machine-checkable metric bounds in constraints.spec and the
// coverage pass counts them as discharged `verify property` obligations.

invariant performance_claim_lattice "Performance Claim Lattice" {
  guarantee """
    The toolchain-level performance claims must follow from the declared
    constraint budgets: staying responsive at 2.5x the incremental
    compilation latency budget and within 1.5x the peak memory budget.
    If a budget tightens past a claim, the prove pass reports the claim
    as no longer entailed - making budget drift visible.
  """
  risk low

  expression expr {
    file_change_to_diagnostics < 250ms
    peak_memory < 75MB
  }

  verify property "performance claims are entailed by declared constraint bounds"
}
