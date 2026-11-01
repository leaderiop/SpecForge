plan 251101-abc "Trim task titles" {
  type           execute
  files_modified ["src/tasks.js"]
  objective      "Titles are trimmed when a task is added"
}

truth TR1 "A padded title is trimmed" {
  plan 251101-abc
  text "A title added with spaces around it is shown without them"
}

artifact AR1 "Task store" {
  plan     251101-abc
  path     "src/tasks.js"
  provides "add() that trims the title"
}

context 251101-abc-CONTEXT "Trim task titles - Context" {
  plan           251101-abc
  gathered       2026-11-01
  boundary       "Titles are trimmed when a task is added"
  canonical_refs [
    { topic "No external specs — requirements fully captured in decisions above" paths [] }
  ]
}

decision 251101-abc-D-01 "Trim both ends" {
  plan    251101-abc
  area    "Trimming"
  text    "Whitespace is trimmed at both ends of a title"
  settled true
  origin  context
}

research 251101-abc-RESEARCH "Trim task titles - Research" {
  plan       251101-abc
  researched 2026-11-01
  confidence high
  summary    "String.prototype.trim covers every whitespace a title can hold"
}

finding F1 "trim is enough" {
  research   251101-abc-RESEARCH
  topic      stack
  text       "String.prototype.trim removes Unicode whitespace too"
  confidence high
}
