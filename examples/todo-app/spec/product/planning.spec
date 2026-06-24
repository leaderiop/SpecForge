// Code organization, planning phases, and shippable artifacts.

use "features/task_management"
use "product/journeys"

module core "todo-core" {
  family      core
  description "Domain logic: tasks, repository, behaviors."
  features    [task_management]
}

milestone mvp "Minimum Viable Product" {
  status        in_progress
  priority      high
  features      [task_management]
  modules       [core]
  exit_criteria [
    "create / complete / list behaviors verified",
    "zero E-level diagnostics",
  ]
}

deliverable todo_cli "todo CLI" {
  artifact_type cli
  status        in_progress
  journeys      [capture_and_complete]
  modules       [core]
  milestones    [mvp]
}
