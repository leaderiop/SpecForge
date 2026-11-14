<!-- Ported from gsd-core v1.19.0 gsd-core/templates/retrospective.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Project Retrospective

*One retrospective per milestone, created when the milestone closes. Lessons feed forward into future planning.*

## The Retrospective's Typed Edits

A milestone's retrospective is its `retrospective` entity, `<version>-RETROSPECTIVE`, and each key lesson is a `lesson` of it. `specforge gsd close-milestone` creates the retrospective when it closes the milestone, from its `--built`, `--worked`, `--inefficient` and `--patterns` lists; its `phases`, `plans` and `sessions` are computed. `specforge schema --kind retrospective` (likewise `lesson`) lists the fields. Fill the values: free text in its quoted heredoc, as written, and an id or enum in its assignment. A refusal names its code: read it with `specforge explain <code>`.

Close the milestone with its retrospective (each list is a JSON list of strings):

```bash
MILESTONE="{the milestone's version, e.g. v1.0}"
IFS= read -rd '' BUILT <<'TEXT' || true; BUILT=${BUILT%$'\n'}
["{key deliverable}"]
TEXT
IFS= read -rd '' WORKED <<'TEXT' || true; WORKED=${WORKED%$'\n'}
["{what saved time, or a way of working that paid off}"]
TEXT
IFS= read -rd '' INEFFICIENT <<'TEXT' || true; INEFFICIENT=${INEFFICIENT%$'\n'}
["{missed opportunity, or what took longer than expected}"]
TEXT
IFS= read -rd '' PATTERNS <<'TEXT' || true; PATTERNS=${PATTERNS%$'\n'}
["{a convention to keep for the next milestone}"]
TEXT
specforge gsd close-milestone "$MILESTONE" --built "$BUILT" --worked "$WORKED" --inefficient "$INEFFICIENT" --patterns "$PATTERNS" || exit 1
```

Record a key lesson:

```bash
MILESTONE="{the milestone's version, e.g. v1.0}"
LESSON_ID="{L and the lesson's number, e.g. L1}"
KIND="{lesson, decision, pattern or surprise}"
IFS= read -rd '' TITLE <<'TEXT' || true; TITLE=${TITLE%$'\n'}
{the lesson in a few words}
TEXT
IFS= read -rd '' LESSON <<'TEXT' || true; LESSON=${LESSON%$'\n'}
{the lesson, specific enough to act on}
TEXT
RETRO="$MILESTONE/$MILESTONE-RETROSPECTIVE"
RETRO_FILE=$(specforge query "$RETRO" --depth 0 --pick '.nodes[0].file') || exit 1
specforge new lesson "$LESSON_ID" "$TITLE" --in "$RETRO_FILE" --retrospective "$RETRO" --kind "$KIND" --text "$LESSON" || exit 1
```

`specforge query <milestone> --depth 2 --kinds retrospective,lesson --format context` reads the retrospective back; across milestones, the lessons graduation promotes become project decisions.
