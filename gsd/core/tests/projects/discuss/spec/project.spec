project taskboard "Taskboard" {
  name          "Taskboard"
  what          "A small board for tracking tasks"
  core_value    "A task is never lost"
  context       "Built to prove the GSD model in SpecForge"
  key_decisions [D-01]
  constraints   [
    { kind tech_stack what "Node 20, no dependencies" why "it ships as one script" }
  ]
}

requirement TASK-01 "Add a task" {
  text     "A user can add a task with a title"
  category "Tasks"
  tier     v1
}

requirement TASK-02 "Complete a task" {
  text     "A user can mark a task complete"
  category "Tasks"
  tier     v1
}

requirement SHARE-01 "Share a board" {
  text     "A user can share a board by link"
  category "Sharing"
  tier     out_of_scope
  reason   "Needs accounts, which v1 does not have"
}
