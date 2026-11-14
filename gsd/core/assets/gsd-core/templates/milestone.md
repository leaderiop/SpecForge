<!-- Ported from gsd-core v1.19.0 gsd-core/templates/milestone.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Milestone Entry Template

## The Milestone's Typed Edits

A milestone is its `milestone` entity and its phases are `phase` entities. Its `completion` is written by `specforge gsd close-milestone`; its `state`, `shipped`, `stats` (files, loc, phases, plans, tasks, days), `git_range` and `progress` are computed, never written. `specforge schema --kind milestone` (likewise `phase`) lists the fields. Fill the values: free text in its quoted heredoc, as written, and an id or version in its assignment. A refusal names its code: read it with `specforge explain <code>`.

Create the milestone and its first phase (`specforge gsd phase` adds each phase at the next number):

```bash
VERSION="{the milestone's version, e.g. v1.1}"
IFS= read -rd '' NAME <<'TEXT' || true; NAME=${NAME%$'\n'}
{the milestone's name}
TEXT
IFS= read -rd '' GOAL <<'TEXT' || true; GOAL=${GOAL%$'\n'}
{one sentence: what the milestone delivers}
TEXT
IFS= read -rd '' PHASE_NAME <<'TEXT' || true; PHASE_NAME=${PHASE_NAME%$'\n'}
{the first phase's name}
TEXT
IFS= read -rd '' PHASE_GOAL <<'TEXT' || true; PHASE_GOAL=${PHASE_GOAL%$'\n'}
{the first phase's goal}
TEXT
IFS= read -rd '' PHASE_CRITERIA <<'TEXT' || true; PHASE_CRITERIA=${PHASE_CRITERIA%$'\n'}
{a JSON list of 2 to 5 observable behaviours that show the phase is done, e.g. ["A board has a share link", "The link opens the board read-only"]}
TEXT
specforge new milestone "$VERSION" "$NAME" --in "spec/$VERSION.spec" --name "$NAME" --goal "$GOAL" || exit 1
specforge gsd phase --milestone "$VERSION" --name "$PHASE_NAME" --goal "$PHASE_GOAL" --criteria "$PHASE_CRITERIA" || exit 1
```

When it ships, record what was delivered and what comes next (the close records the completion):

```bash
MILESTONE="{the milestone's version, e.g. v1.0}"
IFS= read -rd '' DELIVERED <<'TEXT' || true; DELIVERED=${DELIVERED%$'\n'}
{one sentence describing what shipped}
TEXT
IFS= read -rd '' WHATS_NEXT <<'TEXT' || true; WHATS_NEXT=${WHATS_NEXT%$'\n'}
{the next milestone's goals, or "Project complete"}
TEXT
specforge set "$MILESTONE" delivered "$DELIVERED" || exit 1
specforge set "$MILESTONE" whats_next "$WHATS_NEXT" || exit 1
```

`specforge query <milestone> --depth 1 --format context` reads the milestone and its phases back, with the computed stats.

---

<guidelines>
**When to create milestones:**
- Initial v1.0 MVP shipped
- Major version releases (v2.0, v3.0)
- Significant feature milestones (v1.1, v1.2)

**Don't create milestones for:**
- Individual phase completions (normal workflow)
- Work in progress (wait until shipped)
- Minor bug fixes that don't constitute a release
</guidelines>
