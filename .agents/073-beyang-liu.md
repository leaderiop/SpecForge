# 073 — Beyang Liu

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** Sourcegraph co-founder (ex-CTO), now Amp co-founder; code context for AI
**SpecForge anchors:** graph as code-context substrate (specforge trace, mcp tools/find_definition.rs, find_references.rs, find_implementation.rs)

## Why this engineer
Liu spent a decade proving that code intelligence — precise definitions, cross-references, structural search — is the substrate AI coding tools need; Sourcegraph's Cody and now Amp are built on it. SpecForge's trace/find_definition/find_references tools extend that thesis from code to specifications: the validated entity graph is the context substrate agents should navigate. He also lived the market lesson RES-19 formalizes — context quality, not model access, is the durable moat — and the Dec 2025 Sourcegraph/Amp split shows the context-platform-vs-agent-product line being drawn in real time.

## References for SpecForge
**Key works**
- Why Sourcegraph and Amp are becoming independent — Sourcegraph blog, 2025. The strategy split between context platforms and agent products; direct input to RES-19 positioning.
- [Amp](https://ampcode.com) — Amp Inc., 2025-2026. His current coding agent; a demanding consumer of exactly the code-context substrate SpecForge mirrors for specs.
- Building Cody, an open source AI coding assistant — talk, 2023. How assembled context (code intelligence + retrieval) feeds an assistant — the stochastic analogue of export/query.
- [sourcegraph/zoekt](https://github.com/sourcegraph/zoekt) — GitHub, 2014. Trigram-indexed code search; the engineering baseline for what "precise context" costs to serve.

## Study first
1. Code-intelligence primitives (defs/refs) vs SpecForge's typed graph edges
2. Context-platform vs agent-product economics (Sourcegraph → Amp split)
3. zoekt's index design as a cost model for serving context at scale
