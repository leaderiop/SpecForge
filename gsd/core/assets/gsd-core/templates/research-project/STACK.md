<!-- Ported from gsd-core v1.19.0 gsd-core/templates/research-project/STACK.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Stack Research Template

Template for the stack dimension of the project research: the recommended technologies for the project domain.

---

## The Stack's Typed Edits

The stack is `stack_choice` items of the project research `RESEARCH` in `spec/research.spec`, written with `specforge new` and scoped by `--research RESEARCH`: nothing writes a research file by hand. The orchestrator recorded `RESEARCH` before spawning the researchers; the synthesizer sets its summary. `specforge schema --kind stack_choice` lists the fields. A rejected option is a `stack_choice` of tier `alternative` with `--rejected_because`; version compatibility and the sources are `finding` items with `--topic stack`. Fill the quoted values; leave out a flag you have no value for, and repeat the lines per item with the next free id (`specforge query RESEARCH --depth 1 --format context` lists the items already recorded). A refusal names its code: read it with `specforge explain <code>`.

```bash
NAME="{the technology or library}"
VERSION="{its version, e.g. 5.4}"
PURPOSE="{what it is for, and why it is the standard choice}"
TIER="{core, supporting or alternative}"
specforge new stack_choice SC1 "$NAME" --in spec/research.spec --research RESEARCH --name "$NAME" --version "$VERSION" --purpose "$PURPOSE" --tier "$TIER" || exit 1
```

**What goes where:**

- Core technologies (tier `core`): include specific version numbers; explain why this is the standard choice, not just
  what it does; focus on technologies that affect architecture decisions.
- Supporting libraries (tier `supporting`): the libraries commonly needed for this domain; note when each is needed (not
  all projects need all libraries).
- Alternatives (tier `alternative`): don't just dismiss them; explain when an alternative makes sense, which helps the
  user make an informed decision if they disagree.
- What not to use (`rejected_because`): actively warn against outdated or problematic choices; explain the specific
  problem, not just "it's old", and name the recommended alternative.
- Version compatibility (a `finding`, topic `stack`): known compatibility issues, critical for avoiding debugging time
  later.
