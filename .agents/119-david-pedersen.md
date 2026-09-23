# 119 — David Pedersen

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** axum creator; macro-free extractor-based HTTP handler design
**SpecForge anchors:** crates/specforge-registry-server/src/handlers.rs (axum handlers), auth.rs/state.rs (tower-style shared state & middleware)

## Why this engineer
Pedersen designed axum's defining idea — handlers are plain async functions typed by their extractors, layered over tower::Service with zero macros — and specforge-registry-server's handlers.rs is written in exactly that idiom: extractor-ordered parameters, `State<T>` for the server's registries/storage, cross-cutting auth as middleware. His design notes and podcasts are the manual for keeping SpecForge's HTTP surface thin and pushing concerns into layers instead of handler bodies.

## References for SpecForge
**Key works**
- [tokio-rs/axum](https://github.com/tokio-rs/axum) — GitHub, 2021. The framework behind specforge-registry-server; extractor and State documentation is the handler contract.
- [Announcing Axum](https://tokio.rs/blog/2021-07-announcing-axum) — tokio.rs blog, 2021. The creator's rationale: ergonomics without macros, full tower interoperability.
- Axum with David Pedersen — Rustacean Station podcast, 2022. Design, testing, and review process of the framework in the creator's words.

## Study first
1. Extractor order and FromRequestParts vs FromRequest semantics (handlers.rs signatures)
2. State<T> + tower layers pattern: moving auth.rs logic out of handler bodies
3. axum's error model: mapping registry errors to typed HTTP responses
