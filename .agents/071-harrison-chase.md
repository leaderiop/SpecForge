# 071 — Harrison Chase

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** LangChain co-founder/CEO; agent graph orchestration
**SpecForge anchors:** agent graph orchestration (crates/specforge-graph petgraph, cycle detection), MCP subscriptions (crates/specforge-mcp subscriptions.rs)

## Why this engineer
Chase's LangGraph made "agents as state graphs" — nodes, edges, checkpoints, interrupts — the mainstream orchestration pattern, structurally echoing SpecForge's validated petgraph with cycle detection. His team's MCP client work (langchain-mcp-adapters) exercises exactly the surfaces specforge-mcp ships: tools, resources, prompts, and change subscriptions. As CEO of the most-deployed agent framework, he is the consumer-side oracle for how orchestrating agents will query and subscribe to a spec graph.

## References for SpecForge
**Key works**
- [langchain-ai/langgraph](https://github.com/langchain-ai/langgraph) — GitHub, 2023. Durable agent state graphs; the orchestration analog of a validated spec graph.
- [langchain-ai/langchain](https://github.com/langchain-ai/langchain) — GitHub, 2022. The most-deployed agent framework; defines what tools-for-agents must feel like in practice.
- [langchain-ai/langchain-mcp-adapters](https://github.com/langchain-ai/langchain-mcp-adapters) — GitHub, 2025. MCP client adapters — a conformance pressure test for specforge-mcp's subscriptions and resources.
- Ambient agents — LangChain blog, 2024. Chase's case for event-triggered agents rather than chat turns — the argument for graph subscriptions.
- Building the orchestration layer for agents — Sequoia interview, 2024. His thesis that orchestration/context, not raw models, is the bottleneck.

## Study first
1. LangGraph checkpointing vs SpecForge's stateless graph + subscriptions
2. Ambient (event-triggered) agents as subscribers of specforge-watch/mcp deltas
3. How LangChain agents budget tool calls against context windows
