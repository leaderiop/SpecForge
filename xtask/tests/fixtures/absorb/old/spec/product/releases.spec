// One release: no kind holds it yet.

use "product/deliverables"
use "product/milestones"

release first_release "First Release" {
  version      "0.1.0"
  description  "The store, usable every day."
  status       planned
  deliverables [store_cli]
  milestones   [first_cut]
  target_date  2026-12-01
}
