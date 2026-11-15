requirement TASK-01 "Add a task" {
  text    "A user can add a task with a title"
  area    "Tasks"
  tier    v1
  defined 2026-03-02
}

requirement TASK-02 "Complete a task" {
  text    "A user can mark a task complete"
  area    "Tasks"
  tier    v1
  defined 2026-03-02
}

requirement TASK-03 "List the open tasks" {
  text    "A user can list the open tasks"
  area    "Tasks"
  tier    v1
  defined 2026-03-02
}

requirement STORE-01 "Save after every change" {
  text    "Tasks are saved to disk after every change"
  area    "Storage"
  tier    v1
  defined 2026-03-02
}

requirement STORE-02 "Titles survive a reload" {
  text    "A task title survives a reload unchanged"
  area    "Storage"
  tier    v1
  defined 2026-03-02
}

requirement LIST-01 "Filter the list" {
  text    "A user can filter the list by a word in the title"
  area    "Listing"
  tier    v1
  defined 2026-03-02
}

requirement SYNC-01 "Keep two machines in step" {
  text    "A board can be kept in step across two machines"
  area    "Sync"
  tier    v2
  defined 2026-03-02
}

requirement SHARE-01 "Share a board by link" {
  text    "A user can share a board by link"
  area    "Out of Scope"
  tier    out_of_scope
  reason  "Needs accounts, which v1 does not have"
  defined 2026-03-02
}
