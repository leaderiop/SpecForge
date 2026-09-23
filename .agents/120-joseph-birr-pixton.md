# 120 — Joseph Birr-Pixton

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** rustls lead maintainer; pure-Rust TLS
**SpecForge anchors:** crates/specforge-registry/src/client/http_client.rs (reqwest `rustls-tls` feature — every HTTPS registry call terminates in rustls)

## Why this engineer
SpecForge's registry client enables reqwest's rustls-tls feature, so the TLS posture of the whole registry surface is Birr-Pixton's rustls: no unsafe by default, secure defaults with no configuration, and pluggable CryptoProviders (his rustls-aws-lc-rs work). His decade of maintainer records — including the 2024 memorysafety.org benchmarks showing rustls beating OpenSSL on throughput, handshakes/sec, and memory — is the evidence base for trusting a pure-Rust stack in SpecForge's distribution path.

## References for SpecForge
**Key works**
- [rustls/rustls](https://github.com/rustls/rustls) — GitHub, 2016. The TLS library under SpecForge's registry client; provider and verification docs live here.
- [Securing the Web: Rustls on track to outperform OpenSSL](https://www.memorysafety.org/blog/rustls-performance/) — memorysafety.org, 2024. Birr-Pixton's benchmarks: data throughput, handshakes per second, memory usage vs OpenSSL.
- A decade of Rustls — maintainer retrospective, covered by LWN, 2026. Ten years of TLS-in-Rust maintenance lessons.
- rustls-aws-lc-rs — crates.io. His CryptoProvider wiring; relevant if SpecForge ever pins a crypto backend.

## Study first
1. CryptoProvider selection and what reqwest inherits when rustls-tls is enabled
2. Certificate verification defaults and how rustls errors surface through reqwest
3. Why no-unsafe + no-configuration defaults matter for a supply-chain-facing client
