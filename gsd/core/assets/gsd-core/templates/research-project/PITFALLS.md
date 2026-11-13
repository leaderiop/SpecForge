<!-- Ported from gsd-core v1.19.0 gsd-core/templates/research-project/PITFALLS.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Pitfalls Research Template

Template for the pitfalls dimension of the project research: common mistakes to avoid in the project domain.

---

## The Pitfalls' Typed Edits

The pitfalls are `pitfall` items of the project research `RESEARCH` in `spec/research.spec`, written with `specforge new` and scoped by `--research RESEARCH`: nothing writes a research file by hand. The orchestrator recorded `RESEARCH` before spawning the researchers; the synthesizer sets its summary. `specforge schema --kind pitfall` lists the fields. Fill the quoted values; leave out a flag you have no value for, and repeat the lines per item with the next free id (`specforge query RESEARCH --depth 1 --format context` lists the items already recorded). A refusal names its code: read it with `specforge explain <code>`.

```bash
NAME="{the pitfall}"
GOES_WRONG="{what goes wrong}"
WHY="{why it happens}"
AVOID="{how to avoid it, and the phase that should prevent it}"
WARNING_SIGNS="{the early signs that it is happening}"
specforge new pitfall PF1 "$NAME" --in spec/research.spec --research RESEARCH --name "$NAME" --goes_wrong "$GOES_WRONG" --why "$WHY" --avoid "$AVOID" --warning_signs "$WARNING_SIGNS" || exit 1
```

**What goes where:** one pitfall per mistake:

- Critical pitfalls: domain-specific issues, not generic mistakes. Include warning signs (early detection prevents
  disasters) and name the phase that prevents it in `avoid`: it makes the pitfall actionable and informs phase ordering
  and success criteria.
- Technical debt: be realistic, some shortcuts are acceptable. Note when a shortcut is never acceptable or only in MVP,
  and the long-term cost.
- Performance traps: include scale thresholds ("breaks at 10k users"), relevant to this project's expected scale; don't
  over-engineer for hypothetical scale.
- Security mistakes: beyond OWASP basics, the domain-specific issues (a community platform has other concerns than
  e-commerce), with their risk level.
- "Looks done but isn't": what to verify during execution, common in demos versus production.
