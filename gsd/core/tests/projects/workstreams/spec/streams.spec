project taskboard "Taskboard" {
  name        "Taskboard"
  what        "A small board for tracking tasks"
  core_value  "A task is never lost"
  context     "Two workstreams that both touch the store"
  constraints [
    { kind tech_stack what "Node 20, no dependencies" why "it ships as one script" }
  ]
}

requirement TASK-01 "Add a task" {
  text     "A user can add a task with a title"
  category "Tasks"
  tier     v1
}

workstream auth "Auth" {
  purpose "Accounts for the board"
}

workstream store "Store" {
  purpose "Persist the board"
}

milestone v1.0 "Auth 1.0" {
  name       "Auth 1.0"
  workstream auth
}

milestone v1.0 "Store 1.0" {
  name       "Store 1.0"
  workstream store
}

phase 01 "Login" {
  milestone        auth/v1.0
  name             "Login"
  slug             "login"
  goal             "A user can sign in"
  requirements     [TASK-01]
  success_criteria ["A user can sign in", "A wrong password is refused"]
}

phase 01 "Files" {
  milestone        store/v1.0
  name             "Files"
  slug             "files"
  goal             "The board is saved"
  success_criteria ["The board is saved", "The board is loaded"]
}

plan 01-01 "Sign in" {
  phase          auth/v1.0/01
  type           execute
  files_modified ["src/tasks.js", "src/auth.js"]
  requirements   [TASK-01]
  objective      "A user can sign in"
}

plan 01-01 "Save" {
  phase          store/v1.0/01
  type           execute
  files_modified [" src/tasks.js", "src/store.js"]
  objective      "The board is saved"
}
