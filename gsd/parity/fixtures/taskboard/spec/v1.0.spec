milestone v1.0 "Taskboard 1.0" {
  name "Taskboard 1.0"
}

phase 01 "Core" {
  milestone v1.0
  name "Core"
  slug "core"
  goal "Tasks can be added and completed"
  requirements [TASK-01, TASK-02]
  success_criteria ["A user can add a task with a title", "A user can mark a task complete by its id"]
}

phase 02 "Persistence" {
  milestone v1.0
  name "Persistence"
  slug "persistence"
  goal "Tasks are saved to disk and read back"
  depends_on [01]
  requirements [STORE-01, STORE-02]
  success_criteria [
    "Every change to the board is written to tasks.json",
    "A restarted board shows the tasks it had",
    "A title comes back exactly as it was written"
  ]
}

phase 02.1 "Hotfix: titles survive a reload" {
  milestone v1.0
  name "Hotfix: titles survive a reload"
  slug "hotfix-titles-survive-a-reload"
  goal "A title is written to disk and read back unchanged"
  depends_on [02]
  success_criteria [
    "A title with a leading space or a line break comes back unchanged",
    "An empty title is refused"
  ]
}

phase 03 "Listing" {
  milestone v1.0
  name "Listing"
  slug "listing"
  goal "Open tasks are listed and filtered"
  depends_on [02]
  requirements [TASK-03, LIST-01]
  success_criteria [
    "taskboard list shows the open tasks, oldest first",
    "taskboard list <word> shows only tasks whose title has the word"
  ]
}
