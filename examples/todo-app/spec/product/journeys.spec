// UX flows: a persona on a channel exercising features.

use "features/task_management"
use "product/personas"
use "product/channels"

journey capture_and_complete "Capture and complete a task" {
  persona  individual
  channels [cli]
  features [task_management]
  priority high

  flow [
    "User runs `todo add \"Buy milk\"`",
    "System creates the task and prints its id",
    "User runs `todo done <id>`",
    "System marks the task done and confirms",
    "User runs `todo list --status done` and sees the task",
  ]
}
