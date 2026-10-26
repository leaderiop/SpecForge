// The roadmap: milestones group phases; deliverables are what each milestone ships.

use "product/phases"
use "product/users"

module links "links" {
  family      core
  description "Shortening, resolving and counting links."
  features    [phase_shorten, phase_redirect, phase_click_counts]
}

milestone v1 "v1: links that work" {
  status        in_progress
  priority      critical
  features      [phase_shorten, phase_redirect]
  modules       [links]
  owner         "tech-lead"
  exit_criteria [
    "Every verify obligation of phases 1 and 2 has a passing test",
    "specforge check reports 0 errors",
  ]
}

milestone v1_1 "v1.1: know your clicks" {
  status        planned
  priority      medium
  features      [phase_click_counts]
  modules       [links]
  depends_on    [v1]
  owner         "tech-lead"
  exit_criteria ["Click counts are verified against recorded redirects"]
}

deliverable shortlink_service "shortlink service" {
  artifact_type service
  status        in_progress
  journeys      [share_a_link]
  modules       [links]
  milestones    [v1, v1_1]
  owner         "tech-lead"
}
