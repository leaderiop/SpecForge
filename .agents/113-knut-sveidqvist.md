# 113 — Knut Sveidqvist

**Cluster:** C13 — Diagrams & rendering
**Roster role:** Mermaid creator; diagrams-as-code in Markdown-native tooling
**SpecForge anchors:** mermaid `erDiagram` renderer in specforge-emitter/src/model (crates/specforge-emitter/src/model/mermaid.rs), outline mermaid renderer (crates/specforge-emitter/src/outline/mermaid.rs)

## Why this engineer
Sveidqvist built the diagram language that decides whether SpecForge's `model --format=mermaid` output actually lands in front of users: Mermaid renders natively in GitHub, GitLab, and most Markdown tooling, so the emitters' `erDiagram` text is the project's widest distribution channel. His design constraints — Markdown-inspired syntax, a renderer registry per diagram type, zero-install text-first workflow — define both the compatibility bar (`render_mermaid` must emit syntax mainstream renderers accept) and the UX target (diagrams that never drift from the docs around them).

## References for SpecForge
**Key works**
- [mermaid-js/mermaid](https://github.com/mermaid-js/mermaid) — GitHub, 2014–. Reference implementation: per-diagram-type renderers and the `erDiagram` grammar `render_mermaid` targets.
- [Mermaid documentation](https://mermaid.js.org) — syntax reference, including entity-relationship attribute and cardinality syntax.
- **The Official Guide to Mermaid.js** (with Ashish Jain) — Packt, 2021. Authoritative walkthrough of diagram types and tooling integration.
- [Mermaid Chart](https://mermaidchart.com) — commercial editor/live-rendering evolution; the feedback-loop UX for text-diagram authoring.

## Study first
1. `erDiagram` attribute/cardinality syntax vs what `render_mermaid` emits
2. Renderer-registry-per-diagram-type architecture
3. Ecosystem adoption path (Markdown platforms, 2022 GitHub integration)
