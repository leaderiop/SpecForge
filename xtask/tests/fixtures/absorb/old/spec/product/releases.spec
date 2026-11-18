// Two releases: each becomes a version milestone, and the milestones it lists phases under it.

use "product/deliverables"
use "product/milestones"

release first_release "First Release" {
  version      "0.1.0"
  description  "The store, usable every day."
  status       planned
  deliverables [store_cli]
  milestones   [second_cut, first_cut]
  target_date  2026-12-01
}

release second_release "Second Release" {
  version     "0.2.0"
  description "Entries are shared."
  changelog   "Entries can be listed by anyone."
  status      planned
  milestones  [third_cut]
}
