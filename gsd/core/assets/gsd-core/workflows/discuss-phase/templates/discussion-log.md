<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/discuss-phase/templates/discussion-log.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

# The discussion log — for discuss-phase git_commit step

> **Lazy-loaded.** Read this file only inside the `git_commit` step of `workflows/discuss-phase.md`.

## Purpose

Audit trail for human review (compliance, learning, retrospectives). NOT
consumed by downstream agents — those read CONTEXT.md only.

## The Log's Typed Edit

A phase's discussion log is one `discussion` entity in `spec/discuss.spec` (`specforge schema --kind discussion`),
its `entries` one block per question asked. Its entries are append-only: an entry once written is never rewritten or
removed. An option carries its description after " — "; `notes` holds the user's clarifications, follow-up context or
rationale; `mode` is `interactive`, `assumptions` (with `confidence` and `evidence`) or `advisor`. Claude's discretion
areas and deferred ideas are on the context, so the log does not repeat them. A log an interrupted discussion's
checkpoint created, or an earlier session of this phase wrote, already exists: `ENTRIES` holds this session's entries
alone, and the fence writes the log's current entries followed by them, since a list that does not start with the
committed entries is refused. The fence creates the log otherwise.

```bash
PHASE_ID="{the phase's id, e.g. v1.0/03}"
PADDED_PHASE="{the phase number, zero-padded, e.g. 03}"
IFS= read -rd '' NAME <<'TEXT' || true; NAME=${NAME%$'\n'}
{the phase name}
TEXT
IFS= read -rd '' ENTRIES <<'TEXT' || true; ENTRIES=${ENTRIES%$'\n'}
[{"area": "{the gray area}", "question": "{the question asked}", "options": ["{an option} — {its description}"], "chosen": "{the selected option, or the free-text answer}", "mode": "{interactive, assumptions or advisor}", "notes": "{the clarifications or rationale the user gave}"}]
TEXT
LOG=$(specforge query "$PHASE_ID" --depth 1 --pick '.nodes[kind=discussion].id') || exit 1
if [ -n "$LOG" ]; then
  NOW=$(specforge query "$LOG" --depth 0 --format json --pick '.nodes[0].fields.entries') || exit 1
  case "$NOW" in
    ''|'[]') ALL=$ENTRIES ;;
    *) ALL="${NOW%]}, ${ENTRIES#[}" ;;   # the committed entries first: the list is append-only
  esac
  specforge set "$LOG" entries "$ALL" || exit 1
else
  specforge new discussion "${PADDED_PHASE}-DISCUSSION-LOG" "$NAME discussion" --in spec/discuss.spec --phase "$PHASE_ID" --entries "$ENTRIES" || exit 1
fi
```
