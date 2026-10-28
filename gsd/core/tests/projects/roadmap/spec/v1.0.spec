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

phase 01.1 "Hotfix" {
  milestone        v1.0
  name             "Hotfix"
  slug             "hotfix"
  goal             "Titles survive a reload"
  depends_on       [01]
  success_criteria ["A title survives a reload", "An empty title is refused"]
}
