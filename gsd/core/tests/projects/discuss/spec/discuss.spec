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

phase_spec 02-SPEC "Persistence spec" {
  phase   v1.0/02
  created 2026-10-28
  goal    "Tasks go from memory-only to saved on disk"
}

locked_req LR1 "Tasks survive a restart" {
  spec       v1.0/02/02-SPEC
  title      "Tasks survive a restart"
  current    "Tasks are lost when the process ends"
  target     "Tasks are loaded from disk on start"
  acceptance "Restarting the process shows the same tasks"
}

interview IV1 "Where are tasks kept?" {
  spec        v1.0/02/02-SPEC
  round       1
  perspective researcher
  question    "Where are tasks kept?"
  answer      "In one JSON file"
}

interview IV2 "What if the file is corrupt?" {
  spec        v1.0/02/02-SPEC
  round       1
  perspective failure_analyst
  question    "What if the file is corrupt?"
}
