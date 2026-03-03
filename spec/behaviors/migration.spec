// Migration behaviors — format version upgrades

use types/core
use types/codegen
use ports/outbound

behavior migrate_spec_files_between_versions "Migrate Spec Files Between Versions" {
  types      [SpecFile, GeneratedFile]
  ports      [FileSystem]

  contract """
    When specforge migrate --from=X --to=Y is invoked, the system MUST
    transform all .spec files from format version X to format version Y.
    Transformations MUST preserve all entity data. The migration MUST be
    reversible (a migrate back to the original version MUST produce
    equivalent files).
  """

  verify unit        "migration preserves all entities"
  verify unit        "migration updates syntax to new version"
  verify integration "round-trip migration produces equivalent files"
}

behavior detect_format_version_mismatch "Detect Format Version Mismatch" {
  types      [CompilerConfig]

  contract """
    When a project's spec version is older than the compiler version,
    the system MUST emit an I003 info diagnostic suggesting migration.
    The compiler MUST still process older format versions without error.
  """

  verify unit "older version emits I003"
  verify unit "current version does not emit I003"
  verify unit "older version files still compile"
}
