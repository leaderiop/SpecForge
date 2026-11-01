<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick/steps/quick-verification.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 6.5: Verification (only when `$VALIDATE_MODE`)**

Skip this step entirely if NOT `$VALIDATE_MODE`.

Display banner:
```
### GSD ► VERIFYING RESULTS

◆ Spawning verifier... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)
```

```
Agent(
  prompt="Verify quick task goal achievement.
Task directory: ${QUICK_DIR}
Task goal: ${DESCRIPTION}

<required_reading>
- `specforge query ${quick_id} --depth 1 --format context` (Plan)
</required_reading>

${AGENT_SKILLS_VERIFIER}

Check must_haves against actual codebase. Create VERIFICATION.md at ${QUICK_DIR}/${quick_id}-VERIFICATION.md.",
  subagent_type="gsd-verifier",
  model="{verifier_model}",
  description="Verify: ${DESCRIPTION}"
)
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

Read verification status via the canonical query (frontmatter-anchored, and total over its input space):
```bash
# the commands below are specforge commands: there is no launcher to define
STATUS=$(specforge query "${quick_id}" --kinds verification --pick '.nodes[kind=verification].fields.status') || { echo "the verification of this task was refused — see the error above." >&2; exit 1; }; STATUS=${STATUS%%$'\n'*}; STATUS=${STATUS:-pending}
```

`--pick status` returns the bare value, so this path needs **no `jq`**. That is deliberate: #2589
established that a `| jq -r '.field'` pipe yields an **empty** variable with no diagnostic on any
machine without jq — the default on Windows/Git-Bash — which here would route a perfectly good
`passed` verification into the recovery arm below.

Route on `$STATUS` and store the display string as `$VERIFICATION_STATUS` (consumed by the quick index row and the completion banner).

The query is **total** over the closed VerificationStatus set (#5118): beyond the verifier's own statuses it can also return `missing` (no `*-VERIFICATION.md`, or no `status` in its frontmatter), `stale` (the covered inputs changed after the verifier ran), `unparseable` (the report's frontmatter is not YAML) or `phase_dir_not_found` (no directory at `${QUICK_DIR}`). A report `status` outside `passed | gaps_found | human_needed` is not a status at all: the query exits non-zero with the reason on stderr, and the fence above stops. Which status wins when more than one applies is the query's own precedence, not this table's concern — every non-verifier status lands in the same arm here. That arm is reachable in normal operation and must never be dropped.

| `$STATUS` | Action |
|--------|--------|
| `passed` | Store `$VERIFICATION_STATUS = "Verified"`, continue to step 7 |
| `human_needed` | Display items needing manual check, store `$VERIFICATION_STATUS = "Needs Review"`, continue |
| `gaps_found` | Display gap summary, offer: 1) Re-run executor to fix gaps, 2) Accept as-is. Store `$VERIFICATION_STATUS = "Gaps"` |
| anything else — `missing`, `stale`, `unparseable`, `phase_dir_not_found` | Do **not** improvise a result. Report that verification produced no usable status, naming `$STATUS`, then offer: 1) Re-run the verifier, 2) Accept as-is without verification. Store `$VERIFICATION_STATUS = "Unverified (${STATUS})"` |

> **Why the status only, and not `next_action` / `next_command`.** The query projects those two for
> the phase pipeline — they name `execute-phase`, `plan-phase --gaps` and `verify-work`, and they
> append a phase-number argument taken from the directory basename. A quick task directory is named
> `${quick_id}-${slug}` with a date-derived `quick_id`, so that argument resolves to the date: for
> `260808-abc-some-slug` the projected recovery command carries `260808` as its phase argument — a
> date posing as a phase number.
> Quick therefore supplies its own recovery actions above. The split is the point:
> `readVerificationStatus` *discovers and parses* shape-agnostically — it scans whatever directory
> it is given for `*-VERIFICATION.md` — which is what makes the status half correct for
> `${QUICK_DIR}`; but it also reads that directory's basename as a phase token to build the
> projected commands, and that is the half quick must not use.
