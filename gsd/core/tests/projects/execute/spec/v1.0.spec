milestone v1.0 "Taskboard 1.0" {
  name "Taskboard 1.0"
}

phase 01 "Core" {
  milestone        v1.0
  name             "Core"
  slug             "core"
  goal             "Tasks can be added and completed"
  requirements     [TASK-01, TASK-02]
  success_criteria ["A task can be added", "A task can be completed"]
}

phase 02 "Persistence" {
  milestone        v1.0
  name             "Persistence"
  slug             "persistence"
  goal             "Tasks are saved to disk and loaded on start"
  depends_on       [01]
  requirements     [STORE-01]
  success_criteria ["Tasks are saved", "Tasks are loaded on start"]
}
