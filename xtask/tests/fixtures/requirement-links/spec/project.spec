project tracer "Tracer" {
  name        "Tracer"
  what        "A small crate whose tests name the requirements they prove"
  core_value  "A requirement is proven by a test that names it"
  context     "Built to prove the requirement attribute end to end"
  constraints [
    { kind tech_stack what "Rust, no dependencies" why "the workspace never builds it" }
  ]
}

requirement TRACER-01 "Add two numbers" {
  text     "The crate adds two numbers"
  category "Arithmetic"
  tier     v2
}

requirement TRACER-02 "Subtract two numbers" {
  text     "The crate subtracts one number from another"
  category "Arithmetic"
  tier     v2
}

requirement TRACER-03 "Multiply two numbers" {
  text     "The crate multiplies two numbers"
  category "Arithmetic"
  tier     v2
}
