// User-facing capabilities. Behaviors point UP to features via `features [...]`.

feature task_management "Task Management" {
  status   in_progress
  priority high
  effort   m
  problem  "Users need to capture, track, and complete their work items."
  solution "Create, complete, and list tasks backed by a durable repository."
  acceptance [
    "A user can create a task with a title",
    "A user can mark a task done",
    "A user can list tasks filtered by status",
  ]
}
