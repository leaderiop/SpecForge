project taskboard "Taskboard" {
  name          "Taskboard"
  what          "A small board for tracking tasks from the command line"
  core_value    "A task is never lost"
  context       "Built to prove the GSD model in SpecForge"
  key_decisions [D-01, D-02]
  constraints   [
    { kind tech_stack what "Node 20, no dependencies" why "it ships as one script" }
  ]
}
