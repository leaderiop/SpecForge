# Position — Aslak Hellesøy (Cucumber & Gherkin creator; testing/BDD cluster)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. **Split the language from the runner, then hold the boundary.** Gherkin won as a parse-once interchange; Cucumber's multi-language bindings were forced by existing user codebases. SpecForge plugins have no pre-existing codebase — they are standalone logic against a host contract. That contract, not the runtime, is C7-03's defect: stringly JSON over `call_export`, no IDL. Fix the protocol; don't multiply runtimes behind it.
2. **The workload barely needs a language.** ~97% of guest payload is generated manifest, ~3% logic (formal's 4 passes, ~478 lines of graph analysis). Most rules are declarative — `extensions/software/src/describe_validation_rules.json` carries `wasm_function: null`. TS/Python ecosystems buy nothing for pure-function passes over a graph snapshot.
3. **Determinism and containment are my home turf.** Gherkin's parsers are conformance-tested against shared JSON fixtures — that's R-6. Wasm gives memory isolation plus capability imports by construction (R-2); mlua sandboxing is interpreter convention; Python (ambient pip, system interpreter) fails R-2 and R-3 outright. A scripting tier also re-creates C7-11's three-implementation split instead of converging it.

## Biggest risk in my verdict
Authoring ergonomics: Rust + `wasm32-unknown-unknown` may deter the third parties the registry bets on, and wasm's audit debts (C7-02, C7-04, C7-08, C7-10) mean "keep" implies real fixing work.

## What would change my mind
Collectors (`collect__*`) needing rich parsing of test formats like JUnit or Cucumber JSON, plus watch-loop latency pooled/AOT fixes cannot reach: then one capability-scoped scripting tier behind the same manifest protocol.
