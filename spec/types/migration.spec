// Migration types — data shapes for spec file format migration

use "types/graph"
// FormatVersion uses major.minor (no patch) because .spec file format changes
// are always intentional and binary-compatible or not. SchemaVersion (in
// types/graph.spec) uses semver with patch for the Graph Protocol's more
// granular compatibility needs.
type FormatVersion {
  major      integer   @readonly
  minor      integer   @readonly
}

type MigrationDiff {
  file_path       string         @readonly
  before_hash     string         @readonly
  after_hash      string         @readonly
  changed_ranges  integer[][]    @optional
  unified_text    string         @optional
}

type MigrationResult {
  file_path       string         @readonly
  source_version  FormatVersion  @readonly
  target_version  FormatVersion  @readonly
  success         boolean
  error           string         @optional
  diff            MigrationDiff  @optional
  backup_path     string         @optional
}

type MigrationBackup {
  original_path   string    @readonly
  backup_path     string    @readonly
  content_hash    string    @readonly
}

type MigrationSummary {
  migrated_count  integer   @readonly
  failed_count    integer   @readonly
  skipped_count   integer   @readonly
  results         MigrationResult[]
  backups         MigrationBackup[]   @optional
}

type PreMigrationSnapshot {
  node_kinds    SchemaEntityKind[]  @readonly
  edge_types    SchemaEdgeType[]    @readonly
  field_defs    SchemaField[]       @readonly
  captured_at   string              @readonly // ISO 8601 timestamp
}
