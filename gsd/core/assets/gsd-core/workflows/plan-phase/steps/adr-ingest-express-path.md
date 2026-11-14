<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/plan-phase/steps/adr-ingest-express-path.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
## 3.6. Handle ADR Ingest Express Path

**Skip if:** No `--ingest` flag in arguments.

**If `--ingest <path-or-glob>` provided:**

1. Display banner: `GSD ► ADR Ingest Express Path` with `{INGEST_PATH}` and `{INGEST_FORMAT}`.
2. Parse each resolved ADR through `gsd-core/bin/lib/adr-parser.cjs` (`--input`, `--format`) and collect normalized records.
3. Status gate: reject `superseded`/`rejected`/`deprecated`; warn on `proposed`; missing status defaults to `accepted`.
4. Empty-decisions fallback: if all parsed ADRs have zero `decisions[]`, emit `ADR ingest produced no locked decisions; fall back to discuss-phase for this phase.` and exit with `/gsd:discuss-phase {N}` guidance.
5. Record the phase's context as `templates/context.md`'s typed-edit fence does, from the ADR records: its boundary, locked decisions, canonical refs, specifics and deferred ideas, `consequences_positive[]` as success criteria and `consequences_negative[]` as the risk summary in its specifics, which also name `ADR Ingest Express Path ({INGEST_PATH})` as the source.
6. Commit with `git commit -m "docs(${padded_phase}): generate context from ADR ingest"`, then `specforge collect --runner gsd-git --yes` and set `context_content`; continue to step 5.

**Effect:** This bypasses step 4 (loading the context), since the context came from the ADRs.

