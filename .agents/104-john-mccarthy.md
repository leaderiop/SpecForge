# 104 — John McCarthy

**Cluster:** C12 — Requirements engineering, ubiquitous language & docs-as-code
**Roster role:** Formal business communication language (CBCL); shared formal world descriptions
**SpecForge anchors:** spec graph as shared formal vocabulary — `crates/specforge-graph` (petgraph nodes/edges), `schema/specforge.schema.json` Graph Protocol, `crates/specforge-wasm` handshake/describe protocol

## Why this engineer
McCarthy's Common Business Communication Language (1982) proposed that businesses exchange formally structured, machine-interpretable messages — a shared vocabulary with declared terms, partial interpretation and conventions for omission — decades before JSON schemas and EDI XML. SpecForge realizes the same bet one level down: heterogeneous parties (core compiler, Wasm extensions, CLI/LSP/MCP surfaces, test harness) coordinate only through the formally described Graph Protocol, never through prose contracts. McCarthy's insistence that inter-computer communication needs explicit, decidable conventions — not natural language — is the design principle that keeps the zero-domain core extensible by strangers.

## References for SpecForge
**Key works**
- **The Common Business Communication Language** — Stanford, 1982; reprinted in *Formalizing Common Sense* (Ablex, 1990). [Archive: jmc.stanford.edu] The direct precedent for machine-to-machine formal vocabulary — read for its rules on omission and defaults, mirrored in extension handshakes.
- **Programs with Common Sense** — Symposium on Mechanisation of Thought Processes, HMSO London, 1959. The advice-taker argument: represent the world formally so programs can reason and communicate about it — SpecForge's graph-as-world for tools.
- **Recursive Functions of Symbolic Expressions and Their Computation by Machine, Part I** — CACM 3(4), 1960. Programs-as-data (s-expressions): the aesthetic behind a self-hosted spec corpus (`spec/*.spec`) that the compiler itself consumes.

## Study first
1. CBCL: what conventions let independent parties interoperate without a central authority
2. Handshake/describe protocol (PRD-001) as CBCL's declared-vocabulary step
3. McCarthy's nonmonotonic caveats for CBCL — how missing fields should be interpreted in Graph Protocol
