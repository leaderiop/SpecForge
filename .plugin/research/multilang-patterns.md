# Multi-Language Plugin Ecosystems — How "One Protocol, N Languages" Actually Works

Research for the SpecForge plugin-runtime decision. Case studies: Kubernetes
admission webhooks, Envoy proxy-wasm (+ ext_proc callouts), GraphQL federation
subgraphs. Question: how do the most-deployed multi-language extension systems
handle API consistency, docs, testing, and security — and what does MULTI cost?

## 0. TL;DR

All three systems implement "any language" the same way: **one language-neutral
contract enforced by the host, never an in-process embedded interpreter.** Two
shapes exist:

1. **Wire-protocol MULTI** (Kubernetes webhooks, Envoy ext_proc, GraphQL
   subgraphs): plugin = out-of-process service; contract = versioned schema
   (HTTP+JSON AdmissionReview, gRPC protobuf, SDL+`_service`). Consistency is
   cheap because there are no SDKs to maintain — semantics (timeouts, failure
   policy, ordering, audit) live entirely host-side.
2. **Artifact-contract MULTI** (Envoy proxy-wasm): many language SDKs compile
   to one artifact type targeting one versioned ABI. Consistency is expensive:
   SDK×host-version matrices, SDK forks, a stalled ABI, and per-SDK docs — this
   is the real cost ledger of MULTI.

**No studied system multiplexes embedded interpreters in-process** ("wasm + a
scripting tier + ..."). That shape has no successful exemplar at scale; where
Envoy/Google needed richer plugins they added a *second wire protocol*
(ext_proc), not a second embedded runtime.

## 1. The three contracts, side by side

| | Kubernetes admission webhooks | Envoy proxy-wasm / ext_proc | GraphQL federation subgraphs |
|---|---|---|---|
| Contract | `AdmissionReview` JSON over HTTPS, version-negotiated (`admissionReviewVersions`), schema published as OpenAPI | proxy-wasm ABI v0.2.1 (`proxy_abi_version_*` exports, host imports); ext_proc = bidirectional gRPC protobuf stream | GraphQL SDL + federation subgraph spec (`_service`, `@key`, …); gateway composes via `_service` SDL |
| "Any language" mechanism | None needed: HTTP server in anything; official OpenAPI-generated clients for JS, Java, Python, C#, C, Haskell (+ community Go/Ruby/Perl) | 4 SDKs: AssemblyScript (solo-io), C++, Go (upstream Go 1.24 WASI *or* legacy tetratelabs TinyGo), Rust | ~30 server libraries across 14 ecosystems tracked in Apollo's compatibility matrix |
| Where semantics live | Entirely in kube-apiserver: timeouts (1–30s, default 10), `failurePolicy`, reinvocation, idempotence, audit annotations, rejection metrics | Split: host owns eventing/buffers/queues; each SDK re-implements context semantics on top of raw ABI | Split: gateway owns composition/query planning; each library implements `_service` + directive semantics |
| Host-side safety net | `failurePolicy: Fail` (default) / `Ignore`; request filtering (rules/selectors/matchConditions ≤64 CEL); audit + Prometheus metrics | Wasm memory isolation + capability imports (plugins can't reach fs/net uninvited); callouts fail toward `failure_mode_allowed` stats | None — resolvers are trusted in-process code inside each subgraph service |

Sources: [K8s dynamic admission control](https://kubernetes.io/docs/reference/access-authn-authz/extensible-admission-controllers/);
[proxy-wasm/spec](https://github.com/proxy-wasm/spec);
[proxy-wasm Go SDK](https://github.com/proxy-wasm/proxy-wasm-go-sdk);
[Envoy ext_proc](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/ext_proc_filter);
[Apollo compatible subgraphs](https://www.apollographql.com/docs/graphos/schema-design/federated-schemas/reference/compatible-subgraphs).

## 2. Case study: Kubernetes admission webhooks

The purest wire-protocol MULTI. A webhook is an HTTPS endpoint answering
`AdmissionReview` in "the same version it received" (`response.apiVersion`
echo). The protocol is deliberately thin — and everything hard lives in the
config object the *host* interprets:

- **Version negotiation**: `admissionReviewVersions: ["v1"]`; webhooks must
  declare `sideEffects: None | NoneOnDryRun` so the apiserver can safely
  dry-run. `matchPolicy: Equivalent` makes the server convert between API
  versions before calling the webhook — the webhook sees one canonical shape
  regardless of what the client sent.
- **Consistency enforcement is host-side**: the apiserver serializes timeouts,
  applies `failurePolicy` on network/timeout/malformed-response errors,
  distinguishes "webhook said no" (always denies, regardless of policy) from
  "webhook unreachable", reinvokes mutating webhooks (`IfNeeded`), writes
  per-invocation audit annotations (`mutation.webhook.admission.k8s.io/*`,
  `patch.webhook.admission.k8s.io/*`), and exposes
  `apiserver_admission_webhook_rejection_count` with `error_type` labels.
  A plugin author cannot get these wrong — they don't implement them.
- **Docs**: one protocol page + generated API reference. Because there is no
  SDK surface, there is nothing to fragment. The multi-language story is
  OpenAPI codegen: the [kubernetes-client org](https://github.com/kubernetes-client)
  hosts 13 repos of generated/managed clients (JavaScript, Java, Python, C#, C,
  Haskell, + shared `gen` pipeline).
- **Testing**: Kubernetes e2e-tests its own webhook implementation (agnhost
  test image); the ecosystem standard is `envtest`/kind — always against a real
  apiserver. The tests that matter are *integration* tests (ordering, failure
  policy, dry-run), which are language-independent.
- **Security**: TLS is mandatory (scheme must be `https`, `caBundle` PEM
  required for service refs, cert must match `<svc>.<ns>.svc`). API-server
  authentication of itself to webhooks existed only as optional kubeConfig
  creds; scoped short-lived TokenRequest-bound webhook tokens arrived as
  **alpha in v1.37** — i.e., even Kubernetes took ~a decade to ship proper
  webhook authn. Blast radius is managed by config, not by code: scope rules,
  `namespaceSelector`/`objectSelector`, up to 64 CEL `matchConditions`
  (including a CEL `authorizer` for break-glass).

### What MULTI costs here — and what it doesn't

Notably, MULTI costs Kubernetes almost nothing in *SDK* terms — there are no
SDKs. The costs moved into **operations and blast radius**, so large that
Kubernetes ships a dedicated
[good-practices page](https://kubernetes.io/docs/concepts/cluster-administration/admission-webhooks-good-practices/):

- Mutating webhooks run **serially** and can be reinvoked; each adds latency to
  every matching write ("typically in milliseconds" is the *recommendation*,
  not a guarantee). Validating webhooks run in parallel — an ordering lesson
  paid for in the API contract itself.
- A user-space plugin becomes **part of the control plane's dependency graph**:
  the docs warn a webhook matching `subjectaccessreviews` "can block the
  authorization checks that the cluster itself depends on"; as of v1.37 those
  virtual resources are excluded from webhooks by default (deprecated,
  gate-locked soon). Also: mutating node leases "might result in failed node
  upgrades"; mutating webhooks fire *during incident response* (NotReady pods
  during zonal outages) and can stall failover.
- The stated strategic direction: prefer **CEL-based in-process
  ValidatingAdmissionPolicy/MutatingAdmissionPolicy** over webhooks "when
  possible" — i.e., even the inventor of the universal webhook is pulling
  simple logic back in-process for latency and reliability.

## 3. Case study: Envoy proxy-wasm (+ ext_proc)

The artifact-contract MULTI — the closest structural cousin to SpecForge's
current architecture, and the cautionary tale.

- **The spec is real but thin**: [proxy-wasm/spec](https://github.com/proxy-wasm/spec)
  defines ABI versioning (`proxy_abi_version_<major>_<minor>_<patch>` export;
  hosts "should be able to support multiple versions"), snake_case naming,
  explicit host↔guest memory ownership (guest allocates, host copies, SDK must
  free). The "latest and widely implemented" version is **v0.2.1**, with a
  `vNEXT` directory that has not landed widely — ABI evolution effectively
  stalled while hosts evolved.
- **The real contract is the host imports, and they drift.** The new upstream
  Go SDK requires "Envoy >= 1.33.0 — this SDK leverages additional host imports
  added to the proxy-wasm-cpp-host in PR#427". A plugin compiled with a newer
  SDK simply does not run on an older host, and nothing in the ABI *spec*
  version number tells you so. This is C7-03 (no IDL, stringly-typed calls)
  made manifest at ecosystem scale.
- **SDKs fork under version pressure.** The Go ecosystem split: upstream
  `proxy-wasm/proxy-wasm-go-sdk` (Go 1.24 WASI reactors, "effectively a new SDK
  targeting a completely different toolchain") vs legacy
  `tetratelabs/proxy-wasm-go-sdk` (TinyGo). Two incompatible Go SDKs for the
  same ABI; plugin ecosystems fragment.
- **Testing = permanent version matrices.** The Go SDK CI "run[s] end-to-end
  tests with multiple versions of Envoy and Envoy-based istio/proxy". Every
  SDK × every host × every host *version* — the matrix grows multiplicatively
  and is funded by different parties (Google GCP Service Extensions uses the
  new Go SDK; Solo.io maintains the AssemblyScript SDK; Tetrate maintained the
  old Go SDK). There is **no shared cross-SDK conformance suite** comparable to
  Apollo's — each SDK rolls its own e2e.
- **Docs fragment per SDK**, inevitably: the spec repo is a README + ABI
  listings; real documentation lives in four separate SDK repos with uneven
  depth (the C++ SDK is reference-grade because Envoy's own plugins use it;
  others are thinner).
- **Envoy itself flags instability**: the official
  [wasm filter page](https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/wasm_filter)
  says "The Wasm filter is experimental… the configuration structures are
  likely to change," and it is unsupported on Windows.
- **Security**: wasm gives the strongest in-process sandbox of anything studied
  here — memory isolation + capability-imports, which is exactly R-2's model.
  (Sandbox-strength caveats and CVE history are dimension D05 /
  research-sandbox-cves's lane; not duplicated here.)

### Envoy's own answer to wasm's limits: a second *protocol*, not a second runtime

When Google productized Envoy extensions (Cloud Service Extensions), they drew
the line explicitly
([overview](https://docs.cloud.google.com/service-extensions/docs/overview)):

- **Plugins (wasm/proxy-wasm)**: "restricted capability and strict runtime
  requirements… run close to the data plane, and latency optimization is
  managed." Use for header tweaks, redirects, short inline decisions.
- **Callouts (ext_proc gRPC)**: "no runtime restrictions and can reuse existing
  software… Use when the amount of compute or storage is arbitrary, [when you]
  want to maintain state, [or] use external services." Any language, any
  dependency — at the price of a network hop, session affinity, per-message
  timeouts, and `failure_mode` statistics.

Envoy's ext_proc filter documents the ops price of the wire shape: stream
counters, message timeouts, rejected-mutation counters, filter-state access-log
fields for per-phase gRPC latency. **The lesson: within one product, "more
languages and richer logic" was achieved by adding a wire-protocol tier, not by
embedding another interpreter.**

## 4. Case study: GraphQL federation subgraphs

The broadest MULTI (≈30 implementations, 14 language ecosystems) — and the one
that most honestly documents MULTI's drift.

- **Contract**: SDL + the federation subgraph spec. A subgraph is just an HTTP
  GraphQL endpoint that answers `_service { sdl }` and understands `@key`
  entity resolution. Apollo maintains a
  [compatibility matrix](https://www.apollographql.com/docs/graphos/schema-design/federated-schemas/reference/compatible-subgraphs)
  where per-library support is *explicitly non-uniform*: Dgraph and Neo4j fail
  the critical `_service` field outright (❌); `@requires`/`@provides`/federated
  tracing vary from full (gqlgen, HotChocolate, DGS) to missing (Ballerina,
  Ariadne 🔲); several rows are stale (Ballerina module last released 2021;
  Graphene 2024-11; express-graphql archived at 2020). This is what
  "consistent API across languages" looks like when the contract is a spec
  with volunteer implementers: **a published grid of holes.**
- **API consistency mechanism**: composition-time validation in the gateway
  (graph composition rejects incompatible subgraphs) + the schema registry.
  The contract is enforced *late* (at composition/deploy), not at SDK build —
  which is exactly why drift accumulates silently until composition fails.
- **Docs**: one spec + N library docs, with the matrix serving as the
  de-facto "which docs can you trust" index.
- **Testing — the institutionalized answer**: Apollo funds
  [apollo-federation-subgraph-compatibility](https://github.com/apollographql/apollo-federation-subgraph-compatibility),
  a monorepo with a shared conformance test suite ("expected schema and data
  sets… executed tests"), ~40 per-implementation GitHub workflow files
  (test-subgraph-gqlgen.yaml, test-subgraph-strawberry-graphql.yaml, …),
  packaged as an NPX script + reusable GitHub Action, with example
  implementations per library in-tree. This is the cost of MULTI made
  concrete: **one spec, one vendor, ~40 maintained CI pipelines**, forever.
- **Security**: none of it is uniform. Resolvers are trusted in-process code in
  each subgraph service; authz is per-library (middleware, directives,
  custom logic) — the federation conformance suite tests *federation surface
  only*, not authz, rate limiting, or query-cost control. MULTI here means the
  security story is reimplemented N times with N quality levels; safety comes
  from deployment topology (each subgraph is a separate trust domain owned by a
  team), not from any runtime guarantee.

## 5. Cross-cutting: what makes MULTI hold together

| Concern | Wire-protocol MULTI (K8s, ext_proc, subgraphs) | Artifact-contract MULTI (proxy-wasm) |
|---|---|---|
| API consistency | Schema/IDL published + host enforces semantics; version negotiation in the protocol | ABI spec + host imports; *effective* contract = SDK version ↔ host version coupling (Envoy ≥1.33 for new Go SDK) |
| Docs | One protocol doc; generated clients from OpenAPI | Fragmented per SDK; spec repo thin; fork splits docs too |
| Testing | Real-host integration tests; language-independent | Per-SDK e2e matrices across host versions; no shared conformance suite (vs Apollo's funded one) |
| Security | Transport security + host-side blast-radius knobs (failurePolicy, selectors, audit) | Uniform memory isolation via wasm sandbox — the one place artifact-MULTI *beats* wire-MULTI |
| Failure surface | Plugin outage = request-path outage; mitigated by policy config | Plugin/host version skew = "works on my Envoy" hell; mitigated by version matrices |

Three recurring invariants across all three systems:

1. **The host owns semantics; plugins own policy.** Timeouts, ordering,
   failure policy, audit, composition — never re-implemented per language. The
   moment semantics leak into SDKs (proxy-wasm context objects), consistency
   starts decaying.
2. **Conformance is a product you fund, not a property you get.** Apollo
   maintains ~40 CI pipelines; Kubernetes generates its clients from one
   OpenAPI source; proxy-wasm has neither and shows both failure modes
   (forked Go SDKs, stalled ABI).
3. **Language breadth is bought at a trust boundary.** Every system puts
   "any language" either across a network/process boundary (webhooks, gRPC
   callouts, subgraph services) or behind a memory-isolating artifact
   (wasm). None embeds multiple interpreters in-process; the GraphQL case is
   not a counterexample — its "plugins" are whole services, already outside
   the core.

## 6. The costs of MULTI (evidence-backed catalog)

1. **Matrix growth**: SDKs × host versions (proxy-wasm e2e across "multiple
   versions of Envoy and istio/proxy"); implementations × spec features
   (Apollo's 🟢/🔲/❌ grid); K8s avoids this only by having no SDKs at all.
2. **Fork risk under toolchain pressure**: tetratelabs (TinyGo) vs upstream
   (Go 1.24 WASI) Go SDKs — one ABI, two incompatible SDKs, migrated user
   base. [proxy-wasm-go-sdk README](https://github.com/proxy-wasm/proxy-wasm-go-sdk).
3. **Stalled core evolution**: proxy-wasm ABI stuck at v0.2.1 with `vNEXT`
   unlanded, while hosts added imports out-of-band (proxy-wasm-cpp-host
   PR#427). Contracts without funded stewardship ossify or fork.
4. **Heterogeneous guarantees**: Apollo's matrix shows critical features (❌
   `_service`) and whole ecosystems going stale (Ballerina 2021, Graphene
   2024). The weakest implementation becomes the ecosystem's floor.
5. **Ops blast radius replaces code risk**: Kubernetes' webhook failure modes
   (serial latency, control-plane lockout via virtual resources, lease
   mutation breaking upgrades, webhook timeouts during zonal failover) required
   a permanent good-practices doc plus v1.37 protocol changes to contain.
6. **Docs fragmentation** in proportion to SDK count — unavoidable once each
   language re-expresses host semantics idiomatically.
7. **Second-tier escape**: when logic outgrows the inline sandbox, vendors add
   a wire protocol (ext_proc) — i.e., MULTI done right eventually re-imports
   the out-of-process shape anyway, so starting with two runtimes buys you
   both cost centers at once.

## 7. When MULTI works vs fails

**Works** when:

- The plugin is a **deployment unit behind a stable, versioned wire contract**
  (webhook, gRPC callout, subgraph): languages are interchangeable because the
  contract, not the runtime, is the product. (K8s webhooks at near-zero
  language cost; Google's callouts for arbitrary compute/state.)
- All languages compile to **one artifact type** targeting **one funded,
  versioned ABI** (proxy-wasm's four SDKs — the model itself works; the
  burden shows up in matrices, not correctness).
- The host unilaterally owns: timeouts, failure policy, ordering, audit,
  composition — so a plugin cannot be "wrong" about semantics, only about
  policy.
- Conformance testing is funded as a permanent product (Apollo's ~40-pipeline
  monorepo), or avoided entirely by codegen-from-one-schema (K8s).

**Fails** when:

- Plugins need **fine-grained host context** (buffers, shared queues, foreign
  calls): each SDK re-implements those semantics and drifts; the ABI version
  number stops predicting compatibility (Envoy ≥1.33 host-import coupling).
- The contract has **no IDL/versioning discipline**: proxy-wasm's out-of-band
  host imports are the exact failure SpecForge's audit filed as C7-03
  (stringly-typed `call_export`).
- **Security must be uniform across languages**: only the wasm tier gives
  uniform memory isolation; wire-tier gives transport+policy knobs; GraphQL's
  trusted-resolver tier gives *neither* and works only because subgraphs are
  team-owned services. A "MULTI" that mixes sandbox tiers inherits the weakest.
- Conformance is volunteer-run: holes (❌/🔲), stale rows, forks.
- And structurally: **"MULTI = several embedded runtimes in one process" has
  no successful exemplar among the three most-deployed extension systems on
  earth.** Every time scale demanded it, the answer was one runtime plus one
  wire protocol.

## 8. Mapping to SpecForge (evidence → decision inputs)

- **KEEP_WASM already *is* the multi-language story**: like proxy-wasm — any
  language targeting `wasm32-unknown-unknown`, one ABI, one artifact type,
  uniform sandbox (R-2), one host (R-3), hot-reloadable blobs (R-5). The label
  `MULTI` (wasm + scripting tier) corresponds to nothing observed at scale and
  recreates costs 1–7 above (SDK/language matrix, fork risk, heterogeneous
  security floor) inside a single binary — without Apollo-level conformance
  funding or the network boundary that makes wire-MULTI safe.
- **proxy-wasm PR#427 ↔ C7-03**: versioned, generated, *checked-in* contracts
  (IDL) are what kept K8s consistent and are what proxy-wasm lacked; whatever
  runtime wins, SpecForge's stringly `call_export` should become a schema with
  codegen + sync guards (it already has the sync-guard habit: `extension_json_sync`).
- **K8s's lesson for the host API**: push *all* semantics (deadlines — cf. open
  C7-10 `max_execution_ms` unenforced — scoping — cf. C7-09 `query_scope`
  ignored — audit/diagnostics) into the host so plugin SDKs stay thin; a thin
  SDK is the only kind that stays consistent across languages.
- **A future wire tier exists if ever needed**: the ext_proc/webhook shape
  (out-of-process gRPC/HTTP plugin, host-enforced timeout+failure policy) is
  the *proven* way to add "any language, any dependency" later — e.g., for
  collectors scraping external systems — and does not conflict with R-3 (which
  binds the *host*, not plugins).

## Bottom line

Multi-language plugin ecosystems succeed by making the *contract* the product:
either a versioned wire protocol with host-owned semantics (Kubernetes, Envoy
ext_proc, GraphQL subgraphs) or many-language→one-artifact compilation against
a funded ABI (proxy-wasm). Every documented cost of MULTI — version matrices,
SDK forks, conformance drift, docs fragmentation, heterogeneous security —
comes from the artifact-contract shape and is paid down only by permanent
conformance investment; every system that needed richer or more-performant
logic escaped to a wire protocol rather than embedding another interpreter.
SpecForge's `MULTI` (wasm + scripting tier) matches none of the successful
shapes and imports all of their failure modes into one binary.

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** The world's largest multi-language plugin ecosystems
all multiplex at the protocol or single-artifact level with host-owned
semantics — none embeds multiple runtimes; KEEP_WASM already provides the
proven "any language via one contract" shape, so the MULTI option buys the
documented costs without any of the mitigations.
