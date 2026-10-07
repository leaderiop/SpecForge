# A mutation reports what it wrote, typed

**Status:** accepted (2026-10-06)

What an MCP mutation did crossed the dispatcher seam as JSON. Each operation returned a typed
outcome; the handler built a `json!` reply from it; a per-tool `effect` closure in the tool table
read that reply back by key (`o["installed"]`, `o["success"]`, `o["migrated"]`), or ignored it and
returned 3. Enabling a builtin edited one file and reported three; init with a local extension
wrote five and reported three; a format that failed on one file reported none of those it wrote; a
migration reported the file it migrated and not its backup. Domain events came from three
mechanisms (`push_event` in a handler, `ToolOutcome::with_event`, the dispatcher), with payloads
that were not the spec's, and the refresh of the call target after a write was decided in three
places (`Call::wrote` in two handlers, a dispatcher fallback, `init`'s own `serve`). And since
`mcp_mutation_completed` never leaves the process, a stdio client had no way to learn which files
`add_extension`, `remove_extension`, `init` or `infer_session` wrote.

## Decisions

- **D1. Operations record their writes.** Every writing operation (format, rename, init, add,
  remove, migrate) returns the files it created, rewrote or removed and left so
  (`specforge_ops::Writes`), recorded at the write call. A write that changed nothing is not one; a
  file restored by a rollback is forgotten.
- **D2. A mutation handler returns what it wrote.** `Handler::Mutation` returns `Mutated`: its reply
  and a `Written { files, entities, event }` built from the operation's outcome, or nothing for a
  preview (`dry_run`, `check`, `diff`), decided from its typed arguments. No JSON is read back.
- **D3. One module acts on it.** `specforge_mcp::mutation::refresh` brings the call target up to date
  when files were written, whether or not the call then succeeded, by the call-target rules of
  ADR 0014; `report` records the domain event (on success) and then `mcp_mutation_completed`. No
  handler refreshes, pushes an event or reads `Call` state about writes.
- **D4. `files_changed` counts every file the call wrote**, backups and a failed call's partial
  writes included; `entities_affected` the entities it changed. A removal names each file it
  deleted, not the directory; an operation that fails after writing returns its writes on its
  `OpError` (`OpError::writes`).
- **D5. Domain events carry the spec's payloads** (`extension_added { extensionSpecifier,
  totalExtensions, wasDuplicate }`, emitted for a duplicate add too; `project_initialized
  { projectName, extensionCount, specFilePath }`).
- **D6. The CLI reads `Writes`, not `Written`.** It has no session to refresh and no event sink;
  `specforge init` lists the files it wrote from its outcome, and `init`, `add`, `remove` and
  `migrate` put them in their JSON output as `files_written` (a failed run's JSON error too, when
  it left files written).
- **D7. A mutation tells its caller what it wrote.** Events never leave the process, so every MCP
  mutation reply that is not a preview carries `files_written`: root-relative paths (absolute
  outside the root), sorted, in `structuredContent` or in a refusal's `data`; `[]` when it wrote
  nothing. `mutation::report` is its one writer, and `files_changed` is its length.

## Consequences

- `MutationSpec`, `Effect`, `ToolOutcome::with_event`/`take_events` and `Call::wrote` are gone; a
  new mutation tool cannot report a count it did not compute.
- An argument refusal of a mutation is reported as a failed mutation even with `dry_run: true`:
  arguments that do not parse cannot say they asked for a preview.
- A migration that fails after writing is brought up to date in the same call.
- `collect` and `render` stay management tools: their files are output artifacts, not mutations.
- The seven mutation tools' output schemas declare `files_written` (not required: a preview has
  none; the schemas stay open, so existing clients keep validating).
- Amends ADR 0014 ("The MCP call target"): handlers no longer call `Call::wrote`.
