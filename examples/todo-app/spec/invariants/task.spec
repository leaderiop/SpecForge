// Guarantees the todo-app must never violate.

invariant task_id_uniqueness "Task ID Uniqueness" {
  guarantee "No two tasks may share the same id."
  risk medium

  verify property "concurrent task creation never produces duplicate ids"
}

invariant completed_implies_timestamp "Completed Tasks Have a Timestamp" {
  guarantee """
    A task with status `done` MUST have a non-null completedAt timestamp.
    A task that is not `done` MUST NOT have a completedAt timestamp.
  """
  risk low

  verify unit "completing a task sets completedAt"
  verify unit "reopening a task clears completedAt"
}
