// Why the procedure is shaped this way, and what can go wrong (@specforge/governance).

use "procedure"
use "rules"

decision internal_limit_tighter "Release at a tighter internal limit" {
  status       accepted
  date         2026-03-10
  deciders     ["Head of Quality", "Qualified Person"]
  context      """
    Assay results vary between the release test and the stability tests
    run later. A batch released right at the registered limit could fall
    outside it before its expiry date.
  """
  decision     "Release only batches with an assay of at least 98%, three points above the registered 95%."
  consequences [
    "Some batches inside the registered limit are rejected",
    "A drift in the internal limit past the registered one shows up as an unproven claim",
  ]
  invariants   [within_registered_specification]
  constraints  [internal_release_limit]
}

decision two_person_release "QA review and QP certification are separate people" {
  status     accepted
  date       2026-03-10
  context    "One person reviewing and certifying the same batch removes the second check."
  decision   "The QA reviewer of a batch record may not certify the same batch."
  invariants [no_release_without_certification]
}

failure_mode shipped_in_quarantine "Batch shipped while still in quarantine" {
  invariant          no_release_without_certification
  severity           critical
  occurrence         rare
  detection          moderate
  cause              "Stock status set to saleable by hand before the certification is recorded."
  effect             "An uncertified batch reaches patients; a recall is needed."
  mitigation         "The warehouse reads the certification from the batch record, never a manual flag."
  affected_behaviors [ship_batch]
}

failure_mode deviation_missed "Deviation not logged on the batch" {
  invariant          no_release_with_open_deviation
  severity           high
  occurrence         occasional
  detection          unlikely
  cause              "A deviation is noted on paper on the shop floor but never entered against the batch."
  effect             "The QP certifies a batch without knowing about the deviation."
  mitigation         "The batch record review checks the shop-floor logbook line by line."
  affected_behaviors [review_batch_record, certify_batch]
}
