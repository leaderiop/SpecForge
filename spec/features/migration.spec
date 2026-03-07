// Migration features — spec file format version migration

use behaviors/migration

feature spec_file_migration "Spec File Migration" {
  // MCP: provide_mcp_migrate_tool in features/mcp.spec::mcp_mutation_tools
  behaviors [detect_format_version_mismatch, migrate_spec_files_in_place, generate_migration_diff, validate_post_migration_integrity, verify_graph_protocol_compatibility_after_migration, invoke_extension_migration_hooks, rollback_failed_migration]

  problem """
    As the .spec file format evolves across versions, existing projects
    accumulate files in older formats. There is no automated way to detect
    version mismatches, preview changes, or migrate files to the current
    format while ensuring structural integrity is preserved.
  """

  solution """
    specforge migrate detects format version mismatches, transforms .spec
    files to the current version with automatic backups, supports --dry-run
    for previewing changes as POSIX unified diffs, and validates post-migration
    integrity by comparing pre and post graph structures. A pre-migration
    snapshot is captured before any transforms to enable structural comparison.
    Individual file failures do not block the migration of other files; files
    already at the target version are skipped (idempotency). Extension schema
    migration (when extension manifests evolve) is delegated to extension
    authors via Wasm migration hooks declared in ManifestV2.migration_hook.
    Post-migration validation runs once after both core and extension hooks
    complete. Core migration covers only .spec file format syntax evolution.
    See also: graph_protocol_versioning in features/output.spec for the Graph
    Protocol versioning link.
  """
}
