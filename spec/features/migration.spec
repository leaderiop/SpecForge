requirement FEATURE-122 "Spec File Migration" {
  category "Feature"
  tier     v2
  text     """
    As the .spec file format evolves across versions, existing projects
    accumulate files in older formats. There is no automated way to detect
    version mismatches, preview changes, or migrate files to the current
    format while ensuring structural integrity is preserved.

    specforge migrate detects format version mismatches, transforms .spec
    files to the current version with automatic backups, supports --dry-run
    for previewing changes as POSIX unified diffs, and validates post-migration
    integrity by comparing pre and post graph structures. A pre-migration
    snapshot is captured before any transforms to enable structural comparison.
    A file that cannot be migrated leaves every file as it was, and the report
    names each such file; files
    already at the target version are skipped (idempotency). Extension schema
    migration (when extension manifests evolve) is delegated to extension
    authors via Wasm migration hooks declared in ExtensionDeclaration.migration_hook.
    Post-migration validation runs once after both core and extension hooks
    complete. Core migration covers only .spec file format syntax evolution.
    See also: graph_protocol_versioning in features/output.spec for the Graph
    Protocol versioning link.
  """
}
