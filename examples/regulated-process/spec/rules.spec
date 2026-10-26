// The rules no batch may break, and the limits the quality lab works to.

invariant no_release_without_certification "No release without QP certification" {
  guarantee  "A batch reaches the market only after a Qualified Person has certified it."
  risk       critical
  expression expr {
    uncertified_batches_released < 1
  }
}

invariant no_release_with_open_deviation "No release with an open deviation" {
  guarantee  "A batch with an open deviation stays in quarantine until the deviation is closed."
  risk       critical
  expression expr {
    open_deviations_at_release < 1
  }
}

invariant within_registered_specification "Released batches meet the registered specification" {
  guarantee   "Every released batch has an assay result inside the limit registered with the authority."
  risk        critical
  description "Follows from the tighter internal release limit below."
  expression  expr {
    assay_purity >= 95.0
  }
}

constraint internal_release_limit "Internal release limit" {
  category    quality
  priority    critical
  description "The lab releases only batches above an internal limit set tighter than the registered one."
  metric      expr {
    assay_purity >= 98.0
  }
  protects    [within_registered_specification]
}

constraint deviation_gate "Deviations closed before certification" {
  category    quality
  priority    critical
  description "Certification is refused while any deviation on the batch is open."
  metric      expr {
    open_deviations_at_release <= 0
  }
  protects    [no_release_with_open_deviation]
}

constraint certification_gate "Certification before shipment" {
  category    quality
  priority    critical
  description "The warehouse ships only batches whose record carries a QP certification."
  metric      expr {
    uncertified_batches_released <= 0
  }
  protects    [no_release_without_certification]
}

property certified_before_shipped "Certification always precedes shipment" {
  property_type safety
  description   "Across the whole lifecycle, no path ships a batch that was not certified first."
  expression    "batch_released never happens before batch_certified for the same batch"
  scope         "batch lifecycle"
}

property every_batch_decided "Every batch is eventually decided" {
  property_type liveness
  description   "No batch waits in quarantine forever: it is released or rejected."
  expression    "every batch in quarantine eventually reaches released or rejected"
  scope         "batch lifecycle"
}
