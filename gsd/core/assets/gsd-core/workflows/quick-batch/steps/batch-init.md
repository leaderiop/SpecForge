<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick-batch/steps/batch-init.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 2b: Create a new batch (only when `$RESUME_BATCH_ID` is empty)**

Skip this step entirely if `$RESUME_BATCH_ID` is set (resume-mode.md owns
that path instead).

**Get the task list.** If `--file <path>` was present in `$ARGUMENTS`, use its
value as `$TASK_FILE`. Otherwise the remaining, non-flag text of `$ARGUMENTS`
IS the inline task list (a bulleted/numbered list, ≥2 items — the same
grammar `parseTaskList` enforces).

If `$TASK_FILE` is set:
```bash
# the commands below are specforge commands: there is no launcher to define
QB_ITEMS=$(sed -E 's/^[[:space:]]*([-*]|[0-9]+[.)])[[:space:]]+//' "$TASK_FILE" | grep -v '^[[:space:]]*$'); QB_CREATE_LIST=$([ "$(printf '%s\n' "$QB_ITEMS" | grep -c .)" -ge 2 ] && printf '%s\n' "$QB_ITEMS" | while IFS= read -r t; do printf '%s\t%s\n' "$(date -u +%y%m%d)-$(LC_ALL=C tr -dc a-z0-9 </dev/urandom | head -c 3)" "$t"; done)   # one quick_id<TAB>description line per item, list markers stripped, at least 2 items, one quick id (<yymmdd>-<3 base-36>) per item; the batch router is not part of @gsd/core
```

Otherwise, the inline list must land on disk first — `quick-batch create`
only accepts `--file` (path-confined, same as `/gsd:quick-batch`'s own
security posture): write it to a scratch file under a temp area before
calling the verb.
```bash
TASK_FILE="$(mktemp)"
mkdir -p "$(dirname "$TASK_FILE")"
printf '%s\n' "$INLINE_TASK_LIST" > "$TASK_FILE"
QB_ITEMS=$(sed -E 's/^[[:space:]]*([-*]|[0-9]+[.)])[[:space:]]+//' "$TASK_FILE" | grep -v '^[[:space:]]*$'); QB_CREATE_LIST=$([ "$(printf '%s\n' "$QB_ITEMS" | grep -c .)" -ge 2 ] && printf '%s\n' "$QB_ITEMS" | while IFS= read -r t; do printf '%s\t%s\n' "$(date -u +%y%m%d)-$(LC_ALL=C tr -dc a-z0-9 </dev/urandom | head -c 3)" "$t"; done)   # one quick_id<TAB>description line per item, list markers stripped, at least 2 items, one quick id (<yymmdd>-<3 base-36>) per item; the batch router is not part of @gsd/core
rm -f "$TASK_FILE"
```

```bash
[ -n "${QB_CREATE_LIST:-}" ] && QB_CREATE_RC=0 || QB_CREATE_RC=1   # the list is empty when the task list held fewer than 2 items

```

**If `$QB_CREATE_RC` is non-zero:** the task list failed to parse (fewer than
2 items — row 2/12) or the dependency DAG was invalid. Print the CLI's error
message verbatim and STOP. Do not dispatch anything.

**Otherwise:** `$QB_CREATE_LIST` holds the batch's items, one per line (every
item starts `pending`, wave `0` — no dependency/file-overlap signal exists
yet before planning; this is expected, not a bug, per the design's negative-
space note).

```bash
BATCH_ID="$(date -u +%y%m%d)-batch"
BATCH_MANIFEST_JSON=$QB_CREATE_LIST   # the manifest's items, one quick_id<TAB>description line each: keep each item's status (pending) and wave (0) in your notes beside it
ITEM_COUNT=$(printf '%s\n' "$BATCH_MANIFEST_JSON" | grep -c .)
```

Report to user:
```
Creating quick batch ${BATCH_ID}: ${ITEM_COUNT} item(s).
Manifest: kept in this conversation (the batch router is not part of @gsd/core: no BATCH.json)
```

Continue to Step 3 in `quick-batch.md`.
