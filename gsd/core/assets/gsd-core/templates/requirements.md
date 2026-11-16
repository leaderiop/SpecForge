<!-- Ported from gsd-core v1.19.0 gsd-core/templates/requirements.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Requirements Template

Template for the `requirement` entities: checkable requirements that define "done."

---

## The Requirements' Typed Edits

Requirements are entities in `spec/requirements.spec`, one `requirement` per requirement, written with `specforge new`;
nothing writes a requirements list by hand. `specforge schema --kind requirement` lists the fields, and `specforge new
requirement --help` prints them as flags. Fill the values: free text, and the metric's JSON record (its name is free
text), in its quoted heredoc, as written, and an id, enum or date in its assignment, and repeat the line per requirement. Leave out the flag of a value it does not
have: `--reason` is for an `out_of_scope` requirement only, and a requirement with no persona, metric or source yet
drops `--personas`, `--metric` or `--source`. A list flag (`--personas`, `--source`) takes one item: repeat it per
item. A refusal names its code: read it with `specforge explain <code>`.

```bash
REQ_ID="{AREA-NN, e.g. AUTH-01}"
IFS= read -rd '' TEXT <<'TEXT' || true; TEXT=${TEXT%$'\n'}
{the requirement: user-centric, testable, atomic}
TEXT
IFS= read -rd '' AREA <<'TEXT' || true; AREA=${AREA%$'\n'}
{its area, e.g. Authentication}
TEXT
TIER="{v1, v2 or out_of_scope}"
CATEGORY="{its quality category, e.g. security}"
PRIORITY="{must, should or could}"
PERSONA="{the id of a recorded persona it serves}"
IFS= read -rd '' METRIC <<'TEXT' || true; METRIC=${METRIC%$'\n'}
{the measure it is held to, one JSON record, e.g. {"name": "p95 sign-in", "threshold": 0.5, "unit": "s", "direction": "at_most"}}
TEXT
IFS= read -rd '' SOURCE <<'TEXT' || true; SOURCE=${SOURCE%$'\n'}
{where it came from: a person's name, or a declared ref such as gh.issue:42}
TEXT
IFS= read -rd '' REASON <<'TEXT' || true; REASON=${REASON%$'\n'}
{why it is out of scope}
TEXT
specforge new requirement "$REQ_ID" "$TEXT" --in spec/requirements.spec --text "$TEXT" --area "$AREA" --tier "$TIER" --category "$CATEGORY" --priority "$PRIORITY" --personas "$PERSONA" --metric "$METRIC" --source "$SOURCE" --reason "$REASON" || exit 1
```

A `v1` requirement must be in a phase: `gsd/E101` binds once the project has a phase, so requirements recorded before
the roadmap wait for it, and the roadmapper maps each `v1` requirement to exactly one phase (the phase's
`requirements`). Coverage, traceability and status are computed from the phases, plans and verifications that cover a
requirement: `specforge query <id> --depth 1 --format context` reads them back.

**What goes where:**

- `id`: `AREA-NN` (AUTH-01, CONTENT-02, SOCIAL-03), upstream's `[CATEGORY]-[NUMBER]`.
- `text`: user-centric, testable, atomic ("User can reset password via email link").
- `area`: upstream's `### [Category]` heading: a feature area from the research, in the words the domain uses
  (an auth, content or payments area, say).
- `tier`:
  - `v1`: committed scope, mapped to roadmap phases;
  - `v2`: acknowledged but deferred, not in the current roadmap; moving one to `v1` needs a roadmap update;
  - `out_of_scope`: explicitly excluded, with its `reason`: it prevents "why didn't you include X?" later.
    The research's anti-features land here as well, each keeping its warning.
- `category` (optional): its quality class, one of `functional`, `performance`, `compatibility`, `usability`,
  `reliability`, `security`, `maintainability`, `portability`, `safety` or `regulatory` (the first eight are ISO/IEC
  25010:2011's product quality characteristics by their short names; `safety` comes from its 2023 revision, which
  also renamed usability to interaction capability and portability to flexibility).
- `priority` (optional): `must`, `should` or `could`; a requirement nobody wants is `out_of_scope`, not "won't".
- `personas` (optional, `--personas <id>` per persona): the ids of recorded personas it serves (`specforge search ""
  --kinds persona` lists them).
- `metric` (optional): the measure it is held to, one JSON record: `name`, a `threshold` number, its `unit` when it
  has one (a count has none) and a `direction` (`at_most`, `at_least` or `exactly`), e.g.
  `{"name": "p95 sign-in", "threshold": 0.5, "unit": "s", "direction": "at_most"}`.
- `source` (optional, `--source <text>` per source): where it came from: a person's name, or a declared `ref` such as
  `gh.issue:42`.
- `constrains` (`--constrains <id>`), `protects` (`--protects <id>`), `tags` (`--tags <word>`) and `effort`
  (`--effort xs|s|m|l|xl`), each optional and left out of the line above when unknown: the requirements it limits,
  the invariants it guards, words to group it by, and a T-shirt estimate. Add them to the line, or set them later
  (`specforge link <id> constrains <other>`, `specforge set <id> effort m`).

**Evolution.** After a phase completes its verification marks the covered requirements complete; after a roadmap
update, every `v1` requirement is still mapped, new requirements are recorded if scope expanded, and descoped ones move
to `v2` or `out_of_scope` (`specforge set <id> tier …`). A requirement is complete when its feature is implemented,
verified and committed.
