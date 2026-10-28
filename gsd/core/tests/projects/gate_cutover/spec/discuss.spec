context 01-CONTEXT "Decisions context" {
  phase          v1.0/01
  gathered       2026-10-28
  boundary       "Decisions are cited by plans"
  canonical_refs [
    { topic "No external specs" paths [] }
  ]
}

decision D-01 "Adopt strict typing" {
  phase  v1.0/01
  area   "Typing"
  text   "Adopt strict typing"
  origin context
}

decision D-02 "Cache tokens in memory" {
  phase  v1.0/01
  area   "Tokens"
  text   "Cache tokens in memory"
  origin context
}

decision D-03 "Log every retry" {
  phase  v1.0/01
  area   "Retries"
  text   "Log every retry"
  origin context
}

context 04-CONTEXT "Gap context" {
  phase          v1.0/04
  gathered       2026-10-28
  boundary       "Users can sign in and out"
  canonical_refs [
    { topic "No external specs" paths [] }
  ]
}

decision D-01 "Adopt strict typing" {
  phase  v1.0/04
  area   "Typing"
  text   "Adopt strict typing"
  origin context
}
