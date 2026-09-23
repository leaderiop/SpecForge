# 039 — Lee Byron

**Cluster:** C6 — Schemas, validation & serialization (Graph Protocol)
**Roster role:** GraphQL co-creator; schema introspection & "one schema, many consumers"
**SpecForge anchors:** Graph Protocol moat thesis (one schema, many consumers); `crates/specforge-mcp` (~30 tools over the compiled graph); `GraphProtocolSchema` shared by CLI export, LSP, and model renderers

## Why this engineer
Byron co-created GraphQL around one durable bet: publish a single typed schema and let every client — web, mobile, IDE, dashboards — introspect it independently instead of hand-rolling endpoints. That is precisely SpecForge's Graph Protocol thesis: one compiled graph schema serving CLI exports, the LSP, and the MCP tool surface. GraphQL's introspection system (a queryable meta-schema, not a docs page) is the strongest existing precedent for `GraphProtocolSchema` as a first-class artifact, and GraphQL's evolution policy — deprecate fields, never version the schema — is the reference discipline for growing the protocol via Wasm extension vocabulary without breaking consumers.

## References for SpecForge
**Key works**
- [GraphQL Specification](https://spec.graphql.org) — GraphQL Foundation, 2015–present (co-author; Executive Director). Introspection, schema-as-contract, and deprecate-don't-version evolution.
- [graphql/graphql-js](https://github.com/graphql/graphql-js) — GitHub. The JavaScript reference implementation: how a spec + introspection API co-evolve with tooling.
- [facebook/immutable-js](https://github.com/facebook/immutable-js) — GitHub. Byron's other widely adopted abstraction: persistent data structures behind a small API.
- [leebyron.com](https://leebyron.com) — Personal site (Product Engineering Lead at Watershed; Executive Director, GraphQL Foundation) with essays and talks on schema design and adoption.

## Study first
1. GraphQL introspection (`__schema`/`__type`) as a protocol surface, not documentation
2. Client-driven development: how one schema served many independent consumers
3. The evolution policy: `@deprecated` over versioning — keeping `schema_version` semantics honest
