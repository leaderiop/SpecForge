// What ships, grouped into workstreams, checkpoints and one launch release.

use "features"
use "journeys"

module marketing "Marketing workstream" {
  description "The story, the store page and the launch emails."
  family      workstream
  features    [all_day_light, preorder]
}

module retail "Retail workstream" {
  description "Packaging and everything a retail partner needs to say yes."
  family      workstream
  features    [shelf_ready_box, one_dial]
}

milestone preorders_open "Pre-orders open" {
  status        completed
  priority      critical
  target_date   2026-09-15
  features      [preorder, all_day_light]
  modules       [marketing]
  exit_criteria ["Store page live", "Ship week confirmed by logistics"]
  owner         "growth"
}

milestone retail_sell_in "Retail sell-in" {
  status        in_progress
  priority      high
  start_date    2026-09-15
  target_date   2026-11-01
  features      [shelf_ready_box, one_dial]
  modules       [retail]
  depends_on    [preorders_open]
  exit_criteria ["Two partners have placed a first order"]
  owner         "sales"
}

deliverable store_page "Store page and pre-order flow" {
  artifact_type web_app
  status        shipped
  journeys      [discover_and_preorder]
  modules       [marketing]
  milestones    [preorders_open]
  owner         "growth"
}

deliverable press_kit "Press kit" {
  artifact_type media
  status        shipped
  description   "Photos, the day-to-evening clip and a one-page fact sheet."
  journeys      [discover_and_preorder]
  modules       [marketing]
  milestones    [preorders_open]
  owner         "marketing"
}

deliverable sell_in_deck "Retail sell-in deck" {
  artifact_type sales_deck
  status        in_progress
  journeys      [stock_the_shelf]
  modules       [retail]
  milestones    [retail_sell_in]
  depends_on    [press_kit]
  owner         "sales"
}

release launch "Halo launch" {
  version      "1.0.0"
  status       in_progress
  description  "Everything that has to be live on launch day."
  deliverables [store_page, press_kit, sell_in_deck]
  milestones   [preorders_open, retail_sell_in]
  target_date  2026-11-15
  owner        "launch-team"
}
