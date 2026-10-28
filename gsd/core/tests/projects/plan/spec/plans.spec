plan 01-01 "Task store" {
  phase          v1.0/01
  type           execute
  files_modified ["src/tasks.js", "test/tasks.test.js"]
  requirements   [TASK-01]
  objective      "Tasks can be added to an in-memory store"
}

plan 01-02 "Completion" {
  phase          v1.0/01
  type           execute
  depends_on     [01-01]
  files_modified ["src/complete.js"]
  requirements   [TASK-02]
  objective      "A task can be marked complete"
}

plan 02-01 "Save to disk" {
  phase          v1.0/02
  type           execute
  files_modified ["src/store.js"]
  requirements   [STORE-01]
  objective      "Tasks are written to one JSON file"
}

plan 261028-a1b "Fix the title trim" {
  type           execute
  files_modified ["src/tasks.js"]
  objective      "A title with spaces around it is trimmed"
}
