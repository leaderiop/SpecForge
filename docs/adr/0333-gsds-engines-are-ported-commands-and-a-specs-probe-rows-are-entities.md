# GSD's engines are ported commands, and a spec's probe rows are entities

**Status:** accepted (2026-11-12, R3 of the Beyond Parity build plan). **Amends**
[ADR 0343](0343-a-gsd-block-calls-only-specforge.md) D2's glue example (the requirements heredoc is gone; the
regression gate's run step stays the example). Builds on
[ADR 0297](0297-a-judged-rules-finding-is-not-an-error-a-write-adds.md) (E080),
[ADR 0284](0284-a-name-is-unique-within-its-scope.md) (a reference resolves in its scope) and
[ADR 0343](0343-a-gsd-block-calls-only-specforge.md).

## Context

- `/gsd-spec-phase` Step 5.5 (the edge-completeness probe) looked for upstream's compiled `edge-probe.cjs` under
  `RUNTIME_DIR`, the git top and `$HOME/.claude`, and exited 1 when none was there. SpecForge never creates one, so
  the step always stopped.
- `/gsd-plan-phase` step 7.95 ran the spec-less probe fallback's gate, which looked for `spec-section.cjs` the same way
  and exited 1 before it read its own toggle.
- Only the input guard was ported (`gsd edge-probe-requirements`). Upstream's three sources (`src/edge-probe.cts`,
  `src/probe-core.cts`, `src/spec-section.cts`) were not in the snapshot, and the parity roster pointed both engines at
  a pass that ports neither.
- No kind held a spec's edge coverage or its prohibitions: even a working engine had nowhere to record its answer.

## Decisions

- **D1. An engine a GSD step runs with `node` is a `specforge gsd` command.** The engine is ported to pure Rust in the
  extension (`gsd/core/src/probe/`), its source and unit suites are vendored into the snapshot, and its cases are
  ported and named against the vendored suites. It is never resolved from an install directory and never stopgapped.
- **D2. A SPEC section's rows are entities scoped by the spec.** `edge` holds one row of `## Edge Coverage` and
  `spec_prohibition` one row of `## Prohibitions`, each `.scoped_by("spec")` and pointing at a `locked_req` of that
  spec. A section is "supplied" when the spec holds a row of its kind; `specforge query` counts them, so upstream's
  `spec-section` helper is the host's.
- **D3. gsd/E523 holds a row's lifecycle, and a resolution is one edit.** A resolved row carries its tier (an edge
  also its resolution), a dismissed row its reason, an unresolved row neither; the requirement is a locked
  requirement of the row's own spec; a spec holds one edge per requirement and category. E080 refuses a write that
  adds an error, so moving a row field by field fails at its first field: `specforge gsd resolve-edge` and
  `resolve-prohibition` write the whole resolution in one plan. `resolve-edge` keys a row by its requirement and
  category, upstream's resolution key, and creates it when the spec has none, so an agent never picks an id.
- **D4. The classifier reads the requirement as the SPEC shows it.** A locked requirement's text is its `title`,
  `current`, `target` and `acceptance`, and its `text_en` is classified first when present. Authored shapes are the
  enum list `shapes` on `locked_req` and `requirement` (`numeric_range`, …), mapped to upstream's names. An edge whose
  pair the engine did not propose (an agent-raised row) is an item of the report, not an orphan: its category is a
  kernel enum, so the typo the orphan check guards against is already a type error.
- **D5. The cues run as upstream's JavaScript regex text.** `SHAPE_CUES` are upstream's sources between the slashes,
  read by the one JavaScript regex reader of the crate (`gsd/core/src/js_regex.rs`, the injection hooks' matcher),
  which learns `\b` and `\B`. A test pins each cue and the taxonomy to the vendored `.cts`.

Rejected:

- Widening `prohibition` with a spec owner. A kind has one scope field and `prohibition`'s is `plan`; a spec-owned row
  would fall into the project scope, where its name collides across specs, and every plan prohibition would gain a
  lifecycle upstream's `must_haves.prohibitions` does not have.
- A `--record` flag that writes the proposals as unresolved edges. It would be a second writer of each row, and
  `specforge new edge` on a recorded pair is E523; the report already counts an unrecorded proposal as unresolved.
- Translating the cues to the `regex` crate's syntax: a second reader of the same text, and a paraphrase of upstream's.
- Gating the two steps off until the engines were ported.

## Consequences

- `/gsd-spec-phase` records the phase spec and its locked requirements before Step 5.5, and the probe reads them from
  the graph; no requirements JSON is written.
- `specforge gsd edge-probe-requirements` is removed; `specforge gsd edge-probe <phase_spec|phase>` answers upstream's
  coverage report, merging a spec's recorded edges.
- Plan-phase 7.95 judges a section supplied by the spec's `edge` and `spec_prohibition` entities, not by a SPEC file's
  headings.
- "Probe" in `CONTEXT.md` stays the checker effect; GSD's edge probe is named by its command.
