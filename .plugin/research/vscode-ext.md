# Case Study: VS Code Extension Architecture

Researcher: `research-vscode-ext` · 2026-09-27 · Sources: primary VS Code docs/release notes/source + incident reporting. Feeds the KEEP_WASM vs embedded-runtime vs MULTI decision (`.plugin/decision.md`).

## 1. How VS Code sandboxes extensions

**The unit of isolation is a process, not a capability grant.** Extensions never run inside the UI. The renderer (workbench window) is Electron-sandboxed (no Node.js), and all extension code runs in a separate **Extension Host**: "a process that runs all the installed extensions isolated from the renderer process. There is one extension host per opened window" ([Migrating VS Code to Process Sandboxing, Nov 2022](https://code.visualstudio.com/blogs/2022/11/28/vscode-sandbox)).

Key mechanics:

- **One host per window, all extensions share it.** The host is not per-extension; every installed extension in a window lives in the same OS process.
- **UtilityProcess migration.** The extension host was originally forked from the renderer over Node.js sockets. Microsoft contributed a new Electron `UtilityProcess` API specifically to host it: requirements were "isolated process with support for spawning child processes, full Node.js support, message ports for direct IPC" ([sandbox blog](https://code.visualstudio.com/blogs/2022/11/28/vscode-sandbox)). Running the host in a utility process became default in v1.75 ([Jan 2023 release notes](https://code.visualstudio.com/updates/v1_75), "Utility process for extension host").
- **IPC by MessagePort.** Renderer ↔ extension host talk over MessagePorts so a busy main process (user input) is never involved; all `vscode` API calls are therefore async RPC across the boundary.
- **Declared hosts and runtimes.** There are three hosts — local (Node.js), web (Browser WebWorker), remote (Node.js in container/SSH) — selected by manifest `extensionKind` and `main`/`browser` entry points ([Extension Host docs](https://code.visualstudio.com/api/advanced-topics/extension-host)).
- **Stated goals are stability and performance, not security:** the host "prevents extensions from impacting startup performance, slowing down UI operations, modifying the UI," plus lazy loading via declared activation events ([Extension Host docs](https://code.visualstudio.com/api/advanced-topics/extension-host)).

## 2. What language extensions use

**TypeScript/JavaScript, exclusively.** Desktop/remote extensions are JS running on Node.js; web extensions are JS in a WebWorker ([Extension Host docs](https://code.visualstudio.com/api/advanced-topics/extension-host)). The API surface is a JS object (`require('vscode')`), typed by the shipped `vscode.d.ts`.

The web host is the interesting variant: it is the closest VS Code gets to a capability sandbox. Web extensions get **no Node APIs, no module loading (single-file bundle enforced), no child processes or executables, file access only through the virtual `vscode.workspace.fs` API, and network only via CORS-constrained `fetch`** ([Web Extensions guide](https://code.visualstudio.com/api/extension-guides/web-extensions)). That is the imports-are-capabilities model — but it exists **only** in the browser runtime. The desktop host, where most extensions run, grants full Node.js.

## 3. Pros and cons of the separate-process model

**Pros (why it won):**

1. **UI isolation.** Extension CPU work can never block the renderer; the UI thread never executes extension code ([Extension Host docs](https://code.visualstudio.com/api/advanced-topics/extension-host)).
2. **Crash containment.** A crashed or hung extension host kills the host process, not the window; the host can be restarted (`Developer: Restart Extension Host`) without losing editor state.
3. **Lazy activation.** Extensions declare activation events (inferred from `contributes` since v1.74), so install does not imply load ([v1.74 notes](https://code.visualstudio.com/updates/v1_74)).
4. **Placement flexibility.** The same process model maps cleanly onto remote machines and the browser — the reason vscode.dev works at all.
5. **Debuggability.** One process to profile/attach ("Show Running Extensions", process explorer), rather than N.

**Cons:**

1. **It is a reliability boundary, not a security boundary.** The host requires "full Node.js support" and extensions "are free to spawn as many child processes as they require" ([sandbox blog](https://code.visualstudio.com/blogs/2022/11/28/vscode-sandbox)). Microsoft's own docs admit the limit: *"Workspace Trust can't prevent a malicious extension from executing code and ignoring Restricted Mode. You should only install and run extensions that come from a well-known publisher that you trust."* ([Workspace Trust docs](https://code.visualstudio.com/docs/editing/workspaces/workspace-trust)).
2. **IPC tax and asynchrony everywhere.** Every host call marshals across a process boundary and must be async; bulk data (documents, outlines) pays serialization twice. VS Code spent years migrating IPC from Node sockets to MessagePorts ([sandbox blog](https://code.visualstudio.com/blogs/2022/11/28/vscode-sandbox)).
3. **Shared-host contention.** One misbehaving extension's event-loop stall degrades every extension in the window. Microsoft's acknowledgment is the post-hoc `extensions.experimental.affinity` setting — "Configure an extension to execute in a different extension host process" ([source: extensions.contribution.ts](https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/extensions/browser/extensions.contribution.ts)) — plus the 2025-era split into per-affinity hosts. There is still **no per-extension CPU/memory cap**.
4. **Memory overhead.** Each window pays a full V8/Node heap for the host; web-host parity required a second, capability-restricted code path and a single-file bundling constraint that authors must work around ([Web Extensions guide](https://code.visualstudio.com/api/extension-guides/web-extensions)).

## 4. How the marketplace handles security

Because the runtime does not restrain extensions, VS Code pushes security to **distribution-time controls plus user trust**:

- **Repository signing + install-time verification.** All extensions uploaded to the Marketplace are signed by the Marketplace since mid-November 2022; VS Code verifies the signature on every install/update, with `extensions.verifySignature` as opt-out ([v1.74 notes](https://code.visualstudio.com/updates/v1_74)); publishing signed by default from v1.75 ([v1.75 highlights](https://code.visualstudio.com/updates/v1_75)). Failure yields a structured error taxonomy (`PackageIntegrityCheckFailed`, `CertificateRevoked`, `NotSigned`, …) ([docs](https://code.visualstudio.com/docs/configure/extensions/extension-marketplace)).
- **The verification pipeline is operationally fragile.** In v1.77 (March 2023) Microsoft **disabled** signature checking because Marketplace-side bugs produced false positives blocking valid installs ([v1.77 notes](https://code.visualstudio.com/updates/v1_77)) — an honest data point on what running signing infrastructure costs.
- **Trust guidance instead of enforcement.** With no runtime defense, the documented answer to malicious extensions is publisher reputation and Workspace Trust prompts ([Workspace Trust docs](https://code.visualstudio.com/docs/editing/workspaces/workspace-trust)).
- **Does it work? The GlassWorm record says: partially, at best.** A year-long campaign (still active in 2026) hit **both** Open VSX and the Microsoft Marketplace:
  - Oct 2025: self-propagating worm, invisible Unicode (variation-selector) code, Solana-blockchain C2 with Google Calendar fallback; steals Open VSX/GitHub/Git credentials, drains ~49 crypto-wallet extensions, republishes itself through compromised publishers ([The Hacker News, Oct 24 2025](https://thehackernews.com/2025/10/self-spreading-glassworm-infects-vs.html); ~35,800 downloads across 7 extensions per [Truesec](https://www.truesec.com)).
  - Leaked publisher tokens in extension repos forced the Eclipse Foundation to revoke Open VSX tokens ([The Hacker News, Oct 31 2025](https://thehackernews.com/2025/10/eclipse-foundation-revokes-leaked-open.html)).
  - Waves continued: 3 more extensions (Nov 2025), 24 typosquats of Flutter/React/Vue tooling (Dec 2025), compromised maintainer account pushing poisoned updates to extensions with 22k+ downloads (Feb 2026), 72 extensions abused via `extensionPack`/`extensionDependencies` for transitive delivery (Mar 2026), until a CrowdStrike/Google/Shadowserver C2 takedown in May 2026 ([Nov](https://thehackernews.com/2025/11/glassworm-malware-discovered-in-three.html), [Dec](https://thehackernews.com/2025/12/glassworm-returns-with-24-malicious.html), [Feb](https://thehackernews.com/2026/02/open-vsx-supply-chain-attack-used.html), [Mar](https://thehackernews.com/2026/03/glassworm-supply-chain-attack-abuses-72.html), [May 2026](https://thehackernews.com/2026/05/glassworm-malware-takedown-disrupts.html)).

**Verdict on the model:** signing and takedowns are reactive, detection-shaped controls. Every GlassWorm payload executed with full user privilege the moment the extension activated — no signature check undoes that. The industry's most popular extension ecosystem has, after a decade, *no runtime capability restriction on desktop*.

## 5. What SpecForge should learn

Context: SpecForge loads `.wasm` plugins through Extism/Wasmtime (`crates/specforge-extism/src/runtime.rs` — `ExtismRuntime` with a shared `HostContext`, host functions `host_emit_diagnostic`, `host_read_file_check`, `host_query_graph` in `src/host_context.rs`), and `.plugin/decision.md` keeps KEEP_WASM with audit gaps C7-02/03/04/08/09/10.

1. **Validate the core bet: process isolation is not capability security.** VS Code is the strongest existence proof that "run third-party code out-of-process in a real language runtime" still ends with the vendor writing *"only install extensions from a publisher you trust"* on a security docs page. SpecForge's imports-are-capabilities model (R-2) is the property VS Code lacks; GlassWorm is what fills that gap in practice. Finishing C7-04 (fs deny-by-default) is the VS Code lesson applied — VS Code's fs story is allow-everything.
2. **Ship distribution-time controls anyway; they are complementary.** Wasm containment stops *runtime* abuse but not credential theft from the *publishing* side (the Eclipse token revocations hit a wasm-free ecosystem, but the pattern — leaked publish tokens — applies to any registry). `specforge-registry-server` should sign artifacts and verify at load, and expect false-positive operational pain (v1.77) — keep an explicit, logged override rather than a silent default-off.
3. **Manifest-declared lazy activation works; copy it.** Activation events + implicit inference (v1.74) kept install cost at zero. SpecForge manifests already declare triggers — keep loading lazy and declare triggers structurally, not imperatively.
4. **Type the host boundary before the API accretes.** VS Code's boundary is an untyped-in-practice JS proxy that grew for a decade and forced an IPC-migration project. SpecForge's three host functions are narrow today; locking them into a WIT IDL (C7-03) preserves that and makes hallucinated host APIs a compile error (decision.md D07).
5. **Plan for the shared-host contention problem now, not after the fact.** `ExtismRuntime` holds `Mutex<HashMap<String, LoadedPlugin>>` with one shared `HostContext` — the same "one host, all extensions" shape that forced VS Code to bolt on `affinity`. Per-plugin warm pools (C7-08) avoid head-of-line blocking; fuel/epoch limits (C7-10) give the per-plugin CPU cap VS Code never built.
6. **One artifact across runtimes beats N runtimes.** VS Code maintains Node.js and WebWorker extension stories with different capabilities and a bundling burden, splitting its ecosystem. A single wasm artifact runs identically in CLI, server, and (via Wasmtime/browser runtimes) web hosts — the MULTI-runtime counterfactual re-creates VS Code's split inside SpecForge.
7. **Do not copy the trust fallback.** "Workspace Trust can't prevent a malicious extension" is the sentence SpecForge's architecture exists to make impossible to write. Traps are per-call, contained failures; a compromised plugin in SpecForge can emit garbage diagnostics but cannot open a socket — that delta is the whole decision.

## Bottom line

**KEEP_WASM** — confidence **5** — VS Code, the world's most successful plugin platform, proves that process isolation plus marketplace signing still ends in trust-based security (GlassWorm), while SpecForge's capability-restricted wasm runtime provides the enforcement layer VS Code's architecture structurally cannot.
