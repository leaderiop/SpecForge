milestone v1.0 "Gate cutover 1.0" {
  name "Gate cutover 1.0"
}

phase 01 "Decisions" {
  milestone        v1.0
  name             "Decisions"
  slug             "decisions"
  goal             "Decisions are cited by plans"
  requirements     [REQ-00]
  success_criteria ["Every decision is cited", "Every decision is cited too"]
}

phase 02 "Verify" {
  milestone        v1.0
  name             "Verify"
  slug             "verify"
  goal             "Verify commands run in directories that exist"
  depends_on       [01]
  requirements     [WEB-01, WEB-02]
  success_criteria ["The web tests run", "The web tests run too"]
}

phase 03 "Clean" {
  milestone        v1.0
  name             "Clean"
  slug             "clean"
  goal             "A clean verify phase"
  depends_on       [02]
  requirements     [REQ-03]
  success_criteria ["The web tests run", "The web tests run too"]
}

phase 04 "Gap" {
  milestone        v1.0
  name             "Gap"
  slug             "gap"
  goal             "Users can sign in and out"
  depends_on       [03]
  requirements     [REQ-01, REQ-02]
  success_criteria ["Users can sign in", "Users can sign in too"]
}
