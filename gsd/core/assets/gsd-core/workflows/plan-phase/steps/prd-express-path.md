<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-phase/steps/prd-express-path.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

# PRD Express Path — record the phase's context from a PRD

Runs when `--prd <filepath>` is provided (§3.5 of `plan-phase.md`).

1. Read the PRD file:
```bash
PRD_CONTENT=$(specforge files read "$PRD_FILE" --pick .text 2>/dev/null)
if [ -z "$PRD_CONTENT" ]; then
  echo "Error: PRD file not found: $PRD_FILE"
  exit 1
fi
```

2. Display banner:
```
### GSD ► PRD EXPRESS PATH

Using PRD: {PRD_FILE}
Recording the phase context from requirements...
```

3. Parse the PRD content into the phase's context. The orchestrator should:
   - Extract all requirements, user stories, acceptance criteria, and constraints from the PRD
   - Map each to a locked decision (everything in the PRD is treated as a locked decision)
   - Identify any areas the PRD doesn't cover and mark as "Claude's Discretion"
   - **Extract canonical refs** from the phase entity, plus any specs/ADRs referenced in the PRD — expand to full file paths (MANDATORY)

4. Record the context as typed edits, filling the values (free text in its quoted heredoc, as written) (`specforge schema --kind context`, `--kind decision`
   and `--kind deferred_idea` list the fields): one `new decision` line per requirement, story or criterion of the PRD
   (`D-01`, `D-02`, …), one `new deferred_idea` line per item the PRD marks as future, v2 or out of scope, and no line
   for a part that is empty:

```bash
PHASE_ID="{the phase id, e.g. v1.0/03}"
CONTEXT="{padded phase}-CONTEXT"
IFS= read -rd '' TITLE <<'TEXT' || true; TITLE=${TITLE%$'\n'}
{the phase's name} context
TEXT
IFS= read -rd '' BOUNDARY <<'TEXT' || true; BOUNDARY=${BOUNDARY%$'\n'}
{what this phase delivers, extracted from the PRD}
TEXT
IFS= read -rd '' DISCRETION <<'TEXT' || true; DISCRETION=${DISCRETION%$'\n'}
{an area the PRD does not cover: implementation details, technical choices}
TEXT
IFS= read -rd '' SPECIFICS <<'TEXT' || true; SPECIFICS=${SPECIFICS%$'\n'}
{specific references, examples or concrete requirements from the PRD}
TEXT
IFS= read -rd '' CANONICAL_REFS <<'TEXT' || true; CANONICAL_REFS=${CANONICAL_REFS%$'\n'}
[{"topic": "{a topic area}", "paths": ["{the full relative path of a spec or ADR it names}"]}]
TEXT
IFS= read -rd '' AREA <<'TEXT' || true; AREA=${AREA%$'\n'}
{the category derived from the content}
TEXT
IFS= read -rd '' DECISION <<'TEXT' || true; DECISION=${DECISION%$'\n'}
{a requirement of the PRD, as a locked decision}
TEXT
IFS= read -rd '' DEFERRED <<'TEXT' || true; DEFERRED=${DEFERRED%$'\n'}
{an item the PRD marks as future, v2 or out of scope}
TEXT
PHASE_FILE=$(specforge query "$PHASE_ID" --depth 0 --pick '.nodes[0].file') || exit 1
CONTEXT_ID="$PHASE_ID/$CONTEXT"
specforge new context "$CONTEXT" "$TITLE" --in "$PHASE_FILE" --phase "$PHASE_ID" --boundary "$BOUNDARY" --discretion "$DISCRETION" --specifics "$SPECIFICS" --canonical_refs "$CANONICAL_REFS" || exit 1
specforge new decision D-01 "$DECISION" --in "$PHASE_FILE" --phase "$PHASE_ID" --area "$AREA" --text "$DECISION" --origin context || exit 1
specforge new deferred_idea DI1 "$DEFERRED" --in "$PHASE_FILE" --phase "$PHASE_ID" --origin discussion --text "$DEFERRED" || exit 1
```

`CANONICAL_REFS` holds one item per topic area, grouping its paths. With no external spec, one item whose topic says
so and whose `paths` is `[]`: "No external specs — requirements fully captured in decisions above".

5. Commit:
```bash
# the commands below are specforge commands: there is no launcher to define
specforge git commit --add spec -m "docs(${padded_phase}): generate context from PRD" && specforge collect --runner gsd-git --yes
```

6. Set `context_content` to the recorded context (`specforge query "$CONTEXT_ID" --depth 1 --format context`) and continue to step 5 (Handle Research).

**Effect:** This completely bypasses step 4 (load the phase context) since we just recorded it. The rest of the workflow (research, planning, verification) proceeds normally with the PRD-derived context.
