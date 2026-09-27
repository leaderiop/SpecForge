# R: Plugin API Design — What Makes Plugin APIs Succeed or Fail

**Analyst:** research-plugin-api-design (batch 2). Question: study the API design of successful plugin
systems (VS Code, Figma, Obsidian, Neovim, Kubernetes CRDs), extract the patterns that correlate with
adoption and the anti-patterns that kill ecosystems, and apply them to SpecForge's spec-validation +
analysis plugin use case. Method: primary-source reads of each platform's own docs/policies (web_search
provider was down; every claim below is sourced to a fetched primary document). SpecForge facts are from
`.plugin/evidence.md` (rev 4c9e9f2) and `.plugin/decision-brief.md`. Complements D08 (which covers the
host-contract/IDL lens from inside); this file brings outside ecosystem evidence.

---

## 1. Case studies (what each platform actually does)

### 1.1 VS Code — declarative contributions + ruthless stability discipline

- **Activation is declarative and lazy.** Extensions declare `activationEvents` in `package.json`; the
  host activates them only when the event fires. `onStartupFinished` exists specifically so extensions
  "will not slow down VS Code startup" (vs the `*` event). Since 1.74, contributed commands/views/
  languages no longer even need an explicit activation declaration — the host **infers activation from
  the contribution itself**. ([activation-events](https://code.visualstudio.com/api/references/activation-events))
- **Contributions are static JSON.** Everything the host can show before running code (menus, grammars,
  languages, configuration schemas) lives in the manifest's `contributes` section, not in code.
- **API stability is an explicit covenant.** "We take Extension API compatibility seriously... once we
  introduce an API, we cannot easily change it anymore." Unstable surface ships as **Proposed API**:
  Insiders-only, per-extension gated via `enabledApiProposals`, graduated to stable only after iteration.
  ([proposed API](https://code.visualstudio.com/api/advanced-topics/using-proposed-api))
- **The platform dogfoods its own public API.** "Many core features of VS Code are built as
  [extensions] and use the same Extension API" (Git, Markdown, themes live in `microsoft/vscode/extensions`).
  This is the canonical implementation of "no first-party tier". ([API landing](https://code.visualstudio.com/api))

### 1.2 Figma — sandbox-first with manifest-declared capabilities

- Plugin code runs **on the main thread in a minimal JS sandbox with no browser APIs** — "browser APIs
  like `XMLHttpRequest`, `fetch`, `setTimeout`, and the DOM are not directly available from the sandbox."
  UI/network code goes into an iframe; the two halves talk only by message passing.
  ([how plugins run](https://developers.figma.com/docs/plugins/how-plugins-run/))
- **Capabilities are deny-by-default and accountable.** `networkAccess.allowedDomains` supports
  `["none"]`; a blanket `["*"]` **requires a written `reasoning` string**; `devAllowedDomains` separates
  dev-time from shipped grants. Grants are displayed on the plugin's public Community page.
  Enforcement is at the transport: out-of-manifest requests fail with a CSP error.
  ([manifest](https://developers.figma.com/docs/plugins/manifest/))
- **Breaking changes are manifest-versioned with migration docs** (`documentAccess: "dynamic-page"` is
  required for new plugins; old behavior stays for un-migrated ones). API version is declared per plugin
  (`"api": "1.0.0"`) and never auto-upgraded. Official TypeScript typings
  ([figma/plugin-typings](https://github.com/figma/plugin-typings)) are the authoring surface.

### 1.3 Obsidian — lowest-friction authoring, highest registry counts

- A plugin is a tiny manifest (`id`, `version`, `minAppVersion`, `isDesktopOnly`) + one JS bundle
  ([sample manifest](https://raw.githubusercontent.com/obsidianmd/obsidian-sample-plugin/master/manifest.json)).
  No sandbox: desktop plugins get full Electron/Node (a deliberate trust-for-velocity trade).
- The API is a lifecycle class: `onload`/`onunload`, plus **registration helpers that auto-dispose**
  (`registerDomEvent`, `registerInterval` are removed "when this plugin is disabled") and
  **`checkCallback` commands** that let the host ask "can this run now?" without activating anything.
  ([sample main.ts](https://raw.githubusercontent.com/obsidianmd/obsidian-sample-plugin/master/src/main.ts))
- Distribution is a curated JSON registry in a public repo
  ([obsidian-releases](https://github.com/obsidianmd/obsidian-releases)); inclusion is a human-reviewed
  PR [INFERENCE from repo structure — review docs page is a JS app I could not render]. That review
  burden is the price of the no-sandbox choice.
- **Measured: 8,101 community plugins in the registry file** (fetched 2026-09-27,
  `community-plugins.json | jq length`). For an app released in 2020, this is the strongest recent
  datapoint that plain-JS + lifecycle-class + tiny manifest scales.

### 1.4 Neovim — the authoring-language switch is the adoption lever

- v0.5 (2021) made **Lua first-class**, chosen because it is "tiny – perfect for embedding... fast –
  LuaJIT can be orders of magnitude faster than Vimscript... simple". The release notes state the
  outcome directly: "an **explosion in the number of Lua plugins**... often from contributors who were
  completely new to (neo)vim plugin development and **were averse to learning Vimscript** for that task."
  ([0.5 newsletter](https://neovim.io/news/2021/07/)) Note also: Fennel, Teal, MoonScript all compile to
  Lua — one embedded runtime became a multi-language *target*, not a multi-runtime system.
- **Stability contract:** "Neovim's strict API contract, which mandates that after an API function makes
  it into a stable release, its signature **must not change in any way**." Ergonomics are added as new
  functions (`vim.keymap.set`) rather than edits to stable ones. ([0.7 notes](https://neovim.io/news/2022/04/))
- Host-side perf lesson: `filetype.vim` (hundreds of eager autocommands at startup) → `filetype.lua`
  (single dispatch + table lookup), because startup-time profiling drove a core redesign
  ([0.7 notes](https://neovim.io/news/2022/04/)) — hosts must design eager host work out of the hot path.

### 1.5 Kubernetes CRDs — validation as declared, compiled, budgeted data

- `apiextensions.k8s.io/v1` made a **structural OpenAPI v3 schema mandatory** (optional in the beta API):
  every field typed, validated at admission, unknown fields pruned.
  ([CRD docs](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/))
- **CEL validation rules** (`x-kubernetes-validations`) are the sanctioned escape hatch, and they are
  engineered for determinism: rules are **compiled (and type-checked) at CRD create/update — a failing
  compile rejects the CRD**; they are **object-scoped: "no cross-object or stateful validation rules are
  supported"**; and they run under an explicit **cost budget** — the API server estimates rule cost at
  create time and rejects rules "exceeding budget by more than 100x", with runtime enforcement too.
  (same page, Validation Rules section)
- **Deprecation policy as written law:** API elements can only be removed by version increment; GA APIs
  are never removed within a major version; nothing may be deprecated "in favor of a less stable API
  version"; deprecated endpoints emit RFC 7234 `Warning` headers and audit annotations.
  ([deprecation policy](https://kubernetes.io/docs/reference/deprecation-policy/))

### 1.6 Two cautionary counter-cases

- **Firefox 57 / WebExtensions:** a *justified* platform migration executed as a hard deadline decoupled
  from replacement-API completeness: "The Firefox 57 deadline is not dependent on how many add-ons will
  be ported," with only "close to 2,000 WebExtensions listed on AMO" at announcement time (Feb 2017) and
  Mozilla's own admission: "We know it will be a painful transition and we will lose valuable members of
  the community." Documented fallout includes veteran authors quitting after wasted rewrite cycles.
  ([compatibility milestones](https://blog.mozilla.org/addons/2017/02/16/the-road-to-firefox-57-compatibility-milestones/))
- **Chrome Manifest V3:** years-long forced migration (background pages → service workers, blocking
  `webRequest` → declarative `declarativeNetRequest`, remote code banned so "an extension can only execute
  JavaScript that is included within its package and subject to review"). Defensible security goals, but
  the churn damaged developer trust; a 2025 survey literature now exists specifically on developer
  insight into MV3 ([arXiv:2507.13926](https://arxiv.org/abs/2507.13926)).
  ([MV3 overview](https://developer.chrome.com/docs/extensions/develop/migrate/what-is-mv3))

### 1.7 Closest workload analog — Shopify Functions

Shopify (the largest commercial plugin ecosystem doing *validation-shaped* work: discounts, checkout
validation, delivery constraints) runs third-party functions as **WASM modules**: declarative GraphQL
**input queries** project exactly the data needed, the module returns a JSON operations document, hosts
never call plugins directly ("Shopify invokes them as-needed"), and Shopify "strongly recommends Rust as
the most performant language choice to avoid your function failing with large carts."
([About Shopify Functions](https://shopify.dev/docs/apps/build/functions)) I.e., for a spec-validation +
compiler-pass workload shape, a major platform independently converged on SpecForge's current model.

---

## 2. Patterns that correlate with adoption

| # | Pattern | Evidence |
| --- | --- | --- |
| P1 | **Declarative manifest is the contract.** Static, inspectable metadata (contributions, capabilities, schemas) separable from code, so the host can serve/route/validate without executing untrusted code. | VS Code `contributes`+`activationEvents`; Figma manifest; Obsidian manifest; K8s structural schema; Shopify input queries |
| P2 | **Low authoring friction beats API power.** Mainstream language, no toolchain, hello-world in minutes. The single clearest adoption driver in the sample: Neovim's plugin "explosion" is attributed by its own team to *not having to learn Vimscript*; Obsidian (JS, no build step required) hit 8.1k plugins. | §1.3, §1.4 |
| P3 | **Sandbox by default; capabilities are named, denied-by-default, and accountable** (written justification for wildcards, public display of grants, dev vs prod separation). | Figma networkAccess; MV3 remote-code ban; K8s stateless CEL |
| P4 | **Stability discipline is written down and enforced.** Never break; deprecate with warnings, timelines, and never toward less-stable replacements; stage risk in gated/proposed tiers. | VS Code proposed API; Nvim signature contract; K8s deprecation rules #1–#8 |
| P5 | **Protect host performance by design, not by pleading.** Lazy activation, deferred work, declared budgets enforced by the host. | VS Code `onStartupFinished`; Chrome service workers; K8s CEL budget; Nvim filetype.lua |
| P6 | **One tier: the platform eats its own API.** Built-ins have no private path; every feature is a plugin using the public contract. | VS Code core-as-extensions; K8s (CRDs themselves are API objects); SpecForge R-1 mirrors this |
| P7 | **Declarative-first, imperative-second.** Where the domain allows, replace imperative interception with declared rules (K8s: CEL over webhooks for common checks; Chrome: declarativeNetRequest over blocking webRequest); keep arbitrary logic as the *second* tier, sandboxed and budgeted. | §1.5, §1.6 |
| P8 | **One runtime can serve many languages.** Neovim: Fennel/Teal/MoonScript compile to Lua; Shopify: Rust *or* JS compile to wasm. Language diversity is achieved at the compiler layer of a single enforced runtime. | §1.4, §1.7 |

## 3. Anti-patterns that kill (or wound) plugin ecosystems

| # | Anti-pattern | Evidence |
| --- | --- | --- |
| A1 | **Breaking changes / hard migration deadlines decoupled from replacement completeness.** Even necessary migrations visibly drain ecosystems when the new surface can't carry old workloads and the deadline doesn't move. | Firefox 57; MV3 |
| A2 | **Ambient authority + trust-by-review.** Full host access forces human review of every plugin (Obsidian's model) or store policy fights (MV3's motivation); the review step becomes the ecosystem bottleneck and the supply-chain attack surface. | §1.3, §1.6 |
| A3 | **Heavy authoring toolchain.** Requiring a compiler toolchain, SDK, and per-platform build to ship a hello-world suppresses the long tail (the pre-Lua Vimscript era; legacy Firefox XUL/SDK authors). | §1.4, §1.6 |
| A4 | **Declared-but-unenforced surface.** Fields/exports that exist in the contract but are ignored at runtime, and drift between spec and behavior, destroy author trust faster than missing features. (This is also SpecForge's audit signature: C7-03 no IDL, C7-09 `query_scope` ignored, C7-10 `max_execution_ms` unused, `validate__*` exports that never fire.) | D08; evidence.md §1.3 |
| A5 | **Plugins as a host-performance hazard.** Eager activation of every installed plugin (the pre-`activationEvents` world; Chrome background pages; Nvim `filetype.vim`) turns the extension itself into the host's latency problem and users start disabling extensions wholesale. | §1.1, §1.4, §1.6 |
| A6 | **First-party asymmetry.** Private APIs for built-ins (VS Code's proposed-API gate is a partial example even inside the best-in-class platform) or a trusted native tier for the host's own plugins makes the "all plugins equal" promise hollow and poisons third-party incentives. | §1.1, R-1 |
| A7 | **Manifest-by-execution.** When the only way to know what a plugin contributes is to run it, static verification, reproducible builds, and safe marketplace indexing all collapse. (K8s is the positive proof: the schema is data; Shopify's projection query is data.) | §1.5, §1.7 |

## 4. Application to SpecForge (spec validation + analysis plugins)

**Where SpecForge already matches the winning pattern (keep these):**

- `describe_*` manifests as static, embedded JSON = P1/A7-compliant; the K8s lesson says go *further*
  (mandatory, schema-checked structural validation — the `apiextensions/v1` move), not sideways.
- R-1 (no first-party tier) = P6; VS Code shows it is achievable at scale, and its absence is A6.
- Signed registry with verifiable blobs = a P3 distribution control few ecosystems even have.
- Declarative `validation_rules` executed host-side = exactly K8s's CEL tier (P7): deterministic,
  object-scoped (entity-scoped), snapshot-testable (R-6). The C6-11 fix (host-side dispatch of custom
  rules) accidentally aligns with the strongest precedent in the sample.
- Shopify Functions is an independent replication of SpecForge's workload model (wasm + JSON in/out +
  declarative input + perf-critical Rust guests) at 10⁶-merchant scale.

**Where the audit record shows SpecForge violating the patterns (the real gaps):**

- **A4 is SpecForge's disease, not "wasm".** No IDL (C7-03), declared-but-ignored `query_scope`/
  `max_execution_ms` (C7-09/C7-10), `validate__*` surface that never fires, and a drift-catching *sync
  test* standing in for a schema. Every successful platform in §1 enforces its contract by construction
  (compiled CEL, WIT-style IDLs, type-checked manifests). Switching interpreters does not fix this —
  D08's WIT analysis stands on this outside evidence too.
- **A5 is one audit finding away:** C7-10 (30s timeout declared, never enforced) is precisely the
  "declared budget, no enforcement" failure K8s solved with compile-time cost estimation + runtime
  budgets; Figma solved it with a modal cancel button. Enforce declared budgets or delete them from the
  contract.
- **P3 inverted in one spot:** C7-04 (fs access allow-by-default) contradicts the Figma/K8s norm —
  capabilities must be deny-by-default, and Figma adds the accountability trick worth copying: a
  wildcard grant requires a written `reasoning`, and grants are shown to users (SpecForge could surface
  capability grants at `specforge add` time).
- **P2 is the adoption risk.** The current authoring path (Rust crate + wasm32 target + SDK macros +
  vendored blobs) is the heaviest hello-world in the sample — heavier than every *successful* ecosystem's
  and comparable to the *failed* Firefox-era native path (A3). The mitigating fact: SpecForge's primary
  author is AI coding agents (evidence.md §1.5), for whom the cost is toolchain/build latency and
  verifiability, not language familiarity — and K8s shows a declarative-first tier (schema + CEL ≈
  SpecForge's manifest + declarative rules) can carry most of an ecosystem *without* a scripting
  runtime, reserving the code tier for genuine analysis logic (SpecForge's compiler passes ≈ K8s
  controllers/webhooks).

**Runtime-agnostic prescriptions (what the pattern evidence says to do regardless of the final verdict):**

1. Give the protocol a real IDL; generate host + SDK types from it; delete the sync-test shims (fixes
   C7-03 structurally and makes A4-class drift impossible).
2. Make the manifest mandatory-and-structural like `apiextensions/v1`: unknown fields pruned or
   rejected, budgets declared and *enforced*, capability grants explicit.
3. Two-tier validation per K8s: declarative rules (stateless, entity-scoped, host-executed, budgeted)
   as tier one; sandboxed `validate__*`/pass code with enforced CPU/memory limits as tier two. Never
   allow tier two to silently replace tier one.
4. Written deprecation policy (K8s §1.5 is the template): additive evolution, `Warning`-style
   diagnostics on deprecated exports, never deprecate toward a less-sandboxable surface.
5. Whatever runtime wins: raise the floor for AI-agent authors — scaffolded templates, schema-validated
   manifests, single-command build — because P2/A3 says friction, not language choice, is what chokes
   ecosystems. And per P8, add new *languages* only as compile targets of the one runtime, not as new
   runtimes.

## Bottom line

Across five successful systems and two wounded ones, the correlates of adoption are consistent:
a static declarative manifest as the contract, deny-by-default named capabilities, written and enforced
stability/budget guarantees, one tier where built-ins use the public API, and above all low authoring
friction. The killers are unenforced declared surface, breaking migrations, ambient authority, and
manifest-by-execution. SpecForge's architecture (declarative manifests, wasm sandbox, signed registry,
R-1 parity) matches the successful pattern — Shopify Functions is the same design validated at scale —
while its audit findings are all instances of the *contract-not-enforced* anti-pattern, which is
independent of the interpreter choice. From the API-design dimension the evidence favors one enforced
runtime with a real IDL and a friction-reduced authoring path over adding a second runtime to buy
ergonomics (Neovim's own history: one Lua runtime, many compile-to-lua languages).

## Verdict

**Verdict:** KEEP_WASM
**Confidence:** 4
**One-line rationale:** Every ecosystem winner in this study enforced a static, sandboxed, declarative
contract and cut authoring friction at the toolchain layer — SpecForge should fix its IDL/budget/
capability gaps inside the wasm model (as Shopify Functions did for the identical workload shape) rather
than add a second runtime.
