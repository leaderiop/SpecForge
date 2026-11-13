<!-- Ported from gsd-core v1.19.0 gsd-core/templates/research-project/ARCHITECTURE.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Architecture Research Template

Template for the architecture dimension of the project research: system structure patterns for the project domain.

---

## The Architecture's Typed Edits

The architecture is `pattern` items and `finding` items (topic `architecture`) of the project research `RESEARCH` in `spec/research.spec`, written with `specforge new` and scoped by `--research RESEARCH`: nothing writes a research file by hand. The orchestrator recorded `RESEARCH` before spawning the researchers; the synthesizer sets its summary. `specforge schema --kind pattern` (likewise `finding`) lists the fields. An anti-pattern is a `pattern` with `--anti true`, and `--example` holds a code example. Fill the values: free text in its quoted heredoc, as written, and an id, enum or date in its assignment; leave out a flag you have no value for, and repeat the lines per item with the next free id (`specforge query RESEARCH --depth 1 --format context` lists the items already recorded). A refusal names its code: read it with `specforge explain <code>`.

```bash
IFS= read -rd '' PATTERN <<'TEXT' || true; PATTERN=${PATTERN%$'\n'}
{the pattern's name}
TEXT
IFS= read -rd '' WHAT <<'TEXT' || true; WHAT=${WHAT%$'\n'}
{what it is, and its trade-offs}
TEXT
IFS= read -rd '' WHEN <<'TEXT' || true; WHEN=${WHEN%$'\n'}
{when to use it, and when it is overkill}
TEXT
IFS= read -rd '' FINDING <<'TEXT' || true; FINDING=${FINDING%$'\n'}
{a structural finding: the major components and their boundaries, the data flow, the project structure, or what breaks first at scale}
TEXT
CONFIDENCE="{high, medium or low}"
specforge new pattern P1 "$PATTERN" --in spec/research.spec --research RESEARCH --name "$PATTERN" --what "$WHAT" --when "$WHEN" || exit 1
specforge new finding ARCH1 "$FINDING" --in spec/research.spec --research RESEARCH --topic architecture --text "$FINDING" --confidence "$CONFIDENCE" || exit 1
```

**What goes where:**

- System overview (a `finding`): the major components and their relationships. Don't over-detail: this is conceptual,
  not implementation.
- Project structure (a `finding`): name the folders and why things are grouped as they are, laid out the way the
  chosen stack lays out its projects.
- Patterns (`pattern`): include code examples where helpful, explain trade-offs honestly, and note when a pattern is
  overkill for small projects.
- Scaling considerations (a `finding`): be realistic, most projects don't need to scale to millions. Focus on what
  breaks first, not theoretical limits, and avoid premature optimization recommendations.
- Anti-patterns (`pattern`, `anti true`): specific to this domain, with what to do instead.
