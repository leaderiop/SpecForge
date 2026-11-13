<!-- Ported from gsd-core v1.19.0 gsd-core/templates/requirements.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Requirements Template

Template for the `requirement` entities: checkable requirements that define "done."

---

## The Requirements' Typed Edits

Requirements are entities in `spec/requirements.spec`, one `requirement` per requirement, written with `specforge new`;
nothing writes a requirements list by hand. `specforge schema --kind requirement` lists the fields, and `specforge new
requirement --help` prints them as flags. Fill the quoted values and repeat the line per requirement; `--reason` is for
an `out_of_scope` requirement, and is left out otherwise. A refusal names its code: read it with `specforge explain
<code>`.

```bash
REQ_ID="{CATEGORY-NN, e.g. AUTH-01}"
TEXT="{the requirement: user-centric, testable, atomic}"
CATEGORY="{its category, e.g. Authentication}"
TIER="{v1, v2 or out_of_scope}"
REASON="{why it is out of scope}"
specforge new requirement "$REQ_ID" "$TEXT" --in spec/requirements.spec --text "$TEXT" --category "$CATEGORY" --tier "$TIER" --reason "$REASON" || exit 1
```

A `v1` requirement must be in a phase: `gsd/E101` binds once the project has a phase, so requirements recorded before
the roadmap wait for it, and the roadmapper maps each `v1` requirement to exactly one phase (the phase's
`requirements`). Coverage, traceability and status are computed from the phases, plans and verifications that cover a
requirement: `specforge query <id> --depth 1 --format context` reads them back.

**What goes where:**

- `id`: `CATEGORY-NN` (AUTH-01, CONTENT-02, SOCIAL-03).
- `text`: user-centric, testable, atomic ("User can reset password via email link").
- `category`: derived from the research's feature categories and kept consistent with domain conventions. Typical:
  Authentication, Content, Social, Notifications, Moderation, Payments, Admin.
- `tier`:
  - `v1`: committed scope, mapped to roadmap phases;
  - `v2`: acknowledged but deferred, not in the current roadmap; moving one to `v1` needs a roadmap update;
  - `out_of_scope`: explicitly excluded, with its `reason`: it prevents "why didn't you include X?" later.
    Anti-features from research belong here with their warnings.

**Evolution.** After a phase completes its verification marks the covered requirements complete; after a roadmap
update, every `v1` requirement is still mapped, new requirements are recorded if scope expanded, and descoped ones move
to `v2` or `out_of_scope` (`specforge set <id> tier …`). A requirement is complete when its feature is implemented,
verified and committed.
