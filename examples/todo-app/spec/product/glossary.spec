// Ubiquitous language — shared definitions for the project.

term task "task" {
  definition """
    A unit of work the user wants to track. Has a title, a status, and
    timestamps. Identified by a unique id.
  """
  aliases  ["to-do", "item"]
  see_also [done]
}

term done "done" {
  definition """
    The terminal status of a completed task. A done task carries a
    completedAt timestamp.
  """
}
