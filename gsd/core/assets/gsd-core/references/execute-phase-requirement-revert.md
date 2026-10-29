<!-- Ported from gsd-core v1.18.0 gsd-core/references/execute-phase-requirement-revert.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Revert this phase's own requirement IDs out of `Complete` before rendering the gap report (#2388).** A shared requirement ID can already read `Complete` at this point (its first-declaring plan finished before this verification ran) — a `gaps_found` verdict must not leave that premature `Complete` sitting in REQUIREMENTS.md. Scoped strictly to `PHASE_REQ_IDS` (this phase's own citations from `init.execute-phase`), so another phase's `Complete` row is never touched:

```bash
if [ -n "${PHASE_REQ_IDS}" ]; then
  : # requirements are computed: a requirement is complete when a summary lists it, so there is nothing to revert
  git add spec && git commit -m "docs(phase-{X}): revert premature Complete requirements after gaps found" && specforge collect --runner gsd-git --yes
fi
```
