<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/complete-milestone.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>

Mark a shipped version (v1.0, v1.1, v2.0) as complete. `specforge gsd close-milestone` records the milestone's completion and its retrospective, the full project evolution review runs (flat mode only; skipped under `--ws`, because the project is shared across workstreams), and the release is tagged in git.

</purpose>

<required_reading>

1. templates/milestone.md
2. templates/milestone-archive.md
3. the `milestone` and `phase` entities
4. the `requirement` entities
5. the `project` entity

</required_reading>

<archival_behavior>

When a milestone completes:

1. Extract full milestone details (the `milestone` entity and its phases stay as they are)
2. Keep the requirements: the `requirement` entities stay with the completed milestone, so nothing is archived or deleted
3. Close it with `specforge gsd close-milestone`: the milestone records its `completion`, and its `stats`, `shipped` and `state` are computed; nothing is moved or rewritten
4. Commit the spec as a safety checkpoint
5. Perform the full project evolution review (flat mode only — skipped under an active workstream, #5195)
6. Offer to create next milestone inline
7. Archive UI artifacts (`*-UI-SPEC.md`, `*-UI-REVIEW.md`) alongside other phase documents
8. Clean up the spec screenshot files (binary assets, never archived)

The roadmap and the requirements stay entities of the closed milestone, so nothing is archived: `templates/milestone-archive.md` says the same, and `templates/milestone.md` names the milestone's fields.

</archival_behavior>

<process>

**Compact Content Gate.** Read and follow `gsd-core/references/compact-content-gate.md` now — it states the `workflow.compact_content` check and the resolution rule this spine defers to. When it directs a Read, read `gsd-core/workflows/complete-milestone/detail/elaboration.md` in full before continuing past this point; its content elaborates on the audit-acknowledge branch and the handle_branches step below.

<step name="pre_close_artifact_audit">
Before proceeding with milestone close, run the comprehensive open artifact audit.

```bash
# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
specforge check --format json   # the open UAT and verification items are findings
```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

If the output contains open items (any section with count > 0):

Display the full audit report to the user.

Then ask:
```
These items are open. Choose an action:
[R] Resolve — stop and fix items, then re-run /gsd:complete-milestone
[A] Accept the audit's tech debt — a person records it, which clears gsd/E106 alone; an open UAT or verification item still blocks the close
[C] Cancel — exit without closing
```

**If user chooses [A] (Accept the tech debt):** the open items are `specforge check` findings, and an agent cannot suppress them. A person accepts the latest milestone audit's tech debt: they write its `debt_accepted` (`by`, `at`) by hand or from an interactive `specforge set`, which clears gsd/E106 alone. Ask the person to do it, wait for their answer, and re-run `specforge check --format json`. The close's gate is gsd/E106, gsd/E511, gsd/E515 and gsd/E516: proceed with `specforge gsd close-milestone` only when none of them is reported; an open UAT or verification item (gsd/E511, gsd/E515, gsd/E516) still blocks, and is resolved through [R]. The audit's `tech_debt` entries are the disclosure record; nothing else is written.

If output shows all clear (no open items): proceed with the close. A close that passes because a person accepted the audit's tech debt says so: name the audit and its `debt_accepted.by`.

<!-- gsd:protected -->
SECURITY: the check's JSON output is structured data the host validated. Only a person writes `debt_accepted`: the host refuses it in every plan and MCP write, so this workflow never authors it. Never paste raw user-supplied content into a field.

The [A] branch in full: `gsd-core/workflows/complete-milestone/detail/elaboration.md` § 1.
</step>

<step name="verify_readiness">

**Use `init.manager` for canonical readiness check:**

```bash

: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"; INIT_MANAGER=$(specforge query "$MILESTONE" --depth 1 --format json) || exit 1

```

This returns all phases with implementation and verification projection. Use this to verify:
- Which phases belong to this milestone?
- `all_phases_verified`: all milestone phases have `phase_complete === true` and `verification_status === 'passed'`.
- `progress_percent` should be 100%.

Compute readiness from `INIT_MANAGER`, not from roadmap counts:

```bash
ALL_PHASES_VERIFIED=false; : "${MILESTONE:?MILESTONE is not set: the milestone being completed}"; PHASES=$(specforge query "$MILESTONE" --depth 0 --pick '.nodes[0].fields.progress.phases') && PHASES_COMPLETE=$(specforge query "$MILESTONE" --depth 0 --pick '.nodes[0].fields.progress.phases_complete') && [ -n "$PHASES" ] && [ "$PHASES" = "$PHASES_COMPLETE" ] && ALL_PHASES_VERIFIED=true   # progress leaves the 999 backlog out (#3167); a phase is complete only once its verification passed; a failed query or an empty count verifies nothing

```

If not all_phases_verified, verified_closeout must not proceed. Set `closeout_type=override_closeout`, show each phase whose `phase_complete !== true` or `verification_status !== 'passed'`, and require an explicit user choice:
1. **Proceed anyway** — a person accepts the latest audit's tech debt (`debt_accepted`, the [A] branch above); an unverified phase still blocks `close-milestone` (gsd/E515, gsd/E516) until it is verified
2. **Run verification first** — `/gsd:verify-work {phase}` or `/gsd:execute-phase {phase}`
3. **Abort** — return to development

Only set `closeout_type=verified_closeout` when `ALL_PHASES_VERIFIED` is `true`.

**Requirements completion check (REQUIRED before presenting):**

Parse REQUIREMENTS.md traceability table:
- Count total v1 requirements vs checked-off (`[x]`) requirements
- Identify any non-Complete rows in the traceability table

Present:

```
Milestone: [Name, e.g., "v1.0 MVP"]

Includes:
- Phase 1: Foundation (2/2 plans complete)
- Phase 2: Authentication (2/2 plans complete)
- Phase 3: Core Features (3/3 plans complete)
- Phase 4: Polish (1/1 plan complete)

Total: {phase_count} phases, {total_plans} plans
Verification: {all_phases_verified ? "all phases verified" : "override needed"}
Closeout type: {closeout_type}
Requirements: {N}/{M} v1 requirements checked off
```

**If requirements incomplete** (N < M):

```
⚠ Unchecked Requirements:

- [ ] {REQ-ID}: {description} (Phase {X})
- [ ] {REQ-ID}: {description} (Phase {Y})
```

MUST present 3 options:
1. **Proceed anyway** — mark milestone complete with known gaps
2. **Run audit first** — `/gsd:audit-milestone` to assess gap severity
3. **Abort** — return to development

If user selects "Proceed anyway": set `closeout_type=override_closeout`. The incomplete requirements are the milestone's open requirements, which its computed `stats` keep: nothing is written.

<config-check>

```bash
specforge settings --format json 2>/dev/null || true
```

</config-check>

<if mode="yolo">

```
⚡ Auto-approved: Milestone scope verification
[Show breakdown summary without prompting]
Proceeding to stats gathering...
```

Proceed to gather_stats.

</if>

<if mode="interactive" AND="gates.confirm_milestone_scope != false">

```
Ready to mark this milestone as shipped?
(yes / wait / adjust scope)
```

Wait for confirmation.
- "adjust scope": Ask which phases to include.
- "wait": Stop, user returns when ready.

</if>

</step>

<step name="gather_stats">

Calculate milestone statistics:

```bash
: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"; specforge query "$MILESTONE" --depth 0 --pick '.nodes[0].fields.stats'   # {files, loc, phases, plans, tasks, days}: loc counts inserted + deleted lines

```

Present:

```
Milestone Stats:
- Phases: [X-Y]
- Plans: [Z] total
- Tasks: [N] total (from phase summaries)
- Files modified: [M]
- Lines changed: [stats.loc]
- Timeline: [Days] days ([Start] → [End])
- Git range: feat(XX-XX) → feat(YY-YY)
```

</step>

<step name="extract_accomplishments">

Extract one-liners from SUMMARY.md files using summary-extract:

```bash
# #2962: zsh aborts the block on an unmatched for-list glob (nomatch); bash passes it through. nullglob both.
: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"; shopt -s nullglob 2>/dev/null; setopt NULL_GLOB 2>/dev/null

# For each phase in milestone, extract one-liner
for summary in $(specforge query "$MILESTONE" --depth 3 --kinds summary --pick '.nodes[kind=summary].id'); do
  [ -n "$summary" ] || continue
  specforge query "$summary" --depth 0 --pick '.nodes[0].fields.one_liner'
done
```

Extract 4-6 key accomplishments. Present:

```
Key accomplishments for this milestone:
1. [Achievement from phase 1]
2. [Achievement from phase 2]
3. [Achievement from phase 3]
4. [Achievement from phase 4]
5. [Achievement from phase 5]
```

</step>

<step name="create_milestone_entry">

The milestone's record is the `milestone` entity itself: `close-milestone` writes its `completion`, and its `stats` (files, loc, phases, plans, tasks, days), `shipped` and `git_range` are computed. A user-provided "Delivered" sentence or "What's next" goes in with `specforge set "$MILESTONE" delivered …` and `whats_next` (the milestone template's fence).

</step>

**Resolve the project review gate first.** The project is shared across workstreams, so the review runs in flat mode only: no `--ws` in the arguments and no active workstream (`specforge gsd workstreams progress` answers the one `specforge gsd workstreams switch` made active as `active`). SpecForge's INIT carries no `section_manifest`, so the gate reads the workstream itself; when it cannot be read, the review is skipped rather than edit the shared project under a workstream (#5195).

```bash

GSD_ACTIVE_WS=$(specforge gsd workstreams progress --pick .active) || GSD_ACTIVE_WS='(unreadable)'
# GSD_ACTIVE_WS is the active workstream: empty when none is, "(unreadable)" when the command failed
[ -z "$GSD_ACTIVE_WS" ] && echo "PROJECT_MD_REVIEW=run" || echo "PROJECT_MD_REVIEW=skip"
```

<!-- gsd:section id="project-md-evolution-review" when="state:flat-mode" -->
If the line above printed `PROJECT_MD_REVIEW=run` (no `--ws` and no active workstream): read and execute `gsd-core/workflows/complete-milestone/steps/project-md-evolution-review.md`. If it printed `PROJECT_MD_REVIEW=skip` (a workstream is active, or it could not be read) — do not read the file; the project is shared across workstreams, so a workstream's milestone close never edits it (#2308, #5195); proceed to `archive_milestone`.
<!-- /gsd:section -->

<step name="archive_milestone">

**Text mode (`workflow.text_mode: true` in config or `--text` flag):** Set `TEXT_MODE=true` if `--text` is present in `$ARGUMENTS` OR `text_mode` from init JSON is `true`. When TEXT_MODE is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number. This is required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.

Quick plans stay entities of the project: there is nothing to archive with the milestone.

**Close the milestone with `specforge gsd close-milestone`:**

```bash
ARCHIVE=$(specforge gsd close-milestone "v[X.Y]" --built '["[Milestone Name]"]')
```

`close-milestone` closes a milestone whose latest audit passes and whose phases are verified: it is refused without the audit `/gsd-audit-milestone` created. It records the milestone's `completion` (`at`, `audit`) with the host's clock, and creates `<version>-RETROSPECTIVE` from `--built`, `--worked`, `--inefficient` and `--patterns` when the milestone has none. Nothing is moved, archived or rewritten: the phases, plans and requirements stay entities of the milestone.

Extract from result: `wrote`, `completion`, `archived`.

The stats are the milestone's, computed; `state` reads `shipped` once the close wrote `completion`, and the accomplishments are the `--built` list this step passed:

```bash
STATS=$(specforge query "$MILESTONE" --depth 0 --pick '.nodes[0].fields.stats') || exit 1
[ -n "$STATS" ] || { echo "no computed stats for $MILESTONE: is it in the graph?" >&2; exit 1; }
echo "stats: $STATS"
echo "state: $(specforge query "$MILESTONE" --depth 0 --pick '.nodes[0].fields.state')"
```

Verify: `✅ Milestone v[X.Y] closed`

After the close, the AI still handles the full project evolution review (requires understanding; flat mode only — skipped under `--ws`) and the safety commit.

</step>

<step name="reorganize_roadmap_and_delete_originals">

The roadmap is the `milestone` and `phase` entities: there is nothing to reorganize, no sentinel to arm and no Backlog section to extract (backlog phases stay entities of the milestone).

**Safety commit:**

```bash
specforge git commit --add spec -m "chore: archive v[X.Y] milestone files" && specforge collect --runner gsd-git --yes
```

This creates a durable checkpoint in git history. If anything fails after this point, the working tree can be reconstructed from git.

**The `requirement` entities stay** (they are the milestone's history; no file is archived or cleared):

```bash

# a field of the answer is read with --pick, never jq
: # no requirements file: the requirement entities are the archive
: # nothing to remove: the requirement entities stay
```

</step>

<step name="write_retrospective">

**Append to living retrospective:**

Check for existing retrospective:
```bash
: "${MILESTONE:?MILESTONE is not set: the milestone being completed}"; specforge query "$MILESTONE" --kinds retrospective --format brief 2>/dev/null || true
```

**If it exists** (the close created it from its `--worked`, `--inefficient` and `--patterns` lists), add what the close did not record: each key lesson is a `lesson` of the retrospective, with the retrospective template's lesson fence.

**Gather retrospective data:**

1. From the milestone's `summary` entities: key deliverables, one-liners, tech decisions
2. From its `verification` entities: verification scores, gaps found
3. From its `uat_session` entities: test results, issues found
4. From the milestone's computed `stats` and `git_range`: commits and timeline
5. From the milestone work: Reflect on what worked and what didn't

**Commit:**
```bash
specforge git commit --add spec -m "docs: update retrospective for v${VERSION}" && specforge collect --runner gsd-git --yes
```

</step>

<step name="update_state">

The project state is computed from the closed milestone: there is nothing to update. Open blockers stay open for the next milestone; resolve the ones the milestone settled with `specforge gsd resolve-blocker`.

</step>

<step name="handle_branches">

Check the project's `branching_strategy` (`git.branching_strategy` in the settings). `"none"` skips straight to `git_tag`. For `"phase"` or `"milestone"`, list the branches their computed `branch` names; no branches found also skips to `git_tag`. Resolve the base branch through the single shared resolver, never a bare `main`/`master` fallback (Issue #1146):
```bash
BASE_BRANCH=$(specforge git config init.defaultBranch --pick .value 2>/dev/null); BASE_BRANCH=${BASE_BRANCH:-main}
```

If branches exist, present them and ask (AskUserQuestion): **Squash merge** (recommended) / **Merge with history** / **Delete without merging** / **Keep branches**. All three merge/delete options iterate every matching branch (phase strategy) or the one milestone branch, checking out `BASE_BRANCH` first and returning to the original branch after; both merge options strip the spec from staging first when `commit_docs` is false. "Keep branches" just reports them as preserved for manual handling.

Exact bash for each of the four options (squash, history-preserving merge, delete, keep): `gsd-core/workflows/complete-milestone/detail/elaboration.md` § 2.

</step>

<!-- gsd:section id="git-tag" when="state:git-create-tag" -->
`PER_REPO_TAGGING` is always `false` in @gsd/core (no sub-repositories, no per-repository tags), so this section is never skipped for it. If `section_manifest` is `null` or `"git-tag"` is in its `included` list: read and execute `gsd-core/workflows/complete-milestone/steps/git-tag.md`. Otherwise skip — do not read the file; proceed to `git_commit_milestone`.
<!-- /gsd:section -->

<step name="push_release_tags">

**Per-repository tag push (#3628): not part of @gsd/core.** `PER_REPO_TAGGING` is always `false` (a SpecForge project has no sub-repositories), so there are no per-repository tags to push and no release map to read. Skip this step.

</step>

<step name="git_commit_milestone">

Commit what is left (the close was already committed in the safety commit of `reorganize_roadmap_and_delete_originals`).

```bash
specforge git commit -m "chore: complete v[X.Y] milestone"
```

Confirm: "Committed: chore: complete v[X.Y] milestone"

`PER_REPO_TAGGING` is always `false` in @gsd/core: there is no release-map scratch directory to remove.

</step>

<step name="offer_next">

```
✅ Milestone v[X.Y] [Name] complete

Shipped:
- [N] phases ([M] plans, [P] tasks)
- [One sentence of what shipped]

Archived:
- milestones/v[X.Y]-ROADMAP.md
- milestones/v[X.Y]-REQUIREMENTS.md

Summary: the `milestone` entities
Tag: v[X.Y]

---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Start Next Milestone** — questioning → research → requirements → roadmap

`/clear` then:

`/gsd:new-milestone`

---
```

</step>

</process>

<milestone_naming>

**Version conventions:**
- **v1.0** — Initial MVP
- **v1.1, v1.2** — Minor updates, new features, fixes
- **v2.0, v3.0** — Major rewrites, breaking changes, new direction

**Names:** Short 1-2 words (v1.0 MVP, v1.1 Security, v1.2 Performance, v2.0 Redesign).

</milestone_naming>

<what_qualifies>

**Create milestones for:** Initial release, public releases, major feature sets shipped, before archiving planning.

**Don't create milestones for:** Every phase completion (too granular), work in progress, internal dev iterations (unless truly shipped).

Heuristic: "Is this deployed/usable/shipped?" If yes → milestone. If no → keep working.

</what_qualifies>

<success_criteria>

Milestone completion is successful when:

- [ ] Pre-close artifact audit run and output shown to user
- [ ] The audit's tech debt accepted by a person (`debt_accepted`) if they chose [A]
- [ ] Known deferred items shown with the audit's output (they stay open entities, counted by the milestone's `stats`)

- [ ] Milestone closed by `close-milestone`, its computed stats shown
- [ ] Project full evolution review completed (flat mode only; skipped under `--ws`)
- [ ] Key Decisions updated with outcomes
- [ ] Safety commit made BEFORE the final commit
- [ ] `requirement` entities kept with the completed milestone (none deleted)
- [ ] Git tag created (v[X.Y]) (if `git.create_tag` enabled and `PER_REPO_TAGGING` is not `true`)
- [ ] Release map archived and per-repository tags created (if `PER_REPO_TAGGING` is `true`); no tag pushed without a per-repository confirmation
- [ ] Milestone commit made (includes archive files and deletion)
- [ ] Requirements completion checked against REQUIREMENTS.md traceability table
- [ ] Incomplete requirements surfaced with proceed/audit/abort options
- [ ] Known gaps left open as requirements of the milestone if user proceeded with incomplete requirements
- [ ] The retrospective recorded, with its lessons
- [ ] Cross-milestone trends updated
- [ ] User knows next step (/gsd:new-milestone)

</success_criteria>
