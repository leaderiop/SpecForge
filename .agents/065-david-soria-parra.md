# 065 — David Soria Parra

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** MCP co-creator; protocol design for agent-tool boundaries
**SpecForge anchors:** crates/specforge-mcp protocol design (protocol/router.rs, tools/, resources/, prompts/, subscriptions.rs)

## Why this engineer
Parra is a Member of Technical Staff at Anthropic and co-created the Model Context Protocol — the exact standard `crates/specforge-mcp` implements. His design choices (JSON-RPC framing, resources vs tools vs prompts as distinct primitives, subscriptions/notifications, capability negotiation) are the contract SpecForge's ~30 tools, graph resources, and infer prompts must honor; knowing why MCP drew those lines keeps SpecForge from inventing a divergent second vocabulary.

## References for SpecForge
**Key works**
- [Model Context Protocol specification](https://github.com/modelcontextprotocol/modelcontextprotocol) — modelcontextprotocol org, 2024. Normative reference for resources/tools/prompts, subscriptions, and error semantics in specforge-mcp.
- [modelcontextprotocol.io](https://modelcontextprotocol.io) — official docs. Lifecycle and capability negotiation that the server's lifecycle.rs and registry.rs implement.
- The Creators of Model Context Protocol — Latent Space podcast, 2025. Rationale straight from the co-creators on what belongs in the protocol vs the host.
- Anthropic and the Model Context Protocol — Software Engineering Daily, 2025. Interview covering protocol evolution and tool-description token costs.
- MCP co-creator on the next wave of LLM innovation — a16z interview, 2025. Developer-experience priorities that shaped MCP's surfaces.

## Study first
1. MCP spec's resources/tools/prompts split — mirror it in specforge-mcp's modules
2. Subscription/notification semantics vs SpecForge's watch-based graph deltas
3. Tool-description cost discipline (~100-200 tokens each) from RES-24/RES-18
