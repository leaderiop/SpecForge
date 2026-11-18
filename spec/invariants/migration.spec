requirement INVARIANT-075 "Migration Idempotency" {
  category "Invariant"
  tier     v2
  text     """
    Running specforge migrate twice on the same .spec files MUST produce
    identical output. A file already at the target version MUST be left
    unchanged. The migration transform MUST be a pure function of the
    source content and version pair — no external state may influence
    the result.
  """
}

requirement INVARIANT-071 "Migration Backup Safety" {
  category "Invariant"
  tier     v2
  text     """
    Before any in-place migration modifies a .spec file, the system MUST
    create a backup copy with a .bak extension unless --no-backup is
    explicitly specified. The backup MUST be a byte-for-byte copy of the
    original file. If backup creation fails (e.g., disk full, permission
    denied), the migration MUST abort without modifying any file.
  """
}

requirement INVARIANT-070 "Migration Atomicity" {
  category "Invariant"
  tier     v2
  text     """
    Each .spec file MUST be written atomically during migration: the system
    MUST write the migrated content to a temporary file in the same directory,
    then rename the temporary file to the target path, keeping the file's
    permissions. If the rename fails,
    the original file MUST remain unmodified. This guarantees that no
    partially written file can exist on disk, even if the process is
    interrupted mid-write.
  """
}

requirement INVARIANT-074 "Migration Event Ordering" {
  category "Invariant"
  tier     v2
  text     """
    Migration events MUST be emitted in strict temporal order:
    migration_starting → migration_started → migration_complete →
    extension_migration_hooks_complete. No event in this sequence MAY
    be emitted before its predecessor has completed. Consumers that
    depend on this ordering (e.g., pre-migration snapshot capture)
    MUST receive events in the guaranteed sequence.
  """
}

requirement INVARIANT-076 "Migration Semantic Preservation" {
  category "Invariant"
  tier     v2
  text     """
    Migration transforms MUST preserve the entity graph structure. After
    migration, the set of nodes (entity IDs, and each node's kind, title,
    field values and methods) and edges (reference relationships) MUST be
    identical to the pre-migration graph, source positions aside: the
    graph delta between them is empty, except a title a migration step
    adds to a block that had none or a blank one (add_missing_titles); every other node
    and edge MUST be identical. Only syntax-level changes
    (whitespace, keyword spelling, block ordering) are permitted. Any
    migration that adds, removes, or alters a node or edge is a breaking
    change and MUST be flagged by validate_post_migration_integrity.
  """
}

requirement INVARIANT-072 "Migration Canonical Form" {
  category "Invariant"
  tier     v2
  text     """
    Every block a migration step adds or changes MUST be format-clean
    afterwards, laid out by the formatter with its file's format
    configuration (looked up through the run's bound) and its project's
    field forms, before the plan is diffed, judged and written. No byte
    outside those blocks changes but the format header, a file whose every
    line break is CR LF is written so (ADR 0266), and a file no step
    changes is left as it is.
  """
}

requirement INVARIANT-073 "Migration Cross-Extension Reference Stability" {
  category "Invariant"
  tier     v2
  text     """
    Migration MUST preserve cross-extension reference resolution.
    References that resolved before migration MUST resolve identically
    after migration. References to entities in uninstalled extensions
    MUST retain their soft-resolution status (I004).
  """
}
