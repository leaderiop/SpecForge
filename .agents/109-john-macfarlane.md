# 109 — John MacFarlane

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** CommonMark lead; pandoc — markup semantics and boundaries between prose and structure
**SpecForge anchors:** prose-vs-DSL boundary decisions — quoted prose fields (`problem`, `solution`, `definition`, `context`) inside typed entities in `spec/*.spec`; markdown/mermaid/dot renderers in `crates/specforge-emitter/src/model/{markdown,mermaid,dot,dbml}.rs`; `docs/` generated docs

## Why this engineer
MacFarlane spent a career demarcating what markup should carry: CommonMark exists because Markdown's ambiguity broke tooling, and djot/Beyond Markdown because CommonMark's pragmatics still leak into structure. SpecForge faces the identical boundary daily — which content is typed graph data (entities, fields, edges) and which is human prose quoted inside nodes — and MacFarlane's criterion (syntax must have one parse; semantics must not depend on rendering) is the test for that line. Pandoc is also the gold standard for the emitter: lossless, deterministic, round-trippable conversion between a canonical model and markdown, mermaid, dot and DBML outputs.

## References for SpecForge
**Key works**
- [CommonMark Spec](https://spec.commonmark.org/) — commonmark.org, 2014-, versioned. The demonstration that a markup dialect can be made fully unambiguous — the bar for SpecForge's own grammar and for quoted-prose conventions.
- [pandoc](https://github.com/jgm/pandoc) — jgm/pandoc, 2006-. The reference architecture for AST-first document conversion: parse to one internal AST, emit many formats — mirrored by emitter model renderers.
- [Beyond Markdown](https://johnmacfarlane.net/beyond-markdown.html) — essay, 2018 (originally on talk.commonmark.org). Why complexity accumulates when syntax is frozen by legacy — directly relevant to evolving the `.spec` DSL without breaking 219 self-hosted files.
- [djot](https://djot.net/) — djot.net, 2022-. MacFarlane's redesigned light markup with clean block/inline separation — a study in DSL revision done right.

## Study first
1. Beyond Markdown — the cost of ambiguous syntax; audit `.spec` prose fields for parse ambiguity
2. Pandoc's single-AST-to-many-writers pattern vs `crates/specforge-emitter/src/model/`
3. CommonMark's spec-plus-reference-implementation discipline — how grammar.js changes should be governed
