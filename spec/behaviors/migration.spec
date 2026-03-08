// Migration behaviors — spec file format version migration

use invariants/core
use invariants/migration
use invariants/validation
use invariants/wasm
use invariants/zero-entity-core
use types/core
use types/migration
use types/diagnostics
use types/graph
use types/wasm
use types/zero-entity-core
use ports/inbound
use ports/outbound
use events/compilation

behavior detect_format_version_mismatch "Detect Format Version Mismatch" {
  invariants [multi_error_collection, zero_domain_knowledge_core, diagnostic_determinism]
  types      [SpecFile, FormatVersion]

  requires {
    spec_file_available "A .spec file is being parsed and its content is accessible"
  }

  ensures {
    version_mismatch_reported "I007 info diagnostic emitted when detected version differs from expected version"
    unsupported_version_rejected "E015 diagnostic emitted for format versions outside the supported range"
    parsing_continues "Parsing proceeds with best-effort compatibility regardless of version mismatch"
  }

  contract """
    During parsing, the system MUST detect when a .spec file uses an older
    format version than the compiler expects. The detected version and the
    expected version MUST be reported as an I007 info diagnostic. The system
    MUST continue parsing with best-effort compatibility. Files without an
    explicit format version MUST be treated as the oldest supported version.

    Format version MAY be declared in two ways: a header comment
    `// specforge-format: <major>` at the top of a .spec file (before any
    other content), or a `format_version` field inside the spec root block.
    The `format_version` field is a string of the form `"<major>.<minor>"`
    that declares which .spec file format the file was written against; it
    is distinct from the Graph Protocol's SchemaVersion (types/graph.spec).
    If both header comment and root field are present, they MUST agree — a
    mismatch MUST produce an E-level diagnostic. The header comment form is
    RECOMMENDED for brevity.

    The compiler MUST support the current and previous major format versions.
    Files declaring an unsupported format version (older than previous major
    or newer than current) MUST produce an E015 diagnostic with upgrade
    guidance indicating which compiler version supports that format.
  """

  verify unit "older format version detected and reported as I007"
  verify unit "current format version produces no diagnostic"
  verify unit "missing format version treated as oldest supported"
  verify unit "header comment format version detected correctly"
  verify unit "spec root format_version field detected correctly"
  verify unit "mismatched header and root format_version produces E-level diagnostic"
  verify unit "unsupported format version produces E015 with upgrade guidance"
  verify contract "requires/ensures consistency for format version detection"

}

// Transform chaining strategy: when migrating across multiple format versions
// (e.g., v1 → v3), transforms are applied sequentially (v1→v2, v2→v3), not
// directly (v1→v3). Each step is a self-contained transform function. This
// ensures that each version boundary is validated independently.
behavior migrate_spec_files_in_place "Migrate Spec Files In Place" {
  invariants [multi_error_collection, migration_idempotency, migration_backup_safety, migration_atomicity, migration_semantic_preservation, zero_domain_knowledge_core]
  types      [SpecFile, MigrationResult, MigrationSummary, MigrationBackup]
  ports      [CompilerApi, FileSystem]
  produces   [migration_starting, migration_started, migration_complete]

  requires {
    files_exist           "All .spec files targeted for migration exist on disk and are readable"
    valid_target_version  "Target version is a valid format version"
  }

  ensures {
    files_at_target       "All successfully migrated files are at the target version"
    summary_emitted       "migration_summary diagnostic emitted with accurate counts"
    complete_event_fired  "migration_complete event fires after all files are processed"
  }

  maintains {
    semantic_preservation "Migrated files preserve all entity IDs, references, and field values from the source version"
  }

  // Temporal distinction between migration events:
  //   migration_starting — fires before any file I/O begins (intent signal).
  //     Consumers use this to capture pre-migration snapshots while the
  //     filesystem is still untouched.
  //   migration_started — fires after the backup is created but before
  //     transforms run (point of no return). At this point, backups exist
  //     and the system is committed to attempting the migration.

  contract """
    When specforge migrate is invoked, the system MUST first capture a
    pre-migration snapshot of all .spec files (content hash per file) and
    the current graph state before any transforms are applied. The system
    MUST then emit migration_starting (consumed by
    capture_pre_migration_schema_snapshot for schema snapshot)
    before beginning transforms.

    The system MUST transform .spec files from the detected source version
    to the target version. Before modifying any file, the system MUST
    create a backup copy with a .bak extension unless --no-backup is
    specified. Each file MUST be migrated independently — a failure in one
    file MUST NOT prevent migration of others. Each file MUST be written
    atomically (write to temporary file, then rename) to prevent corruption
    from interrupted migrations.

    The system MUST report the number of files migrated, failed, and
    skipped via MigrationSummary. Skipped files are those already at the
    target version — this ensures migration idempotency: running migrate
    on an already-current project produces zero modifications.

    The FileSystem port is required for reading source files, writing
    backups, writing temporary files, and performing atomic renames.
  """

  verify contract "requires/ensures consistency for in-place migration"
  verify unit "files migrated from source to target version"
  verify unit "backup created before modification"
  verify unit "--no-backup skips backup creation"
  verify unit "failure in one file does not block others"
  verify unit "summary reports migrated, failed, and skipped counts"
  verify unit "interrupted migration leaves no partially written files"
  verify unit "pre-migration snapshot captured before migration_starting event"
  verify unit "files already at target version are skipped with skippedCount incremented"

}

behavior generate_migration_diff "Generate Migration Diff" {
  invariants [diagnostic_determinism, migration_idempotency, dry_run_side_effect_freedom, zero_domain_knowledge_core]
  types      [SpecFile, MigrationResult, MigrationDiff]
  ports      [CompilerApi]
  produces   [migration_diff_generated]

  requires {
    spec_files_available "All .spec files targeted for migration exist on disk and are readable"
    dry_run_flag_set "The --dry-run flag is specified on the migrate command"
  }

  ensures {
    diff_produced "Unified diff of all migration changes is computed and displayed"
    no_files_modified "No .spec files are modified on disk during dry-run"
    migration_diff_generated_emitted "migration_diff_generated event fires with the computed diff"
  }

  contract """
    When specforge migrate --dry-run is invoked, the system MUST compute
    and display the unified diff of all changes that would be applied without
    modifying any files. The diff MUST use POSIX standard unified diff format
    (IEEE Std 1003.1 unified output format) compatible with patch(1) and
    git apply. Each file's diff MUST be clearly labeled with the file path
    using the `--- a/path` and `+++ b/path` header convention. A failure
    to compute the diff for a specific file MUST NOT prevent diff generation
    for other files — the system MUST report per-file outcomes using
    MigrationResult. When `--format=json` is specified, the diff MUST be
    serialized as a MigrationDiff JSON object with file-level entries, each
    containing path, before/after content hashes, and a list of changed
    line ranges.
  """

  verify unit "dry-run shows unified diff without modifying files"
  verify unit "diff format is compatible with patch(1)"
  verify unit "each file diff labeled with file path"
  verify unit "failure in one file does not block diff generation for others"
  verify unit "json format diff produces structured output with file-level entries"
  verify contract "requires/ensures consistency for migration diff generation"

}

behavior validate_post_migration_integrity "Validate Post-Migration Integrity" {
  invariants [multi_error_collection, graph_traversal_integrity, migration_event_ordering, diagnostic_determinism, migration_semantic_preservation, migration_cross_extension_stability, zero_domain_knowledge_core]
  types      [Graph, DiagnosticBag]
  ports      [CompilerApi, GraphSerializer]
  // Runs exactly once after the terminal migration event
  // (extension_migration_hooks_complete).
  consumes   [extension_migration_hooks_complete]
  produces   [migration_validation_complete]

  requires {
    extension_hooks_complete_fired "extension_migration_hooks_complete event has fired, confirming all extension migration hooks have run"
  }

  ensures {
    structural_equivalence_checked "Post-migration graph compared against pre-migration graph at the Graph Protocol level"
    differences_reported "Any structural differences reported as warnings"
    migration_validation_complete_emitted "migration_validation_complete event fires after validation finishes"
  }

  contract """
    This behavior validates instance-level integrity (entity IDs, edges,
    field values) after migration. Schema-level backward compatibility is
    handled separately by verify_graph_protocol_compatibility_after_migration.

    After all migration transforms and extension hooks have completed,
    the system MUST export the post-migration graph via GraphSerializer
    and compare at the Graph Protocol level (entity IDs, edge
    relationships, field values). Source spans (line, column) MUST be
    excluded from comparison since format migration inherently shifts
    source positions. Any structural differences MUST be reported as
    warnings. New diagnostics introduced by migration MUST be reported.
  """

  verify unit "post-migration check runs automatically"
  verify unit "structural equivalence verified between pre and post graphs"
  verify unit "structural differences reported as warnings"
  verify unit "new diagnostics from migration reported"
  verify contract "requires/ensures consistency for post-migration integrity validation"

}

behavior capture_pre_migration_schema_snapshot "Capture Pre-Migration Schema Snapshot" {
  invariants [migration_event_ordering, graph_schema_completeness, zero_domain_knowledge_core]
  types      [Graph, SchemaEntityKind, SchemaEdgeType, PreMigrationSnapshot]
  ports      [CompilerApi]
  consumes   [migration_starting]
  produces   [pre_migration_snapshot_captured]

  requires {
    migration_starting_fired "migration_starting event has fired, signaling that migration intent is declared but no file I/O has begun"
  }

  ensures {
    snapshot_captured "PreMigrationSnapshot contains node kinds, edge types, and field definitions from the current graph schema"
    pre_migration_snapshot_captured_emitted "pre_migration_snapshot_captured event fires with the captured snapshot"
  }

  contract """
    Before migration begins, the system MUST capture the current graph
    schema (node kinds, edge types, field definitions) into a
    PreMigrationSnapshot structure to enable before/after comparison.
    This snapshot MUST be taken after parsing but before any migration
    transforms are applied. The migration_starting event is the single
    trigger. No comparison happens at this stage — the snapshot is
    stored for later use by
    verify_graph_protocol_compatibility_after_migration.
  """

  verify unit "pre-migration schema snapshot captured on migration_starting event"
  verify unit "snapshot includes node kinds, edge types, and field definitions"
  verify unit "snapshot persists in memory across migration_starting to extension_migration_hooks_complete"
  verify contract "requires/ensures consistency for pre-migration schema capture"

}

behavior verify_graph_protocol_compatibility_after_migration "Verify Graph Protocol Compatibility After Migration" {
  invariants [graph_traversal_integrity, graph_schema_completeness, migration_event_ordering, migration_cross_extension_stability, zero_domain_knowledge_core]
  types      [Graph, FormatVersion, MigrationResult, SchemaVersion, SchemaEntityKind, SchemaEdgeType, PreMigrationSnapshot]
  ports      [CompilerApi]
  // Single-phase comparison: waits for the pre-migration snapshot and
  // the terminal migration event (extension_migration_hooks_complete).
  // Runs exactly once after both are available — no multi-phase logic.
  consumes   [pre_migration_snapshot_captured, extension_migration_hooks_complete]
  produces   [graph_protocol_compatibility_verified]

  requires {
    pre_migration_snapshot_available "pre_migration_snapshot_captured event has fired, providing the before-state schema snapshot"
    extension_hooks_complete "extension_migration_hooks_complete event has fired, confirming the fully-migrated state is ready"
  }

  ensures {
    compatibility_verified "Schema-level backward compatibility check completed against pre-migration snapshot"
    breaking_changes_warned "W053 warning emitted for any breaking schema changes detected"
    graph_protocol_compatibility_emitted "graph_protocol_compatibility_verified event fires after comparison"
  }

  contract """
    This behavior validates schema-level backward compatibility of the
    Graph Protocol after migration. Instance-level integrity (entity IDs,
    edges, field values) is handled separately by
    validate_post_migration_integrity.

    After all migration transforms and extension hooks have completed,
    the system MUST verify Graph Protocol backward compatibility by
    retrieving the pre-migration schema snapshot (captured by
    capture_pre_migration_schema_snapshot) and comparing it against the
    current post-migration graph schema. If the migration introduces a
    breaking change to the graph output, the system MUST emit W053
    (migration_breaking_graph_change) warning identifying the affected
    schema version fields. Migrations
    that only change formatting or whitespace (no entity structural
    changes) MUST skip this check.

    A breaking change is defined as any of the following: removed node kinds,
    removed edge types, removed required fields, or changed field types.
    Adding new node kinds, new edge types, or new optional fields is NOT
    a breaking change. Renaming a node kind or edge type counts as a removal
    of the old name plus addition of a new name, and therefore IS breaking.

    Cross-extension reference stability: during migration, references
    between entities owned by different extensions MUST remain valid.
    If a migration transform renames or removes an entity that is
    referenced by an entity from another extension, the system MUST
    emit a diagnostic identifying the broken cross-extension reference.

    This links .spec format versioning (FormatVersion) to Graph Protocol
    versioning (SchemaVersion) per Principle 6: breaking changes require
    migration paths. See also: graph_protocol_versioning in features/output.spec.
  """

  verify unit "migration that changes entity structure triggers schema check"
  verify unit "migration that only changes formatting skips schema check"
  verify unit "breaking graph change emits W053 warning"
  verify unit "non-breaking graph change passes silently"
  verify unit "removed node kind detected as breaking"
  verify unit "removed edge type detected as breaking"
  verify unit "removed required field detected as breaking"
  verify unit "changed field type detected as breaking"
  verify unit "added optional field is not breaking"
  verify unit "comparison runs once after extension_migration_hooks_complete"
  verify unit "cross-extension reference broken by migration produces diagnostic"
  verify contract "requires/ensures consistency for graph protocol compatibility verification"

}

behavior rollback_failed_migration "Rollback Failed Migration" {
  invariants [migration_backup_safety, migration_atomicity, zero_domain_knowledge_core]
  types      [MigrationResult, MigrationSummary, MigrationBackup]
  ports      [FileSystem]
  consumes   [migration_started]
  produces   [migration_rolled_back]

  requires {
    migration_started "migration_started event has been emitted, providing the set of migrated files with backups"
  }

  ensures {
    files_restored "All original files are restored from their .bak backups"
    rollback_event_emitted "migration_rolled_back event is emitted with accurate restored, skipped, and failed counts"
  }

  maintains {
    backup_file_preservation ".bak backup files are never deleted during rollback (preserved for user inspection)"
  }

  contract """
    The behavior consumes migration_started to identify the set of files
    that have backups. Rollback is triggered via the --rollback CLI flag
    after post-migration validation detects structural differences.
    Automatic rollback is NOT performed — the user must explicitly opt in.
    When invoked, the system MUST restore all migrated files from their
    .bak backups. Each file MUST be restored atomically (write to temp,
    then rename). If a .bak file is missing for a migrated file, the
    system MUST emit a warning and skip that file. If a .bak restore
    fails for one file, the system MUST emit an E-level diagnostic and
    continue with the remaining files — a single restore failure MUST
    NOT block others. After rollback, .bak files MUST be preserved (not
    auto-deleted) so the user can retry or inspect them. The system MUST
    report the number of files restored, skipped, and failed.

    Note: consumes migration_started for informational context (the backup
    file set created during migration_started), NOT as an execution trigger.
    Rollback is triggered imperatively via the --rollback CLI flag.
  """

  verify unit "restores migrated files from .bak backups"
  verify unit "missing .bak file produces warning and skips"
  verify unit "restore is atomic per file"
  verify unit "rollback failure for one file does not block others"
  verify unit "summary reports restored, skipped, and failed counts"
  verify contract "requires/ensures consistency for migration rollback"

}

behavior invoke_extension_migration_hooks "Invoke Extension Migration Hooks" {
  // multi_error_collection applies here because extension hook failures are
  // collected into the DiagnosticBag (not fail-fast) — each hook failure is
  // an independent error that must be reported alongside others.
  invariants [multi_error_collection, migration_idempotency, wasm_sandbox_integrity, extension_isolation, extension_load_order_determinism, migration_cross_extension_stability]
  types      [MigrationResult, ExtensionLifecycleState, WasmTrapInfo, ManifestV2]
  ports      [CompilerApi, WasmRuntime]
  consumes   [migration_complete]
  produces   [extension_migration_hooks_complete]

  requires {
    migration_complete     "migration_complete event MUST have fired, confirming all .spec files have been migrated"
  }

  ensures {
    all_hooks_invoked      "Every installed extension's migration hook has been invoked"
    hooks_event_emitted    "extension_migration_hooks_complete event emitted"
  }

  maintains {
    extension_isolation    "A failing extension hook does not prevent invocation of remaining hooks"
  }

  contract """
    When migrating spec files, the compiler MUST invoke each installed
    extension's migration hook — a Wasm export whose name is declared in
    the extension manifest's `migration_hook` field on ManifestV2
    (signature: `fn(source_version: i32, target_version: i32) -> i32`).
    The `migration_hook` field is a string naming the Wasm export; if the
    field is absent or empty, no hook is invoked for that extension.
    Extension hooks are invoked after core format migration completes but
    before post-migration validation.

    Migration hook lifecycle: core migration runs first (transforming .spec
    file syntax), then extension hooks run (transforming extension-specific
    data within those files). Post-migration validation
    (validate_post_migration_integrity) runs exactly once, after BOTH core
    migration and all extension hooks have completed. This ensures
    validation sees the fully-migrated state, not an intermediate state.

    If an extension does not declare a `migration_hook` field in its
    ManifestV2, or the field value is empty, the compiler MUST skip that
    extension silently with no diagnostic. The absence of a hook is normal
    and expected for extensions that have no version-sensitive data.

    If an extension's migration hook returns an error (non-zero return code)
    or traps, the compiler MUST collect the error into the DiagnosticBag
    (including WasmTrapInfo if a trap occurred) and continue invoking
    migration hooks for the remaining extensions. A single extension hook
    failure MUST NOT prevent other extensions from running their hooks.

    The compiler MUST invoke extension hooks in the deterministic load order
    defined by extension_load_order_determinism. The compiler MUST NOT
    invoke hooks for extensions in a failed ExtensionLifecycleState.

    Cross-extension reference stability: extension migration hooks MUST
    NOT rename or remove entities that are referenced by other extensions.
    If a hook does so, the post-migration validation phase will detect
    the broken cross-extension references and report them as diagnostics.

    Hook timeout: each extension migration hook has a configurable timeout
    (default 30s). If a hook exceeds the timeout, it MUST be treated as
    a trap — the compiler MUST terminate the hook execution, collect a
    WasmTrapInfo diagnostic, and continue with the next extension.
  """

  verify unit "extension with migration_hook field has it invoked during migrate"
  verify unit "extension without migration_hook field is skipped silently"
  verify unit "extension with empty migration_hook field is skipped silently"
  verify unit "hook returning error collects diagnostic and continues"
  verify unit "hook that traps collects WasmTrapInfo and continues"
  verify unit "hooks invoked in deterministic extension load order"
  verify unit "extension in failed lifecycle state has hook skipped"
  verify unit "hook exceeding timeout treated as trap"
  verify unit "validation runs once after both core and extension hooks complete"
  verify contract "requires/ensures consistency for extension migration hooks"

}
