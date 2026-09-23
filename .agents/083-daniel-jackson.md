# 083 — Daniel Jackson

**Cluster:** C10 — Formal methods (@specforge/formal + analyze passes)
**Roster role:** Alloy; 'Software Abstractions'; 'impossible by construction'
**SpecForge anchors:** specforge-validator design philosophy, `specforge analyze` exploration surface, spec/ self-spec corpus

## Why this engineer
Jackson's Alloy demonstrates SpecForge's core bet: small declarative models checked automatically surface design errors that code review never finds. His "analyze a tiny model, get whole error classes" methodology is the philosophy the validator suite embodies, and his "impossible by construction" program — prefer designs that eliminate error classes over designs that detect them — is the ambition behind checks like E042 (composition cycles rejected at compile time, not at runtime). Abstract Explorer, his tool for browsing abstract descriptions of designs, prefigures an `analyze` mode that visualizes and probes the compiled graph rather than only gating it.

## References for SpecForge
**Key works**
- **Software Abstractions: Logic, Language, and Analysis** — MIT Press, 2006; revised ed. 2012. Alloy's relational logic, `check`/`assert` culture, and small-scope hypothesis — the argument for checking 219 self-spec .spec files cheaply.
- **The Essence of Software: Why Concepts Matter for Great Design** — Princeton University Press, 2022. Concepts-over-features analysis; vocabulary for SpecForge's zero-domain-knowledge core vs. extension-provided concepts.
- [Alloy](https://alloytools.org) — canonical site with the free online edition of the book.
- [AlloyTools/org (Alloy 6)](https://github.com/AlloyTools/org) — GitHub. Temporal Alloy (trace logic) — what property kinds beyond W063's static checks could express.
- **Impossible by construction** — essay, 2025. Design-eliminates-error-classes thesis; cite directly in @specforge/formal docs.
- **Abstract Explorer** — tool, 2025. Interactive exploration of abstract models; UX template for graph inspection commands.

## Study first
1. Software Abstractions ch. 1–2 — modeling-as-analysis mindset
2. Small-scope hypothesis — why petgraph reachability checks catch real bugs
3. Essence of Software — concept boundaries for extension vocabularies
