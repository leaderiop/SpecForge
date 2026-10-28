decision D-01 "ES modules, no build step" {
  text      "ES modules, no build step"
  rationale "`node src/cli.js` must just run"
  outcome   pending
  origin    project
}

decision D-02 "Tasks keep an integer id" {
  text      "Tasks keep an integer id"
  rationale "Short to type: `taskboard done 3`"
  outcome   pending
  origin    project
}

context 02-CONTEXT "Persistence context" {
  phase              v1.0/02
  gathered           2026-03-04
  boundary           "Tasks are saved to disk after every change and read back when the board starts. Listing and filtering are phase 3."
  discretion         ["The indent of the JSON file"]
  specifics          "No specific requirements — open to standard approaches"
  canonical_refs     [
    { topic "No external specs — requirements are fully captured in decisions above" paths [] }
  ]
  reusable_assets    ["`src/tasks.js`: the task list and its operations"]
  patterns           ["ES modules, no build step"]
  integration_points ["`src/cli.js` loads the board before each command and saves it after"]
}

decision D-01 "Tasks are kept in a single JSON file" {
  phase  v1.0/02
  area   "Storage format"
  text   "Tasks are kept in a single JSON file, `tasks.json`, in the working directory"
  origin context
}

decision D-02 "The file is written then renamed" {
  phase  v1.0/02
  area   "Storage format"
  text   "The file is written to a temporary name and renamed over `tasks.json`, so a crash never leaves half a file"
  origin context
}

decision D-03 "A title is stored as typed" {
  phase  v1.0/02
  area   "Titles"
  text   "A title is stored exactly as typed, with no trimming"
  origin context
}

deferred_idea DI1 "Keep two machines in step" {
  phase v1.0/02
  text  "Keeping the file in step across two machines"
  why   "v2, SYNC-01"
}

discussion 02-DISCUSSION-LOG "Persistence discussion" {
  phase   v1.0/02
  entries [
    {
      area    "Storage format"
      options ["One JSON file", "SQLite"]
      chosen  "One JSON file"
      mode    interactive
    }
    {
      area    "Titles"
      options ["Stored as typed", "Trimmed"]
      chosen  "Stored as typed"
      mode    interactive
    }
  ]
}

research 02-RESEARCH "Persistence research" {
  phase              v1.0/02
  context_digest     "fnv1a64:4312322ac67d01d2"
  domain             "File-backed storage for a small Node 20 command line tool"
  researched         2026-03-04
  confidence         high
  summary            "A board of a few hundred tasks fits in one JSON file; node:fs reads and writes it, and a write to a temporary name followed by a rename keeps the file whole."
  user_constraints   [D-01, D-02, D-03]
  responsibility_map [
    { capability "Persist the board" tier "Database/Storage" }
  ]
}

source SRC1 "Node 20 fs documentation" {
  research v1.0/02/02-RESEARCH
  url      "https://nodejs.org/docs/latest-v20.x/api/fs.html"
  tier     primary
  checked  "renameSync replaces the target atomically on one filesystem"
}

finding F1 "One JSON file, written then renamed" {
  research   v1.0/02/02-RESEARCH
  topic      stack
  text       "JSON.stringify the board and fs.renameSync a temporary file over tasks.json"
  confidence high
  sources    [SRC1]
}

stack_choice SC1 "node:fs" {
  research v1.0/02/02-RESEARCH
  name     "node:fs"
  version  "20"
  purpose  "Read and write tasks.json"
  tier     core
}

pattern P1 "Write then rename" {
  research v1.0/02/02-RESEARCH
  name     "Write then rename"
  what     "Write the new content to a temporary file in the same directory, then rename it over the target"
  when     "Every save"
}

pattern P2 "Writing in place" {
  research v1.0/02/02-RESEARCH
  name     "Writing in place"
  what     "A crash mid-write leaves a truncated file"
  anti     true
}

pitfall PF1 "Torn writes" {
  research      v1.0/02/02-RESEARCH
  name          "Torn writes"
  goes_wrong    "A crash during writeFileSync leaves a half-written tasks.json"
  why           "writeFileSync truncates the file before it writes"
  avoid         "Write then rename"
  warning_signs "JSON.parse throws on start"
}

dont_hand_roll DH1 "Locking" {
  research v1.0/02/02-RESEARCH
  problem  "Two processes saving at once"
  instead  "One process per board"
  why      "Lock files go stale after a crash"
}

open_question OQ1 "Is one file enough?" {
  research       v1.0/02/02-RESEARCH
  question       "Is one file enough for a thousand tasks?"
  known          "A task is a few dozen bytes"
  unclear        "How large a board grows in practice"
  recommendation "Revisit if a board passes ten thousand tasks"
  resolved_by    D-01
}
