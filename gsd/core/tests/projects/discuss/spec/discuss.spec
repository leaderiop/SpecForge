decision D-01 "ES modules, no build step" {
  text   "The code is ES modules with no build step"
  origin project
}

context 02-CONTEXT "Persistence context" {
  phase          v1.0/02
  gathered       2026-10-28
  boundary       "Tasks are saved to disk and loaded on start"
  canonical_refs [
    { topic "Storage" paths ["docs/storage.md"] }
  ]
}

decision D-01 "Store tasks in one JSON file" {
  phase  v1.0/02
  area   "Storage"
  text   "Tasks are kept in a single JSON file"
  origin context
}

decision D-02 "Write the file atomically" {
  phase         v1.0/02
  area          "Storage"
  text          "The file is written to a temp name, then renamed"
  origin        context
  superseded_by D-01
}

deferred_idea DI1 "Sync between machines" {
  phase v1.0/02
  text  "Tasks sync between machines"
  why   "Needs accounts, which v1 does not have"
}
