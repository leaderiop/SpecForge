<!-- Ported from gsd-core v1.19.0 gsd-core/templates/research-project/FEATURES.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Features Research Template

Template for the features dimension of the project research: the feature landscape for the project domain.

---

## The Features' Typed Edits

The features are `finding` items (topic `features`) of the project research `RESEARCH` in `spec/research.spec`, written with `specforge new` and scoped by `--research RESEARCH`: nothing writes a research file by hand. The orchestrator recorded `RESEARCH` before spawning the researchers; the synthesizer sets its summary. `specforge schema --kind finding` lists the fields. Fill the quoted values; leave out a flag you have no value for, and repeat the lines per item with the next free id (`specforge query RESEARCH --depth 1 --format context` lists the items already recorded). A refusal names its code: read it with `specforge explain <code>`.

```bash
TEXT="{the feature, and whether it is table stakes, a differentiator or an anti-feature}"
CONFIDENCE="{high, medium or low}"
specforge new finding FEAT1 "$TEXT" --in spec/research.spec --research RESEARCH --topic features --text "$TEXT" --confidence "$CONFIDENCE" || exit 1
```

**What goes where:** one finding per feature, its text naming its category:

- Table stakes: non-negotiable for launch. Users don't give credit for having them, but penalize for missing them (a
  community platform without user profiles is broken).
- Differentiators: where the product competes. They align with the project's core value; don't try to differentiate on
  everything.
- Anti-features: prevent scope creep by recording what seems good but isn't, with the alternative approach ("real-time
  everything" often creates complexity without value).
- Feature dependencies: critical for roadmap phase ordering. If A requires B, B must be in an earlier phase; conflicts
  inform what not to combine in the same phase.
- MVP definition: be ruthless about what's truly minimum. "Nice to have" is not MVP: launch with less, validate, then
  expand.
