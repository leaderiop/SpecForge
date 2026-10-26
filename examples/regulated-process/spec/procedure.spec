// The steps of the procedure, and the records each one leaves behind.
//
// The kinds come from @specforge/software, but nothing here is code: a
// `behavior` is a step with a contract, an `event` is a record the step
// produces.

use "lifecycle"
use "rules"

behavior review_batch_record "Review the batch record" {
  category   command
  invariants [no_release_with_open_deviation]
  contract   """
    Quality Assurance MUST review the executed batch record against the
    master record, check every in-process result, and list every
    deviation found. The review MUST be signed and dated by the reviewer.
  """
  requires {
    record_complete "the executed batch record has every page and signature"
  }
  ensures {
    deviations_listed "every deviation found is logged against the batch"
    review_signed     "the review is signed and dated by one named reviewer"
  }
  produces   [batch_record_reviewed]
}

behavior close_deviation "Investigate and close a deviation" {
  category   command
  invariants [no_release_with_open_deviation]
  contract   """
    Each deviation MUST have a root cause, an impact assessment on the
    batch and, where needed, a corrective action, before QA closes it.
  """
  requires {
    deviation_logged "the deviation is recorded against the batch"
  }
  ensures {
    impact_assessed "the impact on product quality is documented"
  }
  produces   [deviation_closed]
}

behavior certify_batch "QP certifies the batch" {
  category   command
  invariants [no_release_with_open_deviation, within_registered_specification]
  satisfies  [certified_before_shipped]
  contract   """
    The Qualified Person MUST certify a batch only when the batch record
    review is signed, every deviation is closed and every release test
    is inside the internal limit. Otherwise the QP MUST reject it.
  """
  requires {
    record_reviewed   "the batch record review is signed"
    deviations_closed "no deviation on the batch is open"
    tests_in_limit    "every release test result is inside the internal limit"
  }
  ensures {
    certification_recorded "the certification is signed, dated and attributable to one QP"
  }
  consumes   [batch_record_reviewed, deviation_closed]
  produces   [batch_certified]
}

behavior reject_batch "QP rejects the batch" {
  category  command
  satisfies [every_batch_decided]
  contract  """
    When a batch cannot be certified, the QP MUST reject it with a
    written reason, and the batch MUST stay segregated until destroyed.
  """
  produces  [batch_rejected]
}

behavior ship_batch "Release the batch to market" {
  category   command
  invariants [no_release_without_certification]
  satisfies  [certified_before_shipped, every_batch_decided]
  contract   """
    The warehouse MUST move a batch from quarantine to saleable stock
    only after reading a QP certification on its record.
  """
  consumes   [batch_certified]
  produces   [batch_released]
}

event batch_record_reviewed "Batch record reviewed" {
  category        record
  participates_in [batch_lifecycle]
  sync {
    timeout "5 working days after manufacture"
  }
}

event deviation_closed "Deviation closed" {
  category        record
  participates_in [deviation_handling]
  sync {
    timeout "30 days after the deviation is logged"
  }
}

event batch_certified "Batch certified" {
  category         record
  participates_in  [batch_lifecycle]
  follows_protocol [qa_qp_handover]
  sync {
    timeout "2 working days after the review"
  }
}

event batch_rejected "Batch rejected" {
  category        record
  participates_in [batch_lifecycle]
  sync {
    timeout "2 working days after the review"
  }
}

event batch_released "Batch released" {
  category        record
  participates_in [batch_lifecycle]
  sync {
    timeout "1 working day after certification"
  }
}
