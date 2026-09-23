# 007 — Graydon Hoare

**Cluster:** C2 — DSL & language design
**Roster role:** Rust creator; systems-language discipline & diagnostics culture
**SpecForge anchors:** Rust workspace edition 2024 (`Cargo.toml`); rustc-style diagnostics goal (`spec/product/journeys.spec`: "errors printed in rustc style with suggestions")

## Why this engineer
Hoare created Rust and its two most transferable legacies: compiler diagnostics that teach while they reject, and an evolution process (editions, RFCs) that lets a language change without breaking working users. SpecForge is a Rust-native workspace (edition 2024) that has promised rustc-style errors with suggestions as a product feature, not an afterthought. His "not rocket science rule" — only merge on green — is also the discipline behind a ~2,900-test suite standing between every commit and the graph.

## References for SpecForge
**Key works**
- [rust-lang/rust](https://github.com/rust-lang/rust) — GitHub, 2010. Canonical implementation: error codes, diagnostic rendering, and edition machinery SpecForge inherits as a Rust 2024 workspace.
- [The Rust Compiler Development Guide: Diagnostics](https://rustc-dev-guide.rust-lang.org/diagnostics.html) — rustc-dev-guide.rust-lang.org. Concrete mechanics of error codes and structured suggestions — the standard the journeys.spec "rustc style" requirement points at.
- **The "Not Rocket Science Rule" and peripheral thoughts** — graydon2.dreamwidth.org, 2010. Only merge if tests pass: CI hygiene that keeps a 2,900-test gate meaningful.
- **Graydon Hoare Remembers the Early Days of Rust** — The New Stack, 2023. First-person account of a personal language project becoming a product — the closest parallel to SpecForge's solo-founder origin.

## Study first
1. rustc-dev-guide diagnostics chapter (codes, suggestions, rendering)
2. Not Rocket Science Rule (2010)
3. Early-days retrospective — how scope discipline survived growth
