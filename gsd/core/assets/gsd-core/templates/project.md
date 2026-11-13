<!-- Ported from gsd-core v1.19.0 gsd-core/templates/project.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Project Template

Template for the `project` entity: the living project context, with its key decisions.

---

## The Project's Typed Edits

A project is entities in `spec/project.spec`: one `project` and one `decision` (origin `project`) per key decision,
written with `specforge new` and tied to the project with `specforge link`; nothing writes a project by hand.
`specforge schema --kind project` (likewise `decision`) lists the fields, and `specforge new project --help` prints them
as flags. `constraints` and `out_of_scope` are JSON lists. Fill the values: free text in its quoted heredoc, as written, and an id, enum or date in its assignment; leave out a flag you have no value
for, and repeat the `new decision` and `link` lines per decision with the next id (`D-02`). A project that already exists
takes `specforge set` / `specforge link` on its fields instead of `new`. A refusal names its code: read it with
`specforge explain <code>`.

```bash
SLUG="{the project's id: its name in kebab-case}"
IFS= read -rd '' NAME <<'TEXT' || true; NAME=${NAME%$'\n'}
{the project's name}
TEXT
IFS= read -rd '' WHAT <<'TEXT' || true; WHAT=${WHAT%$'\n'}
{what this is: 2-3 sentences on what it does and who it is for, in the user's words}
TEXT
IFS= read -rd '' CORE_VALUE <<'TEXT' || true; CORE_VALUE=${CORE_VALUE%$'\n'}
{the one thing that must work if everything else fails}
TEXT
IFS= read -rd '' CONTEXT <<'TEXT' || true; CONTEXT=${CONTEXT%$'\n'}
{background that informs implementation: environment, prior work, user feedback, known issues}
TEXT
IFS= read -rd '' CONSTRAINTS <<'TEXT' || true; CONSTRAINTS=${CONSTRAINTS%$'\n'}
[{"kind": "{tech_stack, timeline, budget, dependencies, compatibility, performance or security}", "what": "{the limit}", "why": "{why it holds}"}]
TEXT
IFS= read -rd '' OUT_OF_SCOPE <<'TEXT' || true; OUT_OF_SCOPE=${OUT_OF_SCOPE%$'\n'}
[{"item": "{what is not built}", "why": "{why it is excluded}"}]
TEXT
DECISION_ID="{D-NN: D-01 for a new project, else the next free decision id}"
IFS= read -rd '' DECISION <<'TEXT' || true; DECISION=${DECISION%$'\n'}
{a choice made during questioning}
TEXT
IFS= read -rd '' RATIONALE <<'TEXT' || true; RATIONALE=${RATIONALE%$'\n'}
{why it was chosen}
TEXT
specforge new project "$SLUG" "$NAME" --in spec/project.spec --name "$NAME" --what "$WHAT" --core_value "$CORE_VALUE" --context "$CONTEXT" --constraints "$CONSTRAINTS" --out_of_scope "$OUT_OF_SCOPE" || exit 1
specforge new decision "$DECISION_ID" "$DECISION" --in spec/project.spec --origin project --text "$DECISION" --rationale "$RATIONALE" --outcome pending || exit 1
specforge link "$SLUG" key_decisions "$DECISION_ID" || exit 1
```

The project's requirements are `requirement` entities (`templates/requirements.md`), not fields of the project: their
status (active, validated) is computed from the phases and verifications that cover them. `specforge export --kinds
project,decision --format context` reads the project and its decisions back.

**What goes where:**

- `what`: the current, accurate description of the product, 2-3 sentences on what it does and who it is for. Use the
  user's words and framing; update it when the product evolves beyond it.
- `core_value`: the single most important thing. Everything else can fail; this cannot. It drives prioritization when
  tradeoffs arise and rarely changes: when it does, it is a significant pivot.
- `customer`, `revenue_model`, `success_metric`: optional, only for monetized or customer-facing projects, one line
  each (`--customer`, `--revenue_model`, `--success_metric`). A constraint reference, not a business plan; they inform
  requirement prioritization (features serving the customer or revenue model come first). Leave them out for internal
  tools, experiments or meta workspaces.
- `context`: background that informs implementation decisions: technical environment, prior work, user feedback, known
  issues or technical debt. Update it as new context emerges.
- `constraints`: hard limits on implementation choices, each with its `kind`, `what` and `why`. Include the why:
  constraints without rationale get questioned.
- `out_of_scope`: explicit boundaries on what is not built, each with its reason (it prevents re-adding later):
  considered and rejected, deferred, or explicitly excluded.
- `decision`: one per significant choice that affects future work, added as it is made throughout the project. Its
  `outcome` tracks what became of it: `good` (proved correct), `revisit` (may need reconsideration) or `pending` (too
  early to evaluate).

**Evolution.** The project changes at phase transitions and milestone boundaries, through typed edits on its fields
(`workflows/transition.md`, `workflows/complete-milestone.md`): after a phase, log new decisions and check `what` still
holds; after a milestone, review every field, check the core value is still the right priority, audit `out_of_scope`'s
reasons and update `context` with the current state.

**Brownfield.** For an existing codebase, onboard or map it first (`/gsd:onboard`, `/gsd:map-codebase`): the
`@gsd/codebase` map records what the code already does, and its requirements are recorded as `requirement` entities
like any other. Present the inferred current state, ask what the user wants to build next, and write `context` to
include the current codebase state.
