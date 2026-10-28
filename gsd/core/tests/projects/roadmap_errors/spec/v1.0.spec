project errors "Roadmap errors" {
  name        "Roadmap errors"
  what        "A roadmap that breaks the roadmap rules"
  core_value  "Every rule fires once"
  context     "A fixture"
  constraints [
    { kind other what "none" why "a fixture" }
  ]
}

requirement LOOSE-01 "In no phase" {
  text     "Mapped nowhere"
  category "Fixture"
  tier     v1
}

requirement TWICE-01 "In two phases" {
  text     "Mapped twice"
  category "Fixture"
  tier     v1
}

requirement GONE-01 "Out of scope, no reason" {
  text     "Out of scope without a reason"
  category "Fixture"
  tier     out_of_scope
}

milestone v1.0 "Errors 1.0" {
  name "Errors 1.0"
}

phase 01 "First" {
  milestone        v1.0
  name             "First"
  slug             "first"
  goal             "First"
  requirements     [TWICE-01]
  success_criteria ["a", "b"]
}

phase 02 "Second" {
  milestone        v1.0
  name             "Second"
  slug             "second"
  goal             "Second"
  requirements     [TWICE-01]
  success_criteria ["a", "b"]
}

phase 04 "Fourth" {
  milestone        v1.0
  name             "Fourth"
  slug             "fourth"
  goal             "Fourth"
  depends_on       [05]
  success_criteria ["a", "b"]
}

phase 05 "Fifth" {
  milestone        v1.0
  name             "Fifth"
  slug             "fifth"
  goal             "Fifth"
  depends_on       [04]
  success_criteria ["a", "b"]
}
