# 116 — Sean McArthur

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** hyper & reqwest creator; Rust HTTP client infrastructure
**SpecForge anchors:** crates/specforge-registry/src/client/http_client.rs (reqwest blocking Client, JSON + multipart package upload, rustls-tls)

## Why this engineer
Every HTTPS call SpecForge's registry client makes goes through reqwest, McArthur's ergonomic wrapper over his own hyper engine (hyper 1.0, reqwest 0.12). His project's READMEs, issue triage, and changelogs are the ground truth for the behaviors SpecForge inherits: connection pooling, timeout layering, feature-gated TLS backends, and blocking-client semantics (a per-call Tokio runtime — matters for CLI startup cost in specforge-cli).

## References for SpecForge
**Key works**
- [hyperium/hyper](https://github.com/hyperium/hyper) — GitHub, 2014. The fast, correct HTTP/1+2 implementation underneath reqwest; the spec for pooling and connection behavior the registry client leans on.
- [seanmonstar/reqwest](https://github.com/seanmonstar/reqwest) — GitHub, 2016. The exact crate behind specforge-registry's client; canonical documentation of the rustls-tls/blocking/json/multipart feature matrix SpecForge enables.
- Netstack.fm — podcast, 2025. The story of Rust networking with hyper, told by its maintainer: design history and long-horizon maintenance philosophy.
- Hyper with Sean McArthur — Rustacean Station podcast, 2021. Creation story, backpressure and graceful-shutdown decisions.

## Study first
1. reqwest blocking client internals and why each call spins a Tokio runtime
2. hyper's Service model and connection-pooling semantics
3. Feature-gating TLS backends (rustls vs native-tls) and its compile-time trade-offs
