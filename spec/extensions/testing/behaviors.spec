// @specforge/testing behaviors

use "types/zero-entity-core"
use "extensions/testing/types"
use "extensions/testing/invariants"

behavior te_contribute_testability "Contribute Testability to Owned Kinds" {
  category command
  invariants [te_testable_kinds_from_one_table, te_owners_declare_no_test_vocabulary]
  types [TestingVerifyKind]

  contract """
    @specforge/testing MUST contribute testability as entity enhancements
    carrying verify kinds, one per testable kind, each naming the kind's
    owner. Applying such an enhancement marks the target kind testable
    with exactly those verify kinds. An enhancement whose owner the
    project does not use is skipped silently.
  """

  requires {
    kinds_registered "the owning extensions' kinds are registered before enhancements apply"
  }

  ensures {
    kinds_testable   "each table kind is testable with the table's verify kinds"
    owner_optional   "a kind whose owner is not loaded produces no diagnostic"
  }

  verify unit "testing makes software and governance kinds testable with their verify kinds"
  verify unit "an enhancement with verify kinds makes its target kind testable"
}

behavior te_validate_unverified_testable "W004: Unverified Testable Entities" {
  category query
  invariants [te_testable_kinds_from_one_table]
  types [ValidationRulePattern]

  contract """
    Detect entities of a testable software kind that declare no verify
    obligations and no gherkin scenario. Entities marked `abstract true`
    are exempt: their obligations are carried by the concretes that
    refine them. Governance constraints are testable but not required to
    declare obligations.
  """

  ensures {
    unverified_detected "testable entity with no verify and no gherkin produces W004"
    verified_passes     "testable entity with verify produces no diagnostic"
    abstract_exempt     "an abstract entity never produces W004"
  }

  verify unit "testable behavior with no verify produces W004"
  verify unit "testable behavior with verify passes"
}

behavior te_validate_verify_kind_allowlist "W009: Verify Kind Outside the Kind's Allowlist" {
  category query
  invariants [te_testable_kinds_from_one_table]
  types [ValidationRulePattern, TestingVerifyKind]

  contract """
    Detect `verify <kind> "..."` statements whose kind is not among the
    verify kinds @specforge/testing registered for the entity's kind.
    A bare `verify "..."` has no kind and always passes.
  """

  ensures {
    allowed_passes    "verify kind in the allowlist produces no diagnostic"
    disallowed_warned "verify kind outside the allowlist produces W009"
  }

  verify unit "behavior with verify unit passes (unit in allowedVerifyKinds)"
  verify unit "invariant with verify load produces W009 (load not in allowedVerifyKinds)"
  verify unit "W009 message includes allowed set"
}

behavior te_coverage_pass "Coverage Analysis Pass" {
  category query
  invariants [te_testable_kinds_from_one_table]
  types [TestingVerifyKind]

  contract """
    @specforge/testing's `coverage` compiler pass (run by `specforge
    analyze coverage`, or `analyze` with every pass) MUST score the
    project at three layers. Intent: a testable entity that declares no
    verify obligations is A001, and an invariant with none is A002, an
    error when its risk is high. Enforcement: invariants nothing
    references are counted in the summary (the finding is software's
    W003 from `specforge check`, not repeated here). Proof, from the recorded
    tests (specforge-report.json, which `analyze` reads by default after
    `specforge collect`): a test proves an obligation by naming its text.
    An obligation no passing test names is A015, a test naming an
    obligation its entity doesn't declare is A016, and a failing test is
    A014. An entity is proven when it has tests, all of them pass, and
    every obligation is proven. A formal claim the prove pass entailed
    discharges `verify property` obligations without executable tests.
    The summary MUST report the discharge funnel (entities with
    obligations, proven, formally discharged, report failures).
  """

  ensures {
    intent_scored      "entities without obligations are A001, invariants A002"
    orphans_counted    "unreferenced invariants are counted, not re-reported"
    proof_recorded     "recorded passing tests prove an entity; a failing one is A014"
    obligations_proven "each obligation needs a passing test that names it (A015); unknown names are A016"
    formal_discharge   "entailed formal claims discharge verify property obligations"
  }

  verify unit "a high-risk invariant without obligations is an A002 error"
  verify unit "invariant references count as enforcement"
  verify unit "recorded test results prove entities and failing tests are A014"
  verify unit "an obligation no passing test names is A015 and a test naming an undeclared obligation is A016"
  verify unit "a proved formal claim discharges verify property obligations"
}

behavior te_coverage_gate "Proof Coverage Gate" {
  category validation
  types [TestingVerifyKind]

  contract """
    `specforge analyze coverage --min N` MUST exit non-zero (E048) when
    the share of testable entities the coverage pass proved is below N
    percent, after printing the full analysis. It needs test results
    (the project's specforge-report.json or --test-results) and the
    coverage pass, and a project with nothing testable satisfies any
    threshold.
  """

  ensures {
    below_fails  "proof coverage below the threshold fails with E048"
    above_passes "proof coverage at or above the threshold passes"
    needs_results "the gate refuses to run without test results"
  }

  verify unit "coverage at or above the threshold passes"
  verify unit "coverage below the threshold fails with E048"
  verify unit "the gate needs test results"
}

