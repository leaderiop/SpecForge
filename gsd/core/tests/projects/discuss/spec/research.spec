research 02-RESEARCH "Persistence research" {
  phase              v1.0/02
  domain             "File-backed storage for a small Node tool"
  researched         2026-10-28
  confidence         high
  summary            "One JSON file, written atomically, is enough"
  user_constraints   [D-01]
  responsibility_map [
    { capability "Persist tasks" tier "storage" }
  ]
}

source SRC1 "Node fs docs" {
  research v1.0/02/02-RESEARCH
  url      "https://nodejs.org/api/fs.html"
  tier     primary
  checked  "2026-10-28"
}

finding F1 "JSON is enough" {
  research   v1.0/02/02-RESEARCH
  topic      stack
  text       "A single JSON file holds the tasks of a small board"
  confidence high
  sources    [SRC1]
}

stack_choice SC1 "node:fs" {
  research v1.0/02/02-RESEARCH
  name     "node:fs"
  purpose  "Read and write the task file"
  tier     core
}

pattern P1 "Write then rename" {
  research v1.0/02/02-RESEARCH
  name     "Write then rename"
  what     "Write a temp file, then rename it over the target"
  when     "Every save"
}

pitfall PF1 "Torn writes" {
  research   v1.0/02/02-RESEARCH
  name       "Torn writes"
  goes_wrong "A crash mid-write leaves a half file"
  why        "writeFile truncates before it writes"
  avoid      "Write then rename"
}

dont_hand_roll DH1 "File locking" {
  research v1.0/02/02-RESEARCH
  problem  "Two processes saving at once"
  instead  "Use one process per board"
  why      "Locks are platform-specific"
}

open_question OQ1 "Is one file enough?" {
  research    v1.0/02/02-RESEARCH
  question    "Is one file enough for a thousand tasks?"
  unclear     "How large a board gets"
  resolved_by D-02
}

package_audit PA1 "node:fs" {
  research   v1.0/02/02-RESEARCH
  package    "node:fs"
  registry   "builtin"
  legitimacy legitimate
}

research RESEARCH "Project research" {
  confidence medium
  summary    "Node tools of this size need no framework"
}

finding F1 "No framework needed" {
  research   RESEARCH
  topic      stack
  text       "A framework adds nothing at this size"
  confidence medium
}
