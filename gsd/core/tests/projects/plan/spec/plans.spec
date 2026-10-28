plan 01-01 "Task store" {
  phase          v1.0/01
  type           execute
  files_modified ["src/tasks.js", "test/tasks.test.js"]
  requirements   [TASK-01]
  objective      "Tasks can be added to an in-memory store"
}

task T1 "Store module" {
  plan   v1.0/01/01-01
  type   auto
  files  ["src/tasks.js"]
  action "Write src/tasks.js with add() and list()"
  verify [
    { automated "node --test test/tasks.test.js" fails_when "the test run exits non-zero" }
  ]
  done   "Tasks can be added and listed"
}

task T2 "Store tests" {
  plan   v1.0/01/01-01
  type   tdd
  files  ["test/tasks.test.js"]
  action "Write test/tasks.test.js covering add() and list()"
  verify [
    { automated "node --test test/tasks.test.js" fails_when "a test fails or none runs" }
  ]
  done   "The store is tested"
}

truth TR1 "A user can add a task and see it listed" {
  plan         v1.0/01/01-01
  text         "A user can add a task and see it listed"
  verification explicit
  criterion    1
}

artifact AR1 "Task store" {
  plan     v1.0/01/01-01
  path     "src/tasks.js"
  provides "add() and list()"
}

key_link KL1 "Tests exercise the store" {
  plan v1.0/01/01-01
  from "test/tasks.test.js"
  to   "src/tasks.js"
  via  "require"
}

plan 01-02 "Completion" {
  phase          v1.0/01
  type           execute
  depends_on     [01-01]
  files_modified ["src/complete.js"]
  requirements   [TASK-02]
  objective      "A task can be marked complete"
}

task T1 "Complete a task" {
  plan   v1.0/01/01-02
  type   auto
  files  ["src/complete.js"]
  action "Write src/complete.js with complete(id)"
  verify [
    { automated "node --test test/tasks.test.js" fails_when "the test run exits non-zero" }
    { automated "node src/complete.js --check" fails_when "the script exits non-zero" }
  ]
  done   "A task can be completed"
}

checkpoint C1 "How is completion stored?" {
  plan          v1.0/01/01-02
  after         T1
  kind          decision
  decision      "How is completion stored?"
  options       [
    { name "flag" pros ["simple"] cons ["loses the time"] }
    { name "timestamp" pros ["keeps the time"] cons ["a bigger record"] }
  ]
  resume_signal "Type the option name"
}

truth TR1 "A user can complete a task" {
  plan v1.0/01/01-02
  text "A user can complete a task"
}

artifact AR1 "Completion module" {
  plan     v1.0/01/01-02
  path     "src/complete.js"
  provides "complete(id)"
}

key_link KL1 "Completion uses the store" {
  plan v1.0/01/01-02
  from "src/complete.js"
  to   "src/tasks.js"
  via  "import"
}

plan 02-01 "Save to disk" {
  phase          v1.0/02
  type           execute
  files_modified ["src/store.js"]
  requirements   [STORE-01]
  objective      "Tasks are written to one JSON file"
}

task T1 "Write the file" {
  plan        v1.0/02/02-01
  type        auto
  files       ["src/store.js"]
  action      "Write src/store.js with save() and load()"
  verify_text "npm test"
  done        "Tasks are saved and loaded"
}

truth TR1 "Tasks survive a restart" {
  plan v1.0/02/02-01
  text "Tasks survive a restart"
}

artifact AR1 "Store module" {
  plan     v1.0/02/02-01
  path     "src/store.js"
  provides "save() and load()"
}

key_link KL1 "The store holds the tasks" {
  plan v1.0/02/02-01
  from "src/store.js"
  to   "src/tasks.js"
  via  "import"
}

prohibition PR1 "No partial files" {
  plan         v1.0/02/02-01
  statement    "The store never leaves a half-written file"
  verification judgment
}

plan 261028-a1b "Fix the title trim" {
  type           execute
  files_modified ["src/tasks.js"]
  objective      "A title with spaces around it is trimmed"
}

truth TR1 "A padded title is trimmed" {
  plan 261028-a1b
  text "A padded title is trimmed"
}

artifact AR1 "Task store" {
  plan     261028-a1b
  path     "src/tasks.js"
  provides "add() that trims"
}
