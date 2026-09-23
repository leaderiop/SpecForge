# 044 — James Munns

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** postcard format author; embedded Rust & resource-frugal serialization
**SpecForge anchors:** postcard 1.1.3 in the dependency path via extism-convert (`crates/specforge-extism`, extism 1.21; wasm payloads via `include_bytes!` builtins); `schema/specforge-binary-report.schema.json` (external schema for a compact binary report)

## Why this engineer
Munns designed postcard — serde-compatible, no_std, varint-encoded, deliberately schemaless on the wire — and it is the format extism-convert uses to move typed values across the Wasm host/guest boundary. SpecForge depends on exactly that path to pass payloads to Wasm extension validators, and its binary report format makes the same trade Munns made: compactness and cheap decode over self-description, with the schema kept external (`specforge-binary-report.schema.json` declares the contract; the bytes stay small). His writing on the v1.0 run-up and the "self-describing postcard" question is the primary source on when schemas belong in-band vs. out-of-band.

## References for SpecForge
**Key works**
- [jamesmunns/postcard](https://github.com/jamesmunns/postcard) — GitHub, 2019–present. The format itself: varint encoding, serde derive compatibility, `no_std` first.
- [postcard on docs.rs](https://docs.rs/postcard) — Rust documentation. Wire-format guarantees and flavor (COBS) framing as used on the extism path.
- "The run-up to v1.0 for Postcard" — jamesmunns.com blog, May 2022. Design-history and stability reasoning for a wire format that cannot churn.
- "Use Cases for a 'self describing postcard'" — postcard issue #92, 2023. The in-band vs. out-of-band schema debate, decided in SpecForge's favor by keeping schemas external.
- [OneVariable](https://onevariable.com) — Munns' consultancy; embedded Rust training and consulting material.

## Study first
1. Postcard's varint wire format — what makes wasm-boundary payloads and binary reports small
2. Flavor/framing design (COBS) — delimiting streams without escaping overhead
3. Stability discipline: why a format author refuses self-description by default
