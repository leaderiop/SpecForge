<!-- Ported from gsd-core v1.19.0 gsd-core/templates/roadmap.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Roadmap Template

Template for a `milestone` and its `phase` entities: the roadmap is entities, written in one edit.

---

## The Roadmap Edit

The roadmap is written with one MCP `specforge.edit` call whose `edits` list is the JSON below: all of it is written, or
none of it. A v1 requirement must be in exactly one phase of the project (`gsd/E101`), and that binds from the first
phase on, so the phases are never written one by one: a phase written alone would leave the other v1 requirements in no
phase, and the edit is refused whole. Fill the quoted values and repeat a `phase` op per phase (a `requirement` op per
new requirement); `specforge schema --kind phase` (likewise `milestone`, `requirement`) lists the fields. Leave out the
`milestone` op when the milestone exists (`/gsd-new-milestone` records it first), and the `requirement` ops when the
requirements exist (`/gsd-new-project` records them at its step 7). In `/gsd-ingest-docs` the list also carries the `project`, its decisions and the requirements as
create ops ahead of the milestone (their fields: `templates/project.md`, `templates/requirements.md`). A refusal names its codes: read one with
`specforge explain <code>`, fix the list and send the whole call again.

```json
[
  {"op": "create", "kind": "milestone", "id": "{the milestone version, e.g. v1.1}", "title": "{the milestone name}", "file": "spec/{the milestone version, e.g. v1.1}.spec", "fields": {"name": "{the milestone name}", "goal": "{the milestone goal}"}},
  {"op": "create", "kind": "requirement", "id": "{the first phase's requirement, e.g. SYNC-02}", "title": "{the first requirement's text}", "file": "spec/requirements.spec", "fields": {"text": "{the first requirement's text}", "category": "{the requirements' category}", "tier": "v1"}},
  {"op": "create", "kind": "requirement", "id": "{the second phase's requirement, e.g. SYNC-03}", "title": "{the second requirement's text}", "file": "spec/requirements.spec", "fields": {"text": "{the second requirement's text}", "category": "{the requirements' category}", "tier": "v1"}},
  {"op": "create", "kind": "phase", "id": "{the first phase's number, e.g. 04}", "title": "{the first phase's name}", "file": "spec/{the milestone version, e.g. v1.1}.spec", "fields": {"milestone": "{the milestone version, e.g. v1.1}", "name": "{the first phase's name}", "slug": "{the first phase's slug}", "goal": "{the first phase's goal}", "success_criteria": ["{an observable behavior the first phase makes true}", "{another observable behavior of the first phase}"], "requirements": ["{the first phase's requirement, e.g. SYNC-02}"], "mode": "{standard or mvp}"}},
  {"op": "create", "kind": "phase", "id": "{the second phase's number, e.g. 05}", "title": "{the second phase's name}", "file": "spec/{the milestone version, e.g. v1.1}.spec", "fields": {"milestone": "{the milestone version, e.g. v1.1}", "name": "{the second phase's name}", "slug": "{the second phase's slug}", "goal": "{the second phase's goal}", "success_criteria": ["{an observable behavior the second phase makes true}", "{another observable behavior of the second phase}"], "depends_on": ["{the first phase's number, e.g. 04}"], "requirements": ["{the second phase's requirement, e.g. SYNC-03}"], "mode": "{standard or mvp}"}}
]
```

A phase id is its number within the milestone (`04`), and a `depends_on` names the phases it needs by number; the
milestone scopes them (`v1.1/04`). In a workstream the milestone's id is
`<workstream>/<version>` (`api/v1.1`, which `specforge new milestone --workstream` gives it), and each phase's
`milestone` names that id. `mode` is `mvp` for a vertical-MVP project (`PROJECT_MODE=mvp`) and `standard`
otherwise. A phase's status, its plans and the milestone's progress are computed: `specforge gsd progress-report`
reads them back, and nothing writes a progress table.

**What goes where:**

- Phase count depends on the granularity setting (coarse: 3-5, standard: 5-8, fine: 8-12). Each phase delivers
  something coherent; a phase can have 1+ plans (split if >3 tasks or multiple subsystems). No time estimates.
- Integer phases are planned milestone work; a decimal phase (`02.1`) is an urgent insertion between its neighbours
  (`/gsd-phase --insert`). Numbering continues across milestones: never restart at 01.
- `success_criteria`: 2-5 observable behaviors per phase from the user's perspective ("User can [action]" or "[Thing]
  works/exists"), cross-checked against the requirements while the roadmap is created. They flow downstream to the
  plans' `must_haves` and are verified after execution.
- `requirements`: every v1 requirement in exactly one phase; a requirement no phase can hold moves to v2
  (`specforge set <id> tier v2`).
