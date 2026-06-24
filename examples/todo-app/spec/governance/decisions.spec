// Governance: decisions, constraints, and risk (a small taste).

use "invariants/task"
use "behaviors/task"

decision sqlite_storage "SQLite for local storage" {
  status accepted
  date   2026-01-15
  context """
    The todo CLI is single-user and local-first. We need durable storage
    without running a server.
  """
  decision  "Use an embedded SQLite database file in the user's config directory."
  consequences [
    "Zero-config persistence",
    "No concurrent multi-writer support (acceptable for single-user CLI)",
  ]
  invariants [task_id_uniqueness]
}

constraint create_latency "Task creation is fast" {
  category    performance
  priority    medium
  description "Creating a task must feel instant on the CLI."
  metric      "create_task p99 < 50ms on local SSD"
  constrains  [create_task]

  verify load "benchmark create_task over 10k iterations, assert p99 < 50ms"
}

failure_mode lost_task "Task acknowledged but not persisted" {
  invariant  task_id_uniqueness
  severity   high
  occurrence rare
  detection  moderate
  cause      "Process killed between in-memory insert and disk flush"
  effect     "User believes a task was saved but it disappears on restart"
  mitigation "Write within a SQLite transaction; flush before acknowledging"
}
