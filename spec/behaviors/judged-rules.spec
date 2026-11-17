requirement FEATURE-058 "Judged Rules" {
  category "Feature"
  tier     v2
  text     """
    Some checks cannot be decided by code: whether a plan covers its requirements, whether a truth holds in the
    code. A model judges them and records its answer nowhere the kernel can see, so an edit made after the judgement
    changes nothing and a status computed from a pass is an agent's claim.

    A rule may be judged: it names a judge, its outcomes and the inputs it reads. The judge's verdict is a `verdict`
    statement in the entity's block that only the host writes, with a digest of every input. The rule holds while
    the verdict is fresh and passing; a changed input makes it stale and the rule fires again.
  """
}
