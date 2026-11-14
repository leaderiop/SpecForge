<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/graduation.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# graduation.md — LEARNINGS.md Cross-Phase Graduation Helper

**Invoked by:** `transition.md` step `graduation_scan`. Never invoked directly by users.

This workflow clusters recurring items across the last N phases' LEARNINGS.md files and surfaces promotion candidates to the developer via HITL. No item is promoted without explicit developer approval.

---

## Configuration

Read from project config (`config.json`):

| Key | Default | Description |
|-----|---------|-------------|
| `features.graduation` | `true` | Master on/off switch. `false` skips silently. |
| `features.graduation_window` | `5` | How many prior phases to scan |
| `features.graduation_threshold` | `3` | Minimum cluster size to surface |

---

## Step 1: Guard Checks

```bash
# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
GRADUATION_ENABLED="{{setting:features.graduation|true}}"   # features.graduation (the settings)
GRADUATION_WINDOW="{{setting:features.graduation_window|5}}"   # features.graduation_window (the settings)
GRADUATION_THRESHOLD="{{setting:features.graduation_threshold|3}}"   # features.graduation_threshold (the settings)
```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

**Skip silently (print nothing) if:**
- `features.graduation` is `false`
- Fewer than `graduation_threshold` completed prior phases exist (not enough data)

**Skip silently (print nothing) if total items across all LEARNINGS.md files in the window is fewer than 5.**

---

## Step 2: Collect LEARNINGS.md Files

Find LEARNINGS.md files from the last N completed phases (excluding the phase currently completing):

```bash
specforge query "$MILESTONE" --depth 2 --kinds lesson --pick '.nodes[kind=lesson].id' --last "$GRADUATION_WINDOW"
```

For each file found:
1. Read each lesson's `kind` (decision, lesson, pattern or surprise) and `text`: `specforge query "$MILESTONE" --depth 2 --kinds lesson --format context`
2. Extract each `### Item Title` + body as a single item record: `{ category, title, body, source_phase, source_file }`
3. **Skip lessons Step 4 does not list** — they have been promoted, dismissed or deferred and must not re-surface

---

## Step 3: Cluster by Lexical Similarity

For each category independently, cluster items using Jaccard similarity on tokenized title+body:

**Tokenization:** lowercase, strip punctuation, split on whitespace, remove stop words (a, an, the, is, was, in, on, at, to, for, of, and, or, but, with, from, that, this, by, as).

**Jaccard similarity:** `|A ∩ B| / |A ∪ B|` where A and B are token sets. Two items are in the same cluster if similarity ≥ 0.25.

**Clustering algorithm:** single-pass greedy — process items in phase order; add to the first cluster whose centroid (union of all cluster tokens) has similarity ≥ 0.25 with the new item; otherwise start a new cluster.

**Cluster size filter:** only surface clusters with distinct source phases ≥ `graduation_threshold` (not just total items — same item repeated in one phase still counts as 1 distinct phase).

---

## Step 4: Keep the Open Lessons

A lesson's `status` is computed from its recorded disposition: `open` unless it was graduated, and a deferred lesson is open again once the phase it waits for is complete. List the open lessons:

```bash
specforge query "$MILESTONE" --depth 2 --kinds lesson --pick '.nodes[fields.status=open].id'
```

**Skip any window lesson this list does not hold** (dismissed, promoted, or deferred until a phase not yet complete).

---

## Step 5: Surface Promotion Candidates

For each qualifying cluster, determine the suggested target file:

| Category | Suggested Target |
|----------|-----------------|
| `decisions` | a project `decision`, area "Validated Decisions", linked into the project's `key_decisions` |
| `patterns` | Surfaced only: with `@gsd/core` alone no kind holds a pattern, so a capability extension that declares one promotes it |
| `lessons` | a project `decision`, area "Invariants", linked into the project's `key_decisions` |
| `surprises` | Flag for human review — if genuinely surprising 3+ times, something structural is wrong |

Print the graduation report:

```text
📚 Graduation scan across phases {M}–{N}:

  HIGH RECURRENCE ({K}/{WINDOW} phases)
  ├─ Cluster: "{representative title}"
  ├─ Category: {category}
  ├─ Sources: {list of NN-LEARNINGS filenames}
  └─ Suggested target: {target file} § {section}

  [repeat for each qualifying cluster, ordered HIGH→LOW recurrence]

For each cluster above, choose an action:
  P = Promote now   D = Defer (re-surface next transition)   X = Dismiss (never re-surface)   A = Defer all remaining
```

---

## Step 6: HITL — Process Each Cluster

For each cluster (in order from Step 5), ask the developer:

```text
Cluster: "{title}" [{category}, {K} phases] → {target}
Action [P/D/X/A]:
```

Use `AskUserQuestion` (or equivalent HITL primitive for the current runtime). If `TEXT_MODE` is true, display the cluster question as plain text and accept typed input. Accept single-character input: `P`, `D`, `X`, `A` (case-insensitive).

**On `P` (Promote now):**

Before a promotion, look for an existing decision it would duplicate: `specforge search "{cluster title}" --kinds decision`. A hit is offered as the target: then only link it and record the disposition (the last two lines below), with `DECISION_ID` the hit's id. Otherwise record the cluster as a project decision, link it into the project's `key_decisions`, and record each lesson's disposition (`graduated` is written by `graduate-lessons`, never by `specforge set`):

```bash
DECISION_ID="{the next free project decision id, e.g. D-07}"
AREA="{Validated Decisions for decisions, Invariants for lessons}"
LESSONS='{the lesson ids of the cluster, as a JSON list, e.g. ["v1.0/v1.0-RETROSPECTIVE/L1"]}'
IFS= read -rd '' TITLE <<'TEXT' || true; TITLE=${TITLE%$'\n'}
{the cluster's representative title}
TEXT
IFS= read -rd '' BODY <<'TEXT' || true; BODY=${BODY%$'\n'}
{the merged body: the unique sentences across the cluster's lessons}
TEXT
IFS= read -rd '' SOURCES <<'TEXT' || true; SOURCES=${SOURCES%$'\n'}
Sources: {Phase A, Phase B, Phase C}
TEXT
PROJECT=$(specforge export --kinds project --pick '.nodes[0].id') || exit 1
[ -n "$PROJECT" ] || { echo "no project entity: run /gsd:new-project first" >&2; exit 1; }
PROJECT_FILE=$(specforge query "$PROJECT" --depth 0 --pick '.nodes[0].file') || exit 1
specforge new decision "$DECISION_ID" "$TITLE" --in "$PROJECT_FILE" --origin project --area "$AREA" --text "$BODY" --rationale "$SOURCES" || exit 1
specforge link "$PROJECT" key_decisions "$DECISION_ID" || exit 1
specforge gsd graduate-lessons "$LESSONS" --disposition promoted --target "$DECISION_ID" || exit 1
```

Then commit the spec in one atomic commit: `specforge git commit --add spec -m 'docs(learnings): graduate "{cluster title}" to a project decision'`.

**On `D` (Defer):** the cluster's lessons are open again once the next phase is complete:

```bash
LESSONS='{the lesson ids of the cluster, as a JSON list}'
NEXT_PHASE="{the next phase's id, e.g. v1.0/03}"
specforge gsd graduate-lessons "$LESSONS" --disposition deferred --until "$NEXT_PHASE" || exit 1
```

**On `X` (Dismiss):** the cluster's lessons never re-surface:

```bash
LESSONS='{the lesson ids of the cluster, as a JSON list}'
specforge gsd graduate-lessons "$LESSONS" --disposition dismissed || exit 1
```

**On `A` (Defer all):**

Defer the current cluster (same as `D`) and skip all remaining clusters for this run, deferring each to the next transition. Print:
```text
[graduation: deferred all remaining clusters to next transition]
```
Then proceed directly to Step 7.

---

## Step 7: Completion Report

After processing all clusters, print:

```text
Graduation complete: {promoted} promoted, {deferred} deferred, {dismissed} dismissed.
```

If no clusters qualified (all filtered by backlog or threshold), print:
```text
[graduation: no qualifying clusters in phases {M}–{N}]
```

---

## First-Run Behaviour

On the first transition after upgrading to a version that includes this workflow, all extant LEARNINGS.md files may produce a large batch of candidates at once. A `[Defer all]` shorthand is available: if the developer enters `A` at any cluster prompt, all remaining clusters for this run are deferred to the next transition.

---

## No-Op Conditions (silent skip)

- `features.graduation = false`
- Fewer than `graduation_threshold` prior phases with LEARNINGS.md
- Total items < 5 across the window
- Every qualifying cluster's lessons are dismissed, promoted or deferred (none is listed by Step 4)
