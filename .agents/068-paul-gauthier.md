# 068 — Paul Gauthier

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** aider creator; token-budgeted repo maps
**SpecForge anchors:** export --format=context/brief token budgets (crates/specforge-emitter compile.rs, mcp resources/context.rs, resources/brief.rs)

## Why this engineer
Gauthier built aider's repo map: tree-sitter symbol extraction, graph ranking over the file-dependency graph, then fitting only the most-referenced definitions into an explicit token budget (`--map-tokens`, 1k default, expanded dynamically when no files are in chat). That is precisely the discipline SpecForge's context/brief exports need — fixed-budget, graph-ranked slices of the spec corpus instead of whole-file dumps — and RES-18's 70-90% reduction claims stand or fall on this kind of budget engineering.

## References for SpecForge
**Key works**
- [Aider-AI/aider](https://github.com/Aider-AI/aider) — GitHub, 2023. Production AI pair programmer whose context pipeline is the closest working analogue to `export --format=context|brief`.
- [Building a better repository map with tree sitter](https://aider.chat/2023/10/22/repomap.html) — aider blog, 2023. The extract → rank → budget-fit algorithm SpecForge's emitters should mirror.
- [Repository map docs](https://aider.chat/docs/repomap.html) — aider docs. Concise spec of budget behavior: what stays in a 1k budget, when it expands.
- Aider LLM code editing leaderboard — aider.chat, 2024. Measuring whether context format actually improves agent edit accuracy — the experiment design RES-18 needs.

## Study first
1. Repo-map ranking: tree-sitter tags → file graph → ranked budget fit
2. Dynamic expansion vs hard caps — map to context.rs/brief.rs sizing rules
3. Leaderboard methodology for A/B-ing context formats on real tasks
