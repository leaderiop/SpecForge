<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/execute-phase/steps/threat-id-gate.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
Apply response_language to all user-facing prose — narration between tool calls, status updates, progress notes, and findings included; preserve code, paths, and identifiers.

<step name="threat_id_gate">
Cross-plan threat-ID duplicates — a `T-{phase}-NN` ID claimed by more than one live PLAN file in
this phase (#4683). The reserved `T-{phase}-SC` supply-chain row is never listed: every plan keeps
it by design.

This is a hard stop BEFORE any dispatch — do not spawn executors, do not update state, do not
write artifacts. A reused ID names two different threats, so `SECURITY.md` rows and
`VALIDATION.md`'s Threat Ref column are ambiguous until it is fixed; executing the phase would
silently mark the wrong threat.

Report the full `threat_id_duplicates` list verbatim — each ID with its claiming plan files:

```
### GSD ► THREAT ID DUPLICATES — EXECUTION BLOCKED (#4683)

{for each {id, plans}: "{id} — claimed by {plans.join(', ')}"}

Threat IDs must be unique within a phase. Renumber the newer plans' registers to continue
after the phase's highest in-use `T-{phase}-NN` (see @~/.claude/gsd-core/references/planner-gap-closure.md §9),
then re-run /gsd:execute-phase {phase}.
```

Do not offer to execute anyway. The only forward paths are renumbering the colliding registers
or (if a plan was superseded after the check, so it is removed from the spec) re-running `specforge gsd threat-ids "$PHASE_ID"` so the stale plan drops out of the
scan — superseded plans never hold IDs against their replacements (#2349).
</step>
