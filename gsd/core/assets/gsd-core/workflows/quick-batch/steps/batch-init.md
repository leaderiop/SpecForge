<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick-batch/steps/batch-init.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 2b: Create a new batch (only when `$RESUME_BATCH_ID` is empty)**

Skip this step entirely if `$RESUME_BATCH_ID` is set (resume-mode.md owns
that path instead).

**Get the task list.** Use `$TASK_FILE` from Step 1 (`--file <path>`) when it is set. Otherwise `$TASKS`
from Step 1 (the words after the options, `$INLINE_TASK_LIST` below)
IS the inline task list (a bulleted/numbered list, ≥2 items — the same
grammar `parseTaskList` enforces).

If `$TASK_FILE` is set:
```bash
# the commands below are specforge commands: there is no launcher to define
QB_TASKS=$(specforge files read "$TASK_FILE" --pick .text) || exit 1   # the list's text: quick-init parses it
```

Otherwise, the inline list is the text `quick-init` takes, so no scratch file
is written (a `--file` list stays path-confined, same as `/gsd:quick-batch`'s own
security posture, through `specforge files read`).

```bash
QB_TASKS=$INLINE_TASK_LIST




```

```bash
QB_CREATE_LIST=$(specforge gsd quick-init --batch --tasks="$QB_TASKS" --pick '.items[]' --fields id,slug,description) && QB_CREATE_RC=0 || QB_CREATE_RC=$?   # one row per item: quick_id, slug and description, separated by 0x1f; the list is attached with = because it starts with a hyphen

```

**If `$QB_CREATE_RC` is non-zero:** `quick-init` refused the list and said why: it held fewer than 2 items once
its list markers and blank lines were stripped (row 2/12). Say so, show the task
list as given, and STOP. Do not dispatch anything.

**Otherwise:** `$QB_CREATE_LIST` holds the batch's items, one row per line (every
item starts `pending`, wave `0` — no dependency/file-overlap signal exists
yet before planning; this is expected, not a bug, per the design's negative-
space note).

```bash
IFS=$'\x1f' read -r BATCH_ID ITEM_COUNT < <(specforge gsd quick-init --batch --tasks="$QB_TASKS" --pick . --fields batch,count)   # a function of the day and the list; this call's ids are discarded
BATCH_MANIFEST_JSON=$QB_CREATE_LIST   # the manifest's items, one quick_id, slug, description row each (read: IFS=$'\x1f' read -r quick_id slug description): keep each item's status (pending) and wave (0) in your notes beside it

```

Report to user:
```
Creating quick batch ${BATCH_ID}: ${ITEM_COUNT} item(s).
Manifest: kept in this conversation (the batch router is not part of @gsd/core: no BATCH.json)
```

Continue to Step 3 in `quick-batch.md`.
