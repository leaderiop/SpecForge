// Inference workflow types — data model for tracking AI agent spec inference progress

type SessionStatus = "active" | "paused" | "completed"

type InferenceManifest "Inference Progress Manifest" {
  version          integer
  source_roots     string[]
  source_index     SourceFileEntry[]    @optional
  sessions         InferenceSession[]   @optional

  verify unit "InferenceManifest round-trips through JSON serialization"
  verify unit "InferenceManifest computes summary from source_index on load"
  verify unit "source_roots supports multiple directories (workspace pattern)"
}

type InferenceSession "Agent Inference Session" {
  session_id       string
  started_at       string
  ended_at         string              @optional
  agent            string
  status           SessionStatus

  verify unit "InferenceSession defaults to active status"
  verify unit "InferenceSession records agent identifier"
}

type SourceFileEntry "Analyzed Source File Record" {
  path             string
  content_hash     string
  entities_produced string[]
  analyzed_at      string

  verify unit "SourceFileEntry stores SHA-256 content hash"
}

type InferenceSummary "Rollup Statistics (computed on load, never persisted)" {
  files_total      integer
  files_analyzed   integer
  entities_produced integer
  needs_review     integer

  verify unit "InferenceSummary is derived from source_index, not stored"
}

type InferencePlan "Prioritized Inference Work Plan" {
  phases           InferencePlanPhase[]

  verify unit "InferencePlan orders kinds by dependency (types before behaviors)"
}

type InferencePlanPhase "Single Kind Phase in Work Plan" {
  kind                  string
  priority              integer
  existing_count        integer
  target_spec_directory string           @optional
  suggested_files       string[]         @optional
}

type InferenceGap "Uncovered Public Item in Source Code" {
  name             string
  item_kind        string
  file             string
  line             integer

  verify unit "InferenceGap carries file and line for actionability"
}

type InferenceGapReport "Per-Directory Inference Gap Analysis" {
  directories      DirectoryGaps[]
  total_items      integer
  total_matched    integer
  approximate      boolean

  verify unit "InferenceGapReport marks results as approximate when using regex fallback"
}

type DirectoryGaps "Gap Analysis for One Source Directory" {
  path             string
  source_items     integer
  matched_entities integer
  gaps             InferenceGap[]       @optional
}
