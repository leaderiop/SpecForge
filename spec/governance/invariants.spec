requirement INVARIANT-111 "Performance Claim Lattice" {
  category "Invariant"
  tier     v2
  text     """
    The toolchain-level performance claims must follow from the declared
    constraint budgets: staying responsive at 2.5x the incremental
    compilation latency budget and within 1.5x the peak memory budget.
    If a budget loosens past a claim, the prove pass reports the claim
    as no longer entailed - making budget drift visible.
  """
}
