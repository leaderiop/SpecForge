// Four decisions: accepted, proposed, deprecated, and superseded with and without a successor.

decision one_parser "One Parser" {
  status       accepted
  date         2026-03-01
  context      "Two readers of one format drift apart."
  decision     "Read the format with one parser."
  consequences ["One grammar to keep", "A hand parser is refused in review"]
  alternatives ["A hand parser", "A parser combinator"]
  tags         ["parsing", "format"]
  reason       "Drift cost more than the parser."
  deciders     ["Alice Martin"]
}

decision lazy_cache "Lazy Cache" {
  status   proposed
  context  "Cold runs read every file."
  decision "Cache the parse of each file until it changes."
}

decision sync_writes "Synchronous Writes" {
  status   deprecated
  context  "Writes raced the watcher."
  decision "Write every file synchronously."
}

decision first_store "First Store" {
  status        superseded
  context       "Facts needed a home."
  decision      "Keep facts in one JSON file."
  superseded_by second_store
}

decision second_store "Second Store" {
  status   accepted
  context  "One JSON file grew past a megabyte."
  decision "Keep one fact store per collector."
}

decision old_format "Old Format" {
  status   superseded
  context  "The first format had no versions."
  decision "Write the format without a version line."
}
