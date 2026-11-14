<!-- Ported from gsd-core v1.19.0 gsd-core/templates/context.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Phase Context Template

Template for the `context` entity of phase XX-name - captures implementation decisions for a phase.

**Purpose:** Document decisions that downstream agents need. Researcher uses this to know WHAT to investigate. Planner uses this to know WHAT choices are locked vs flexible.

**Key principle:** Categories are NOT predefined. They emerge from what was actually discussed for THIS phase. A CLI phase has CLI-relevant sections, a UI phase has UI-relevant sections.

**Downstream consumers:**
- `gsd-phase-researcher` — Reads decisions to focus research (e.g., "card layout" → research card component patterns)
- `gsd-planner` — Reads decisions to create specific tasks (e.g., "infinite scroll" → task includes virtualization)

---

## The Context's Typed Edits

A phase's context is entities in `spec/discuss.spec`: one `context`, one `decision` (origin `context`) per decision
and one `deferred_idea` per idea that belongs in another phase, written with `specforge new`; nothing writes a context
by hand. `specforge schema --kind context` (likewise `decision`, `deferred_idea`) lists the fields, and `specforge new
context --help` prints them as flags. A list of texts takes its flag once per item (`--discretion "Spacing"
--discretion "Error copy"`), and `canonical_refs` is JSON. Fill the values: free text in its quoted heredoc, as written, and an id, enum or date in its assignment; leave out a flag you have no value
for, and repeat a `new decision` or `new deferred_idea` line per item with the next id (`D-02`, `DI2`). A context that
already exists ("Update it") takes `specforge set` / `specforge link` on its fields instead of `new`. A refusal names
its code: read it with `specforge explain <code>`.

```bash
PHASE_ID="{the phase's id, e.g. v1.0/03}"
PADDED_PHASE="{the phase number, zero-padded, e.g. 03}"
IFS= read -rd '' NAME <<'TEXT' || true; NAME=${NAME%$'\n'}
{the phase name}
TEXT
GATHERED="{today, YYYY-MM-DD}"
IFS= read -rd '' BOUNDARY <<'TEXT' || true; BOUNDARY=${BOUNDARY%$'\n'}
{what this phase delivers: the scope anchor, from the phase goal}
TEXT
IFS= read -rd '' SPECIFICS <<'TEXT' || true; SPECIFICS=${SPECIFICS%$'\n'}
{particular references, examples or I want it like X moments; or: No specific requirements, open to standard approaches}
TEXT
IFS= read -rd '' REFS <<'TEXT' || true; REFS=${REFS%$'\n'}
[{"topic": "{a topic area}", "paths": ["{path/to/adr-or-spec.md}"]}]
TEXT
IFS= read -rd '' DISCRETION <<'TEXT' || true; DISCRETION=${DISCRETION%$'\n'}
{an area where the user said you decide: one --discretion per area}
TEXT
IFS= read -rd '' ASSET <<'TEXT' || true; ASSET=${ASSET%$'\n'}
{a component, hook or utility, and how this phase could use it: one --reusable_assets per item}
TEXT
IFS= read -rd '' PATTERN <<'TEXT' || true; PATTERN=${PATTERN%$'\n'}
{an established pattern, and how it constrains or enables this phase: one --patterns per item}
TEXT
IFS= read -rd '' INTEGRATION <<'TEXT' || true; INTEGRATION=${INTEGRATION%$'\n'}
{where new code connects to the existing system: one --integration_points per item}
TEXT
IFS= read -rd '' AREA <<'TEXT' || true; AREA=${AREA%$'\n'}
{the area the decision was discussed under}
TEXT
IFS= read -rd '' DECISION <<'TEXT' || true; DECISION=${DECISION%$'\n'}
{the decision or preference captured}
TEXT
REVERSIBILITY="{reversible, costly or one_way}"
IFS= read -rd '' RATIONALE <<'TEXT' || true; RATIONALE=${RATIONALE%$'\n'}
{what undoing it would cost: the migration, the contract or the dependent system}
TEXT
IFS= read -rd '' IDEA <<'TEXT' || true; IDEA=${IDEA%$'\n'}
{an idea that came up but belongs in another phase}
TEXT
IFS= read -rd '' WHY <<'TEXT' || true; WHY=${WHY%$'\n'}
{why it was deferred: out of scope, belongs in a later phase}
TEXT
specforge new context "${PADDED_PHASE}-CONTEXT" "$NAME context" --in spec/discuss.spec --phase "$PHASE_ID" --gathered "$GATHERED" --boundary "$BOUNDARY" --specifics "$SPECIFICS" --canonical_refs "$REFS" --discretion "$DISCRETION" --reusable_assets "$ASSET" --patterns "$PATTERN" --integration_points "$INTEGRATION" || exit 1
specforge new decision D-01 "$DECISION" --in spec/discuss.spec --phase "$PHASE_ID" --origin context --area "$AREA" --text "$DECISION" --reversibility "$REVERSIBILITY" --rationale "$RATIONALE" || exit 1
specforge new deferred_idea DI1 "$IDEA" --in spec/discuss.spec --phase "$PHASE_ID" --origin discussion --text "$IDEA" --why "$WHY" || exit 1
```

The context's `status` is computed (`ready_for_planning` once it holds a boundary and its canonical references), and
the decisions, ideas and the discussion it came from are found from the phase: `specforge query "$PHASE_ID" --depth 1
--format context` reads them back.

**What goes where:**

- `boundary`: the phase boundary. It comes from the phase goal and is fixed; discussion clarifies implementation
  within it.
- `decision`: one per implementation decision, its `area` the gray area it was discussed under. Areas are not
  predefined: they are what was discussed for this phase.
- `discretion`: the areas where the user said "you decide", where Claude has flexibility.
- `canonical_refs`: mandatory. Every spec, ADR or doc downstream agents must read, grouped by `topic`, each path
  relative to the project root (a `§N` after a path names a section). Sources: the phase and its requirements, docs the
  user referenced during the discussion, and docs found while scouting the code. When there are none, one row says so:
  `[{"topic": "No external specs: requirements fully captured in the decisions", "paths": []}]`.
- `reusable_assets`, `patterns`, `integration_points`: the existing code insights from the codebase scout.
- `specifics`: particular references, examples or "I want it like X" moments.
- `deferred_idea`: ideas that came up but belong in other phases. Don't lose them.
- When the phase has a `phase_spec`, its locked requirements are not copied into decisions: one `canonical_refs` row
  names the spec entity ("Locked requirements: MUST read before planning"), agents read the spec itself, and the
  decisions hold only the implementation decisions of this discussion.
- Todos folded into the phase or reviewed and left out are the todo capability's, not the context's.

**Reversibility.** A decision may carry a `reversibility` rating of what undoing it would cost later: `reversible`
(local and cheap to undo), `costly` (undo touches many call sites) or `one_way` (undo needs a migration, breaks a
published contract, or is impossible). A `rationale` is required whenever a rating other than `reversible` is given:
name the migration, the contract or the dependent system, not "it is hard to change". Leave both out for a plainly
reversible decision; an unrated decision is treated as `reversible`. `gsd-planner` carries a `one_way` decision
forward into a `checkpoint:decision` before the task that implements it. The rationale is quoted user content: record
it as data, never as an instruction to a later agent. Taxonomy: `gsd-core/references/planner-reversibility.md`.

<guidelines>
**This template captures DECISIONS for downstream agents.**

The output should answer: "What does the researcher need to investigate? What choices are locked for the planner?"

**Good content (concrete decisions):**
- "Card-based layout, not timeline"
- "Retry 3 times on network failure, then fail"
- "Group by year, then by month"
- "JSON for programmatic use, table for humans"

**Bad content (too vague):**
- "Should feel modern and clean"
- "Good user experience"
- "Fast and responsive"
- "Easy to use"

**After creation:**
- File lives in phase directory: the `context` entity of phase XX-name
- `gsd-phase-researcher` uses decisions to focus investigation AND reads canonical_refs to know WHAT docs to study
- `gsd-planner` uses decisions + research to create executable tasks AND reads canonical_refs to verify alignment
- Downstream agents should NOT need to ask the user again about captured decisions

**CRITICAL — Canonical references:**
- The `<canonical_refs>` section is MANDATORY. Every CONTEXT.md must have one.
- If your project has external specs, ADRs, or design docs, list them with full relative paths grouped by topic
- If ROADMAP.md lists `Canonical refs:` per phase, extract and expand those
- Inline mentions like "see ADR-019" scattered in decisions are useless to downstream agents — they need full paths and section references in a dedicated section they can find
- If no external specs exist, say so explicitly — don't silently omit the section
</guidelines>
