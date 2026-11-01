<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/milestone-summary.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
@~/.claude/gsd-core/references/response-language-directive.md

# Milestone Summary Workflow

Generate a comprehensive, human-friendly project summary from completed milestone artifacts.
Designed for team onboarding — a new contributor can read the output and understand the entire project.

---

## Step 1: Resolve Version

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
VERSION=$(echo "$GSD_ARGS" | sed -nE 's/.*--milestone[[:space:]]+v?([A-Za-z0-9._-]+).*/\1/p'); VERSION=${VERSION:-${MILESTONE#v}}   # --milestone, else the milestone the run is about
```

If `--milestone` is not given:
1. Take the milestone the run is about (`$MILESTONE`: the one still open, else the last)
2. Else take the latest closed milestone: the last id `specforge export --kinds milestone --pick '.nodes[fields.state=shipped].id'` prints
3. A milestone with no phases yet is mid-build: the summary adapts to what it has
4. If nothing found: error "No milestone found. Run /gsd:new-project or /gsd:new-milestone first."

Set `VERSION` to the resolved version (e.g., "1.0").

## Step 2: Locate Artifacts

Determine whether the milestone is **archived** or **current**:

**Archived milestone** (it has a `completion`):
```
ROADMAP_QUERY='specforge query "v${VERSION}" --depth 1 --kinds phase --format context'
REQUIREMENTS_QUERY='specforge query "v${VERSION}" --depth 3 --kinds requirement --format context'
AUDIT_QUERY='specforge query "v${VERSION}" --depth 1 --kinds milestone_audit --format context'
```

**Current/in-progress milestone** (no archive yet):
```
ROADMAP_QUERY='specforge query "v${VERSION}" --depth 1 --kinds phase --format context'
REQUIREMENTS_QUERY='specforge query "v${VERSION}" --depth 3 --kinds requirement --format context'
AUDIT_QUERY='specforge query "v${VERSION}" --depth 1 --kinds milestone_audit --format context'
```

Note: nothing moves when a milestone closes: the audit is the `milestone_audit` entity of the milestone in both cases, so there is no second location to check.

**Always available:**
```
PROJECT_QUERY='specforge query "$PROJECT" --depth 0 --format context'
RETRO_QUERY='specforge query "v${VERSION}" --depth 1 --kinds retrospective --format context'
STATE_QUERY='specforge gsd progress-report --format json'
```

Run each of these queries (`eval "$ROADMAP_QUERY"`, and so on). An empty answer is fine — the summary adapts to what's available.

## Step 3: Discover Phase Artifacts

Find all phases of the milestone:

```bash
IFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true
$ARGUMENTS
GSD_ARGS_END
GSD_ARGS=${GSD_ARGS%$'\n'}
# the commands below are specforge commands: there is no launcher to define
GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -n 1)
specforge gsd progress-report --format json
```

This returns the progress of the phases (the roadmap query above names them). For each phase in the milestone scope:

- Read the phase's `summary` entities (`specforge query "v${VERSION}/{padded}" --depth 3 --kinds summary --format context`) if there are any — extract `one_liner`, `accomplishments`, `decisions`
- Read its `verification` entities (`--kinds verification`) if there are any — extract status, gaps, deferred items
- Read its `context` entities (`--kinds context`) if there are any — extract key decisions from the `<decisions>` section
- Read its `research` entities (`--kinds research`) if there are any — note what was researched

Track which phases have which artifacts.

**If no phases have been executed** (empty milestone or pre-build state): skip to Step 5 and generate a minimal summary noting "No phases have been executed yet." Do not error — the summary should still capture the `project` entity and the roadmap.

## Step 4: Gather Git Statistics

Try each method in order until one succeeds:

**Method 1 — Tagged milestone** (check first):
```bash
git tag -l "v${VERSION}" | head -1
```
If the tag exists:
```bash
git log v${VERSION} --oneline | wc -l
git diff --stat $(git log --format=%H --reverse v${VERSION} | head -1)..v${VERSION}
```

**Method 2 — Project start date** (if no tag):
Read the progress report and extract the `started_at` or earliest session date. Use it as the `--since` boundary:
```bash
git log --oneline --since="<started_at_date>" | wc -l
```

**Method 3 — Earliest recorded commit** (if no date was recorded):
Find the earliest `commit` entity of the milestone:
```bash
specforge query "v${VERSION}" --depth 4 --kinds commit --pick '.nodes[kind=commit]' --fields fields.at,fields.hash | sort | head -1
```
Use that commit's date as the start boundary.

**Method 4 — Skip stats** (if none of the above work):
Report "Git statistics unavailable — no tag or date range could be determined." This is not an error — the summary continues without the Stats section.

Extract (when available):
- Total commits in milestone
- Files changed, insertions, deletions
- Timeline (start date → end date)
- Contributors (from git log authors)

## Step 5: Generate Summary Document

Compose the summary (it is shown, never written):

```markdown
# Milestone v{VERSION} — Project Summary

**Generated:** {date}
**Purpose:** Team onboarding and project review

---

## 1. Project Overview

{From PROJECT.md: "What This Is", core value proposition, target users}
{If mid-milestone: note which phases are complete vs in-progress}

## 2. Architecture & Technical Decisions

{From CONTEXT.md files across phases: key technical choices}
{From SUMMARY.md decisions: patterns, libraries, frameworks chosen}
{From PROJECT.md: tech stack if documented}

Present as a bulleted list of decisions with brief rationale:
- **Decision:** {what was chosen}
  - **Why:** {rationale from CONTEXT.md}
  - **Phase:** {which phase made this decision}

## 3. Phases Delivered

| Phase | Name | Status | One-Liner |
|-------|------|--------|-----------|
{For each phase: number, name, status (complete/in-progress/planned), one_liner from SUMMARY.md}

## 4. Requirements Coverage

{From REQUIREMENTS.md: list each requirement with status}
- ✅ {Requirement met}
- ⚠️ {Requirement partially met — note gap}
- ❌ {Requirement not met — note reason}

{If MILESTONE-AUDIT.md exists: include audit verdict}

## 5. Key Decisions Log

{Aggregate from all CONTEXT.md <decisions> sections}
{Each decision with: ID, description, phase, rationale}

## 6. Tech Debt & Deferred Items

{From VERIFICATION.md files: gaps found, anti-patterns noted}
{From RETROSPECTIVE.md: lessons learned, what to improve}
{From CONTEXT.md <deferred> sections: ideas parked for later}

## 7. Getting Started

{Entry points for new contributors:}
- **Run the project:** {from PROJECT.md or SUMMARY.md}
- **Key directories:** {from codebase structure}
- **Tests:** {test command from PROJECT.md or CLAUDE.md}
- **Where to look first:** {main entry points, core modules}

---

## Stats

- **Timeline:** {start} → {end} ({duration})
- **Phases:** {count complete} / {count total}
- **Commits:** {count}
- **Files changed:** {count} (+{insertions} / -{deletions})
- **Contributors:** {list}
```

## Step 6: Show, never write

**Nothing is written:** the summary is shown and not stored, so there is no earlier summary to overwrite and no reports directory to create.



There is no commit either: the summary is not a file.
```bash
: # no reports directory and no commit: the summary is not a file
```

Compose the summary from the sections above:
```bash
: # nothing to commit

```

## Step 7: Present Summary

Display the full summary document inline.

## Step 8: Offer Interactive Mode

After presenting the summary:

> "Summary shown above; it is not stored as a file.
>
> I have full context from the build artifacts. Want to ask anything about the project?
> Architecture decisions, specific phases, requirements, tech debt — ask away."

If the user asks questions:
- Answer from the artifacts already loaded (CONTEXT.md, SUMMARY.md, VERIFICATION.md, etc.)
- Reference specific files and decisions
- Stay grounded in what was actually built (not speculation)

If the user is done:
- Suggest next steps: `/gsd:new-milestone`, `/gsd:progress`, or sharing the summary with the team

## Step 9: Nothing to update

```bash
: # state.record-session: the project state is computed from the entities, so there is nothing to record


```
