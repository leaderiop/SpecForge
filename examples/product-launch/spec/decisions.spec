// Decisions, limits and risks around the launch (@specforge/governance).

use "features"

decision online_first "Open pre-orders online before retail" {
  status           accepted
  date             2026-07-02
  deciders         ["head of growth", "head of sales"]
  context          """
    Retail partners want proof of demand before they order. We have a
    newsletter audience of 40,000 existing customers.
  """
  decision         "Open pre-orders on our own store six weeks before the retail sell-in."
  alternatives     ["Launch in stores and online on the same day"]
  consequences     [
    "Pre-order numbers become the main argument in the sell-in deck",
    "Retail partners get the lamp six weeks after our own customers",
  ]
  affects_features [preorder, shelf_ready_box]
  constraints      [launch_budget]
}

constraint launch_budget "Launch spend cap" {
  category         budget
  priority         high
  description      "Paid media and samples for the launch stay inside the approved budget."
  metric           "total launch spend"
  threshold        "at most 60,000 EUR until 2026-12-31"
  governs_features [preorder, shelf_ready_box]
}

failure_mode stock_short "Pre-orders outrun first stock" {
  severity           high
  occurrence         occasional
  detection          likely
  cause              "The first production run is sized before pre-order demand is known."
  effect             "Customers see their ship week slip and cancel."
  mitigation         "Close pre-orders automatically at 80% of the confirmed first run."
  threatens_features [preorder]
}
