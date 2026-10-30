<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/extract-learnings.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

<purpose>
Extract decisions, lessons learned, patterns discovered, and surprises encountered from completed phase artifacts into `lesson` entities. Captures institutional knowledge that would otherwise be lost between phases.
</purpose>

<required_reading>
Read all files referenced by the invoking prompt's execution_context before starting.
</required_reading>

<objective>
Analyze completed phase artifacts (PLAN.md, SUMMARY.md, VERIFICATION.md, UAT.md, STATE.md) and extract structured learnings into 4 categories: decisions, lessons, patterns, and surprises. Each extracted item includes source attribution. The output is one `lesson` entity per item, with its `kind` set to the category.
</objective>

<process>

<step name="initialize">
Parse arguments and load project state:

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
# the commands below are specforge commands: there is no launcher to define
PHASE_ARG=$(echo "$GSD_ARGS" | sed -E 's/--ws[[:space:]]+[A-Za-z0-9._-]+//g' | xargs)
GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -n 1)
INIT=$(specforge query "$PHASE_ID" --depth 2 --format json)
PHASE_NUMBER="${PHASE_ID##*/}"   # the phase number is the last segment of its id
```

Parse from the query JSON: the phase's `name` (its number is the last segment of `$PHASE_ID`, kept in `PHASE_NUMBER`).

If phase not found, exit with error: "Phase {PHASE_ARG} not found."
</step>

<step name="collect_artifacts">
Read the phase artifacts. PLAN.md and SUMMARY.md are required; VERIFICATION.md, UAT.md, and STATE.md are optional.

**Required artifacts:**
- the phase's plans (`specforge query "$PHASE_ID" --depth 2 --kinds plan --format context`)
- the `summary` entities of its plans (`specforge query "$PHASE_ID" --depth 3 --kinds summary --format context`)

If PLAN.md or SUMMARY.md files are not found or missing, exit with error: "Required artifacts missing. PLAN.md and SUMMARY.md are required for learning extraction."

**Optional artifacts (read if available, skip if not found):**
- the phase's `verification` entities (`specforge query "$PHASE_ID" --depth 2 --kinds verification --format context`) — verification results
- the phase's `uat_session` entities (`specforge query "$PHASE_ID" --depth 2 --kinds uat_session --format context`) — user acceptance test results
- the project state (`specforge gsd progress-report --format json`) — with decisions and blockers

Track which optional artifacts are missing for the `missing_artifacts` frontmatter field.
</step>

<step name="extract-learnings">
Analyze all collected artifacts and extract learnings into 4 categories:

### 1. Decisions
Technical and architectural decisions made during the phase. Look for:
- Explicit decisions documented in PLAN.md or SUMMARY.md
- Technology choices and their rationale
- Trade-offs that were evaluated
- Design decisions recorded in STATE.md

Each decision entry must include:
- **What** was decided
- **Why** it was decided (rationale)
- **Source:** attribution to the artifact where the decision was found (e.g., "Source: 03-01-PLAN.md")

### 2. Lessons
Things learned during execution that were not known beforehand. Look for:
- Unexpected complexity in SUMMARY.md
- Issues discovered during verification in VERIFICATION.md
- Failed approaches documented in SUMMARY.md
- UAT feedback that revealed gaps

Each lesson entry must include:
- **What** was learned
- **Context** for the lesson
- **Source:** attribution to the originating artifact

### 3. Patterns
Reusable patterns, approaches, or techniques discovered. Look for:
- Successful implementation patterns in SUMMARY.md
- Testing patterns from VERIFICATION.md or UAT.md
- Workflow patterns that worked well
- Code organization patterns from PLAN.md

Each pattern entry must include:
- **Pattern** name/description
- **When to use** it
- **Source:** attribution to the originating artifact

### 4. Surprises
Unexpected findings, behaviors, or outcomes. Look for:
- Things that took longer or shorter than estimated
- Unexpected dependencies or interactions
- Edge cases not anticipated in planning
- Performance or behavior that differed from expectations

Each surprise entry must include:
- **What** was surprising
- **Impact** of the surprise
- **Source:** attribution to the originating artifact
</step>

<step name="capture_thought_integration">
**What this step is:** `capture_thought` is an **optional convention**, not a bundled GSD tool. GSD does not ship one and does not require one. The step is a hook for users who run a memory / knowledge-base MCP server (for example ExoCortex-style servers, `claude-mem`, or `mem0`-style servers) that exposes a tool with this exact name. If any MCP server in the current session provides a `capture_thought` tool with the signature below, each extracted learning is routed through it with metadata. If no such tool is present, the step is a silent no-op — the `lesson` entities are always the primary output.

**Detection:** Check whether a tool named `capture_thought` is available in the current session. Do not assume any specific MCP server is connected.

**If available**, call once per extracted learning:

```
capture_thought({
  category: "decision" | "lesson" | "pattern" | "surprise",
  phase: PHASE_NUMBER,
  content: LEARNING_TEXT,
  source: ARTIFACT_NAME
})
```

**If not available** (no MCP server in the session exposes this tool, or the runtime does not support it), skip the step silently and continue. The workflow must not fail or warn — this is expected behavior for users who do not run a knowledge-base MCP.
</step>

<step name="write_learnings">
Record the learnings as `lesson` entities: one `specforge new lesson` per learning, in the order of the four categories below. A lesson has no phase field, so its id carries the phase: `L-<phase>-<n>` (for example `L-02-1`). Running again on the same phase replaces the lessons of the earlier run: set the fields of an existing id with `specforge set` instead of creating it again, and tell the user about any earlier lesson of the phase that the new run no longer has (a person removes it from the spec file).

Where it goes: the lessons file `spec/lessons.spec` (`--in spec/lessons.spec`; `specforge new` creates it when missing).

Each lesson is written with this command:
```bash
specforge new lesson "L-${PHASE_NUMBER}-1" "<short title>" --in spec/lessons.spec \
  --kind decision \
  --text "<what was decided, why it was decided, Source: artifact>" \
  --milestone "$MILESTONE"

# --kind is decision, lesson, pattern or surprise.
# The text holds the item's body: for a decision its rationale, for a lesson its context,
# for a pattern when to use it, for a surprise its impact, and always "Source: <artifact>".
# The phase number is in the id; the phase name and the date are not stored.
# The count of each category is the number of lessons of that kind, so it is not stored either.
# The missing optional artifacts go in the report below.
# An id that exists is changed with: specforge set "L-${PHASE_NUMBER}-1" text "<text>"

```

Individual items may carry an optional `graduated:` annotation (added by `graduation.md` when a cluster is promoted):
```markdown
**Graduated:** {target-file}:{ISO_DATE}
```
This annotation is appended after the item's existing fields and prevents the item from being re-surfaced in future graduation scans. Do not add this field during extraction — it is written only by the graduation workflow.

The title and text of each lesson are written like this: the `###` heading is the lesson's title, the lines under it are its `--text`, and the `##` heading is its `--kind`. The `#` heading and the `---` rules are not stored:
```markdown
# Phase {PHASE_NUMBER} Learnings: {PHASE_NAME}

## Decisions

### {Decision Title}
{What was decided}

**Rationale:** {Why}
**Source:** {artifact file}

---

## Lessons

### {Lesson Title}
{What was learned}

**Context:** {context}
**Source:** {artifact file}

---

## Patterns

### {Pattern Name}
{Description}

**When to use:** {applicability}
**Source:** {artifact file}

---

## Surprises

### {Surprise Title}
{What was surprising}

**Impact:** {impact description}
**Source:** {artifact file}
```
</step>

<step name="calibrate_estimates">
Rebuild the estimate-vs-actual calibration from every completed phase (#2632, ADR-2629).

```bash
: # estimate-calibrate: not available here, so there is no calibration to rebuild
```

This step is not available here: the planner's estimate correction is a gap, so there is no calibration
to rebuild and no `factor`, `sample_count` or `confidence` to report.
Skip this step, and leave it out of the report.







Phases missing either side are skipped rather than guessed: a fabricated sample would
steer every future estimate.
</step>

<step name="update_state">
The project state is computed from the entities, so there is nothing to update:

```bash
: # state.update: nothing to update, the last activity is computed
```
</step>

<step name="report">
```
---------------------------------------------------------------

## Learnings Extracted: Phase {X} — {Name}

Decisions:  {N}
Lessons:    {N}
Patterns:   {N}
Surprises:  {N}
Total:      {N}

Recorded: the lessons L-{PADDED_PHASE}-1 … L-{PADDED_PHASE}-{N}

Missing artifacts: {list or "none"}

Next steps:
- Review extracted learnings for accuracy
- /gsd:progress — see overall project state
- /gsd:execute-phase {next} — continue to next phase

---------------------------------------------------------------
```
</step>

</process>

<success_criteria>
- [ ] Phase artifacts located and read successfully
- [ ] All 4 categories extracted: decisions, lessons, patterns, surprises
- [ ] Each extracted item has source attribution
- [ ] One `lesson` written per learning, each with its `kind`
- [ ] Missing optional artifacts reported
- [ ] capture_thought integration attempted if tool available
- [ ] The project state needs no update (it is computed)
- [ ] User receives summary report
</success_criteria>

<critical_rules>
- PLAN.md and SUMMARY.md are required — exit with clear error if missing
- VERIFICATION.md, UAT.md, and STATE.md are optional — extract from them if present, skip gracefully if not found
- Every extracted learning must have source attribution back to the originating artifact
- Running extract-learnings twice on the same phase must replace the previous lessons (`specforge set` on the same ids), not add new ones
- Do not fabricate learnings — only extract what is explicitly documented in artifacts
- If capture_thought is unavailable, the workflow must not fail — graceful degradation to the `lesson` entities only
- Every lesson must carry its `kind`; list any missing optional artifacts in the report
</critical_rules>
