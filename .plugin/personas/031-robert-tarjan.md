# Position — Robert Tarjan (graph algorithms & complexity)

**Verdict:** KEEP_WASM
**Confidence:** 4

## Arguments
1. Determinism by allowlist. A wasm guest computes deterministically unless the host imports ambient effects; the capability surface is enumerable. Interpreters get safety by deleting stdlib entries (`os.time`, `io`) — a blacklist over an ambient-capability ecosystem (evidence §4). R-6 snapshot testing needs the former, and it composes with the host's deterministic machinery: three-color DFS (`crates/specforge-graph/src/graph.rs:474`), Kahn ordering (`crates/specforge-resolver/src/resolve.rs:405`).
2. The asymptotics live in the protocol, not the runtime. Marshalling the ~1.7k-entity snapshot is O(V+E) per call under every candidate — swapping engines changes constants only. The leverage is fixing C7-03 (real IDL) and C7-09 (`query_scope` shrinks the marshalled subgraph). Both are in-model fixes.
3. Hot reload (R-5) is an amortization problem: C7-08's engine pool is a ledger and C7-02's "AOT" cache a byte-copy, so watch pays compile per edit. Real serialized-AOT caching amortizes instantiation toward O(1) per plugin. The host already loads extensions as a topologically ordered DAG (`crates/specforge-wasm/src/toposort.rs`), keeping incremental invalidation well-defined; the mirrors (C7-11) then converge into one mechanism.

## Biggest risk in my verdict
The payload is ~97% generated manifest, ~3% logic (evidence §2), so authoring friction looks cheap today. If third-party plugins prove logic-heavy, Rust+wasm toolchain friction stalls the registry bet and a scripting tier becomes necessary.

## What would change my mind
Measurements showing serialized AOT cannot amortize watch-loop latency, or AI-agent authors reliably failing the wasm32 toolchain; plugins needing rich in-graph queries would favor a host-query callback tier over marshalling.
