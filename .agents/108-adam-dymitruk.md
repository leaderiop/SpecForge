# 108 — Adam Dymitruk

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Event Modeling — visual blueprint from command to read model
**SpecForge anchors:** journey/event flow specs — `spec/product/journeys.spec` (persona, channel, step flows) feeding `spec/events/*.spec`; `spec/product/personas.spec` + `channels.spec` as swim-lane actors; `docs/spec-writing-flow.md`

## Why this engineer
Dymitruk's Event Modeling fixes Event Storming's output into a precise two-dimensional blueprint: horizontal swim-lanes per actor/system, vertical slices per user journey, and a strict left-to-right grammar of commands → events → read models, each slice carrying its own wireframe and tests. SpecForge's journey entities (persona, channel, ordered steps mapping to features and events) plus its event entities reproduce the same slice anatomy in a compiled graph — journeys become the horizontal axis, event produces/consumes edges the vertical grammar. His insistence that every slice yields acceptance criteria anticipates SpecForge's `verify` blocks attached to behaviors.

## References for SpecForge
**Key works**
- [Event Modeling](https://eventmodeling.org) — eventmodeling.org, 2020. The canonical method definition: actors, commands, events, read models, slices — the layout journeys/events entities encode.
- **Understanding Event Modeling** — guide on eventmodeling.org. Step-by-step teaching sequence; a model for SpecForge authoring documentation.
- **Adaptech Group Event Modeling & Event Sourcing workshops** — led by Dymitruk, 2020s. Practitioner materials showing blueprint-to-implementation traceability, the property `specforge-report.json` traces.

## Study first
1. The blueprint grammar (command/event/read-model ordering) vs SpecForge event edges
2. Slice-level acceptance criteria → `verify` statements on journey-derived behaviors
3. Event Modeling's documentation-of-decisions habit vs ADR entities (106)
