# 030 — Evan Czaplicki

**Cluster:** C4 — Incremental compilation, LSP, diagnostics UX
**Roster role:** Elm creator; compiler-error UX gold standard
**SpecForge anchors:** diagnostic suggestion quality, check output (crates/specforge-emitter/src/diagnostic_fmt.rs, specforge CLI check)

## Why this engineer
Czaplicki made compiler errors a product feature: Elm's messages lead with a plain-language problem statement, point at the exact span, and offer concrete hints — often a drop-in replacement — refined across the "Compiler Errors for Humans" redesign. SpecForge's check output is the primary feedback loop for both humans and the AI agents writing its 219-file `.spec` corpus, so Elm's bar — a diagnostic you can act on without reading the spec manual — is the acceptance criterion for suggestion quality in diagnostic_fmt.rs.

## References for SpecForge
**Key works**
- [Compiler Errors for Humans](https://elm-lang.org/news/compiler-errors-for-humans) — elm-lang.org, 2015. The founding essay on redesigning terminal error UX around reader experience; SpecForge's check output should be measured against its examples.
- [The Perfect Bug Report](https://elm-lang.org/news/the-perfect-bug-report) — elm-lang.org, 2016. Time-travel debugging framed as report quality — the mindset for what a diagnostic must contain to be actionable.
- Elm: Concurrent FRP for Functional GUIs — Harvard senior thesis, 2012. Origin of the language and its compiler-as-teacher philosophy.
- [elm/compiler](https://github.com/elm/compiler) — GitHub, 2012. Reference implementation where the error-message strings and hint generation live.

## Study first
1. Elm's error format: problem → hint → annotated source region
2. Hints as concrete replacements vs SpecForge's did-you-mean strings
3. Designing check output for non-expert readers (including agents) first
