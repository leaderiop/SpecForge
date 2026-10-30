<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/quick-batch/steps/batch-init.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
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
QB_ITEMS=$(sed -E 's/^[[:space:]]*([-*]|[0-9]+[.)])[[:space:]]+//' "$TASK_FILE" | grep -v '^[[:space:]]*$'); QB_CREATE_JSON=$([ "$(printf '%s\n' "$QB_ITEMS" | grep -c .)" -ge 2 ] && printf '%s\n' "$QB_ITEMS" | while IFS= read -r t; do printf '%s\t%s\n' "$(date -u +%y%m%d)-$(LC_ALL=C tr -dc a-z0-9 </dev/urandom | head -c 3)" "$t"; done | jq -Rn --arg id "$(date -u +%y%m%d)-batch" '{batchId: $id, manifest: {items: [inputs|split("\t")|{quickId: .[0], task: .[1], status: "pending", wave: 0}]}}')   # list markers stripped, at least 2 items, one quick id (<yymmdd>-<3 base-36>) per item; the batch router is not part of @gsd/core
```

Otherwise, the inline list must land on disk first — `quick-batch create`
only accepts `--file` (path-confined, same as `/gsd:quick-batch`'s own
security posture): write it to a scratch file under a temp area before
calling the verb.
```bash
TASK_FILE="$(mktemp)"
mkdir -p "$(dirname "$TASK_FILE")"
printf '%s\n' "$INLINE_TASK_LIST" > "$TASK_FILE"
QB_ITEMS=$(sed -E 's/^[[:space:]]*([-*]|[0-9]+[.)])[[:space:]]+//' "$TASK_FILE" | grep -v '^[[:space:]]*$'); QB_CREATE_JSON=$([ "$(printf '%s\n' "$QB_ITEMS" | grep -c .)" -ge 2 ] && printf '%s\n' "$QB_ITEMS" | while IFS= read -r t; do printf '%s\t%s\n' "$(date -u +%y%m%d)-$(LC_ALL=C tr -dc a-z0-9 </dev/urandom | head -c 3)" "$t"; done | jq -Rn --arg id "$(date -u +%y%m%d)-batch" '{batchId: $id, manifest: {items: [inputs|split("\t")|{quickId: .[0], task: .[1], status: "pending", wave: 0}]}}')   # list markers stripped, at least 2 items, one quick id (<yymmdd>-<3 base-36>) per item; the batch router is not part of @gsd/core
rm -f "$TASK_FILE"
```

```bash
QB_CREATE_RC=$?
if [[ "$QB_CREATE_JSON" == @file:* ]]; then QB_CREATE_JSON=$(cat "${QB_CREATE_JSON#@file:}"); fi
```

**If `$QB_CREATE_RC` is non-zero:** the task list failed to parse (fewer than
2 items — row 2/12) or the dependency DAG was invalid. Print the CLI's error
message verbatim and STOP. Do not dispatch anything.

**Otherwise:** parse `$QB_CREATE_JSON` for `batchId` and `manifest` (every
item starts `pending`, wave `0` — no dependency/file-overlap signal exists
yet before planning; this is expected, not a bug, per the design's negative-
space note).

```bash
BATCH_ID="$batchId"
BATCH_MANIFEST_JSON=$(printf '%s' "$QB_CREATE_JSON" | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{try{const j=JSON.parse(s);process.stdout.write(JSON.stringify(j.manifest))}catch{process.stdout.write("")}})')
ITEM_COUNT=$(printf '%s' "$BATCH_MANIFEST_JSON" | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{try{const j=JSON.parse(s);process.stdout.write(String(j.items.length))}catch{process.stdout.write("0")}})')
```

Report to user:
```
Creating quick batch ${BATCH_ID}: ${ITEM_COUNT} item(s).
Manifest: kept in this conversation (the batch router is not part of @gsd/core: no BATCH.json)
```

Continue to Step 3 in `quick-batch.md`.
