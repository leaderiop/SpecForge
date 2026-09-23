# 107 — Alberto Brandolini

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Event Storming — collective discovery through domain events
**SpecForge anchors:** `event` entities in `spec/events/*.spec` (channel, payload, consumers) + `produces`/`consumes` edges in the graph; `spec/extensions/*.spec` surface-contributed events; PRD-001 extension handshake as facilitated discovery

## Why this engineer
Brandolini's Event Storming starts from domain events — orange sticky notes naming what happened — then walks backward to commands and forward to read models, surfacing bounded contexts and conflicts in a workshop instead of a document. SpecForge's event graph is the durable artifact of that exercise: events with declared channels and payloads, producers and consumers as graph edges, and cross-extension event contributions (`surface-contributions.spec`) playing the role of storming-session hot spots. His rule that the workshop output must become a living, testable model — not a photo of a wall — is precisely why events live in `.spec` files the validator can check.

## References for SpecForge
**Key works**
- **Introducing EventStorming: An Act of Deliberate Collective Learning** — Leanpub, 2018 (leanpub.com/introducing_eventstorming; book info at eventstorming.com). The method end-to-end: big picture → process level → software design; the workshop choreography behind SpecForge's event-first authoring flow.
- **Introducing Event Storming** — blog post, ziobrando.blogspot.com, 2013. The original short statement: start with events, defer design — useful as a 10-minute team onboarding.
- **EventStorming.com** — official book and community site. Evolving guidance and formats beyond the 2018 print.

## Study first
1. Book part on big-picture storming — map its sticky-note color scheme onto entity kinds
2. How event → command → aggregate chains become event producer/consumer edges here
3. Hot-spot conflicts in storming = validator diagnostics candidates for clashing event payloads
