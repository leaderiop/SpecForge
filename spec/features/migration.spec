// Migration feature

use behaviors/migration

feature format_version_migration "Format Version Migration" {
  behaviors [migrate_spec_files_between_versions, detect_format_version_mismatch]

  problem """
    As the .spec format evolves, existing projects need an automated
    upgrade path. Manual migration is error-prone and blocks adoption
    of new compiler features.
  """

  solution """
    specforge migrate transforms .spec files between format versions
    with reversible transformations. The compiler detects version
    mismatches (I003) and suggests migration when newer format
    features are available.
  """
}
