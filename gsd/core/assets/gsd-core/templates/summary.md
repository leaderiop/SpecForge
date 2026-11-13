<!-- Ported from gsd-core v1.19.0 gsd-core/templates/summary.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Summary Template

Template for the `summary` entity of phase XX-name - phase completion documentation.

---

## The Summary's Typed Edits

A plan's summary is entities in the plan's file: one `summary`, one `deliverable` per shipped deliverable and one
`deviation` per deviation or issue, written with `specforge new`; nothing writes a summary by hand.
`specforge schema --kind summary` (likewise `deliverable`, `deviation`) lists the fields, and `specforge new summary
--help` prints them as flags. A list of ids is a comma list (`--requirements_completed AUTH-01,AUTH-02`), a list of
texts takes its flag once per item (`--tags jose --tags prisma`), and a block is JSON (`--requires '<json>'`,
`--checks '<json>'`). Fill the quoted values; leave out a flag you have no value for (no `KEY_DECISIONS`, no
`THREAT_FLAG`, no `RATIONALE` when a person need not judge), and repeat a `new deliverable` or `new deviation` line per
item with the next id (`D2`, `DV2`). A plan that stopped by design takes `--status halted --halted_because "<why>"`.
Never write `started`, `completed`, `duration`, `actuals`, `files_created` or `files_modified`: `complete-plan` writes
the times, and the rest are computed from the plan's commits. A refusal names its code: read it with
`specforge explain <code>`.

```bash
PLAN_ID="{the plan's id, e.g. v1.0/02/02-01}"
TITLE="{the plan's name}: summary"
STATUS="{complete, or halted when the plan stopped by design}"
ONE_LINER="{one substantive line: what actually shipped}"
SUBSYSTEM="{the primary category: auth, payments, ui, api, database, infra, testing}"
TAG="{a searchable technology: one --tags per tag}"
REQUIRES='[{"phase": "{a prior phase id}", "provides": "{what that phase built that this plan uses}"}]'
PROVIDES="{what this plan built: one --provides per item}"
AFFECTS="{a phase or keyword that will need this context: one --affects per item}"
TECH_ADDED="{a library or tool added: one --tech_added per item}"
TECH_PATTERN="{an architectural or code pattern established: one --tech_patterns per item}"
KEY_DECISIONS="{the decision ids this plan took, comma-separated}"
PATTERN="{a convention future phases should keep: one --patterns_established per item}"
REQUIREMENTS_COMPLETED="{the plan's requirements, verbatim, comma-separated}"
THREAT_FLAG="{threat_flag: type | file | the new surface the plan's threat model lacks}"
NEXT_READINESS="{what is ready for the next phase, and any blocker or concern}"
DELIVERABLE="{a deliverable in human-readable form}"
REQUIREMENT="{the requirement id it joins, from the plan's requirements}"
HUMAN_JUDGMENT="{false when passing checks prove it, true when a person signs it off}"
RATIONALE="{why automation is insufficient}"
CHECKS='[{"kind": "{unit, integration, e2e, automated_ui, manual_procedural or other}", "ref": "{path#test name}"}]'
DEVIATION_KIND="{deviation, or issue for a problem met during planned work}"
RULE="{the rule and category, e.g. Rule 1 - Bug}"
DEVIATION_TEXT="{what was wrong, what was done, and how it was verified}"
DEVIATION_FILE="{a path it touched: one --files per path}"
SUMMARY="${PLAN_ID##*/}-SUMMARY"
SUMMARY_ID="$PLAN_ID/$SUMMARY"
PLAN_FILE=$(specforge query "$PLAN_ID" --depth 0 --pick '.nodes[0].file') || exit 1
specforge new summary "$SUMMARY" "$TITLE" --in "$PLAN_FILE" --plan "$PLAN_ID" --status "$STATUS" --one_liner "$ONE_LINER" --subsystem "$SUBSYSTEM" --tags "$TAG" --requires "$REQUIRES" --provides "$PROVIDES" --affects "$AFFECTS" --tech_added "$TECH_ADDED" --tech_patterns "$TECH_PATTERN" --key_decisions "$KEY_DECISIONS" --patterns_established "$PATTERN" --requirements_completed "$REQUIREMENTS_COMPLETED" --threat_flags "$THREAT_FLAG" --next_readiness "$NEXT_READINESS" || exit 1
specforge new deliverable D1 "$DELIVERABLE" --in "$PLAN_FILE" --summary "$SUMMARY_ID" --description "$DELIVERABLE" --requirement "$REQUIREMENT" --human_judgment "$HUMAN_JUDGMENT" --rationale "$RATIONALE" --checks "$CHECKS" || exit 1
specforge new deviation DV1 "$RULE" --in "$PLAN_FILE" --summary "$SUMMARY_ID" --kind "$DEVIATION_KIND" --rule "$RULE" --text "$DEVIATION_TEXT" --files "$DEVIATION_FILE" || exit 1
specforge gsd complete-plan "$SUMMARY_ID" || exit 1
```

`complete-plan` closes the plan: it writes the summary's `started` (the start `specforge gsd start-plan` recorded)
and `completed` (the host's clock), and is refused while the summary lacks a required field.

<frontmatter_guidance>
**Dependency graph:** `requires`/`provides`/`affects` create explicit links between phases, enabling transitive closure for context selection.

**Subsystem:** Primary categorization (auth, payments, ui, api, database, infra, testing) for detecting related phases.

**Tags:** Searchable technical keywords (libraries, frameworks, tools) for tech stack awareness.

**Patterns:** Established conventions future phases should maintain.

**Status (#2830):** `status: complete` is the default — the plan finished. Use `status: halted` instead when the plan reached a designed stop (a gate failure, a spike concluding without expanding into the full build, or any other intentional non-completion) and intentionally left tasks unfinished. `halted` is machine-read: any plan whose `depends_on` (directly or transitively) names a halted plan is reported as blocked, not offered to the executor, until the halt is resolved and re-summarized as `complete`.
</frontmatter_guidance>

<coverage_guidance>
**Purpose (#1602):** The `coverage:` block is a per-deliverable Requirements Traceability Matrix. It lets `verify-work`'s `extract_tests` step route deliverables DETERMINISTICALLY — auto-passing those proven by passing tests and reserving human UAT for genuine judgment — instead of re-deriving coverage from prose. Consumed via `specforge check` (it reads the summary's deliverables).

The coverage block is the summary's `deliverable` entities, and a deliverable's `checks` are its `verification`
entries (`kind`, and `ref` as `path#test name`); a check's `status` is computed from the latest run, never
written.

**Deterministic contract (what the classifier does):**
- A deliverable auto-passes (no human prompt) **only** when `human_judgment: false` AND `verification` is non-empty AND every `verification[].status` is `pass`. This is the narrow, fully-proven case.
- **Everything else is presented to a human** — `human_judgment: true`, an empty `verification:`, any non-`pass`/`unknown` status, or any schema error. A false-negative is a redundant prompt (the status quo); a false-positive ships a bug UAT existed to catch.
- **Fail-safe default:** if you cannot determine coverage for a deliverable, you MUST set `human_judgment: true` with `rationale: "Coverage not determined at authoring time — verifier must classify"`. Never leave a deliverable's `human_judgment` empty, and never set it `false` just to skip the prompt — auto-pass additionally requires a passing `verification` entry, so the flag alone cannot skip the human.
</coverage_guidance>

<one_liner_rules>
The one-liner MUST be substantive:

**Good:**
- "JWT auth with refresh rotation using jose library"
- "Prisma schema with User, Session, and Product models"
- "Dashboard with real-time metrics via Server-Sent Events"

**Bad:**
- "Phase complete"
- "Authentication implemented"
- "Foundation finished"
- "All tasks done"

The one-liner should tell someone what actually shipped.
</one_liner_rules>

<guidelines>
**One-liner:** Must be substantive. "JWT auth with refresh rotation using jose library" not "Authentication implemented".

**Decisions:** each key decision the plan took is a `decision` entity the summary's `key_decisions` names (with
`--origin summary`), or none when the plan was followed as specified.
</guidelines>
