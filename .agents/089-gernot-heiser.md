# 089 — Gernot Heiser

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** seL4 verified kernel; shipping formal verification in production
**SpecForge anchors:** shipping verified components in production, builtin Wasm validators (crates/specforge-extism), incremental verified releases vs. docs/impl drift

## Why this engineer
Heiser led the seL4 project — the first OS kernel with a machine-checked implementation-correctness proof — and then did the harder part: keeping it proven while it shipped in real products, evolving a verification ecosystem (seL4 Foundation) around it. That operational side is SpecForge's actual gap: builtin Wasm validators must stay in lockstep with the protocol (hand-built blobs blocking fresh clones) and docs must not drift from emitted behavior. His lesson is that verification only matters if re-verification is cheap enough to survive change — precisely the specforge-watch/incremental story SpecForge bets on.

## References for SpecForge
**Key works**
- **seL4: Formal Verification of an OS Kernel** — SOSP 2009 (Klein, Elphinstone, Heiser et al.). The milestone proof: functional correctness, safety, and security properties of real kernel code.
- [seL4/seL4](https://github.com/seL4/seL4) — GitHub. The kernel and its proof repository; a working model of proofs maintained across releases.
- **The seL4 Microkernel: An Introduction** — seL4 Foundation, 2020 (free PDF). Accessible overview incl. the proof's scope and what it deliberately does not claim.
- **seL4 in Australia: from research to real-world trustworthy systems** — ACM Queue, 2023. The productionization story: governance, ecosystem, and sustained verification — the closest analogue to shipping checked specs.
- [Trustworthy Systems](https://trustworthy.systems) — the group's site; timelines and publications.

## Study first
1. SOSP 2009 paper — scope/limits of a shipped proof
2. seL4 release process — how proofs track code changes (model for revalidation on graph edits)
3. ACM Queue 2023 — organizational economics of keeping verification alive
