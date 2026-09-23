# 066 — Justin Spahr-Summers

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** Anthropic engineer who co-built MCP (now independent); API-design pedigree (ReactiveCocoa, Carthage)
**SpecForge anchors:** crates/specforge-mcp protocol design (subscriptions.rs, notifications.rs, protocol/error_codes.rs)

## Why this engineer
Spahr-Summers co-designed MCP at Anthropic alongside Parra — reporting credits the two of them with conceiving and building it — after a decade of respected API work on ReactiveCocoa, Mantle, and Carthage. His taste for small orthogonal primitives, explicit state, and honest error surfaces is the design check for specforge-mcp's router, error codes, and subscription events. His GitHub bio now lists independent game development after Anthropic, so cite him as MCP's co-author, not its current owner.

## References for SpecForge
**Key works**
- [Model Context Protocol specification](https://github.com/modelcontextprotocol/modelcontextprotocol) — modelcontextprotocol org, 2024. Co-authored by Spahr-Summers; the contract specforge-mcp implements.
- [ReactiveCocoa/ReactiveCocoa](https://github.com/ReactiveCocoa/ReactiveCocoa) — GitHub, 2012. Composable streams and explicit signal lifetimes — the lineage behind MCP's notification/subscription design.
- [Carthage/Carthage](https://github.com/Carthage/Carthage) — GitHub, 2014. Opinionated simplicity in a decentralized tool; a model for keeping a surface small (specforge-mcp's ~30 tools under growth pressure).
- MCP Protocol: a new AI dev tools building block — The Pragmatic Engineer, 2025. Documents MCP's two-engineer origin and design constraints.

## Study first
1. Subscription semantics in MCP vs ReactiveCocoa signal design
2. Error-code taxonomies (protocol/error_codes.rs) through an explicit-state lens
3. Carthage's constraints-as-features philosophy for CLI/MCP surface growth
