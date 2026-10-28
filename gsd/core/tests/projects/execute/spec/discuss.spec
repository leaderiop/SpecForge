context 01-CONTEXT "Core context" {
  phase          v1.0/01
  gathered       2026-10-28
  boundary       "Tasks can be added and completed in memory"
  canonical_refs [
    { topic "No external specs: requirements are fully captured in the decisions above" paths [] }
  ]
}

decision D-01 "Tasks are kept in memory" {
  phase  v1.0/01
  area   "Storage"
  text   "Tasks are kept in one in-memory list until phase two saves them"
  origin context
}
