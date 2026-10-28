project cutover "Gate cutover" {
  name        "Gate cutover"
  what        "The twin of the fixtures behind gsd-core's plan-time gate goldens"
  core_value  "A gate verdict is the same in both tools"
  context     "Built to reproduce the gate-cutover verdicts of gsd-core 1.18.0"
  constraints [
    { kind tech_stack what "Node 20" why "the fixtures run there" }
  ]
}

requirement REQ-00 "Typing" {
  text     "The code is strictly typed"
  category "Decisions"
  tier     v1
}

requirement REQ-01 "Sign in" {
  text     "Users can sign in"
  category "Auth"
  tier     v1
}

requirement REQ-02 "Sign out" {
  text     "Users can sign out"
  category "Auth"
  tier     v1
}

requirement REQ-03 "Reset a password" {
  text     "Users can reset a password"
  category "Auth"
  tier     v1
}

requirement WEB-01 "Web tests" {
  text     "The web package is tested"
  category "Web"
  tier     v1
}

requirement WEB-02 "Web lint" {
  text     "The web package is linted"
  category "Web"
  tier     v1
}
