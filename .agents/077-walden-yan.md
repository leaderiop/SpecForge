# 077 — Walden Yan

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** Cognition co-founder (Devin); 'context engineering' essayist
**SpecForge anchors:** context-first design thesis (README "Agents Are First-Class Consumers", RES-18, vision/north-star.md)

## Why this engineer
Yan's essay "Don't Build Multi-Agents" — subtitled Principles of Context Engineering — argued that context engineering (automatically giving the model the right context) is effectively the #1 job of agent builders, and that architectures fragmenting context across parallel subagents break reliability. SpecForge's entire thesis is a substrate for that job: a compiled, queryable context store agents page from instead of exploring. He is the design conscience for keeping SpecForge's surfaces context-first — full-context responses, single-threaded-friendly workflows, subscriptions instead of duplicated state.

## References for SpecForge
**Key works**
- [Don't Build Multi-Agents](https://cognition.com/blog/dont-build-multi-agents) — Cognition blog, 2025. The context-engineering essay: share full agent traces; actions carry implicit decisions; single-threaded linear agents first.
- Multi-Agents: What's Actually Working — Cognition blog, 2026. His revision of the thesis; read both to see how the principles evolve.
- [Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents) — Anthropic, 2024. The complementary workflows-vs-agents framing Yan builds against.
- Why Cognition does not use multi-agent systems — jxnl.co interview, 2025. Yan on context passing and single-threaded writes in production (Devin).

## Study first
1. Principles 1-2 as response-shape rules for specforge-mcp tools
2. Context compression for long runs vs SpecForge's multi-resolution --depth
3. Where parallelism is safe (query fan-out) vs where it breaks (writes)
