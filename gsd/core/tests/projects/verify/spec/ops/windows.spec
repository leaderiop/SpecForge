broken_window BW1 "A TODO left in the store" {
  kind        todo
  phase       v1.0/01
  file        "src/tasks.js"
  line        12
  description "A TODO remains in the store"
}

refactor_proposal RP1 "Simplify the store" {
  phase           v1.0/01
  target_file     "src/tasks.js"
  target_function "add"
}
