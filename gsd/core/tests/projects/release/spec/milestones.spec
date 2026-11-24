milestone v1.7 "Addition" {
  completion {
    at 2026-10-01
  }
}

milestone v1.8 "Trim" {
}

phase 01 "Drop subtraction" {
  milestone        v1.8
  slug             "drop-subtraction"
  goal             "The crate keeps addition only"
  success_criteria ["`ops::add` adds", "`ops::sub` is gone"]
}
