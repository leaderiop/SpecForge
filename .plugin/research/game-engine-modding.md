# Game-Engine Modding Languages — Case Study for the Plugin Runtime Decision

**Analyst:** research-game-modding · **Date:** 2026-09-27 · **Method:** web research on primary sources
(luau.org, lua.org, warcraft.wiki.gg, Blizzard's Lua Workshop paper, learn.microsoft.com, Roblox newsroom,
Wikipedia), mapped onto SpecForge's requirements R-1..R-6 (see
[decision-brief.md](../decision-brief.md)).

The question this case study answers: four of the most successful plugin/mod ecosystems in software history
chose four different language models — what actually drove their outcomes, and what transfers to a technical
tool choosing a plugin runtime?

| Platform | Plugin model | Language | API origin | Scale of outcome |
| --- | --- | --- | --- | --- |
| Roblox | official, sandboxed, curated | Luau (managed fork of Lua 5.1) | purpose-built | creators earned **$1.5B in 2025** ($923M in 2024) |
| Minecraft: Java Edition | unofficial, unmanaged, full access | Java (the host's own language) | reverse-engineered bytecode | **200,000+ mods** on CurseForge (2025) |
| Unity | official, managed runtime | C# (sole language since 2017) | purpose-built | 15,000 new projects/day; Asset Store ~40M downloads by 2018 |
| World of Warcraft | official, sandboxed, curated | Lua 5.1 embedded in C++ client | purpose-built | ~10M Lua end-users (2008); addons still maintained 20+ yrs on |

---

## 1. The four case studies

### 1.1 Roblox / Luau — when your plugin language *is* your business

Roblox adopted plain Lua 5.1 around **2006**. Over a decade it accumulated sandbox hardening and library
tweaks; then, as its internal codebase (written in the same scripting language) grew and professional
studios replaced hobbyists, it reshaped Lua 5.1 into **Luau**: rewritten compiler/interpreter, a gradual
type system, linting, and a sandbox designed to hold against "actively malicious" code. Critically,
Roblox **could not afford breaking changes** — it kept the Lua 5.1 semantic baseline for ecosystem
compatibility — and rejected LuaJIT as a base for reasons of portability, ease of change, and "robust code
at scale" ([luau.org/why](https://luau.org/why)). Luau was open-sourced under MIT in November 2021
([Wikipedia](https://en.wikipedia.org/wiki/Luau_(programming_language)),
[GitHub](https://github.com/luau-lang/luau)).

**Sandbox (the part most relevant to R-2).** Safety is enforced at the interpreter/VM level, not by host
policy flags ([luau.org/sandbox](https://luau.org/sandbox)):

- `io.*`, `package.*`, `dofile`, `loadfile` **removed**; `os.*` reduced to `clock/date/difftime/time`;
  `debug.*` mostly removed.
- Loading **bytecode** is removed entirely (untrusted bytecode is unvalidatable); hosts must sign bytecode.
- Globals are made readonly by a **VM-level write barrier**; each script gets its own global table
  (`__index` fallback to the builtin one), so scripts cannot monkey-patch or see each other.
- `__gc` removed (host-only tag destructors) because finalizers break isolation and memory safety.
- A **VM interrupt hook** lets the host terminate any runaway script "eventually... at any function call or
  loop iteration" — Roblox runs a watchdog (10 s limit in Studio, interrupt-on-shutdown). Memory limits are
  host-configurable.
- The Luau project is explicit that the sandbox is **not formally proven** — the stack is C++, safety comes
  from removal + fuzzing + deliberate design, not guarantees.

**Dogfooding.** "Luau is used by Roblox game developers to write game code, and by Roblox engineers to
implement large parts of the user-facing application code as well as portions of the editor (Roblox Studio)
as plugins" ([README](https://github.com/luau-lang/luau)). There is no native plugin tier: Roblox's own app
code runs on the same sandboxed VM as a 13-year-old's game script.

**Types when it mattered.** Luau's gradual type system ([luau.org/typecheck](https://luau.org/typecheck))
offers `nocheck` / `nonstrict` (default) / `--!strict` modes, structural typing, and checked casts — added
precisely because the author population shifted from novices to professional studios and Roblox's own
engineers. `luau-analyze` + the community `luau-lsp` give IDE-grade tooling (README). Roblox now ships
generative-AI creation tools inside Studio ([Wikipedia](https://en.wikipedia.org/wiki/Roblox)) — authoring
by AI on a typed, sandboxed language is already the Roblox production path.

**Outcome.** >2M creators using Studio per year (2020, majority minors); ~345k earning via DevEx by 2020;
creators earned **$923M in 2024 and over $1.5B in 2025**; 42k DevEx participants with median ~$1,500/yr;
83% of US creators are individuals, not studios
([about.roblox.com, Sep 2026](https://about.roblox.com/newsroom/2026/09/global-impact-of-creation-on-roblox),
[Wikipedia](https://en.wikipedia.org/wiki/Roblox)). Since 2008 Roblox has created **none of its own games**
— the platform is 100% third-party content ([Wikipedia](https://en.wikipedia.org/wiki/Roblox)). The
language investment became a product in its own right: Luau has been adopted by Alan Wake 2, Warframe,
Second Life, and Farming Simulator 2025 ([README](https://github.com/luau-lang/luau)).

### 1.2 Minecraft / Java — maximum power, zero governance, ecosystem of 200k mods

Java Edition has **no official mod API**. The community decompiles the game's Java bytecode (MCP), and
loaders (Forge 2011, Fabric 2018, later NeoForge/Quilt forks) weave mods together with shared mappings
([Wikipedia: Minecraft modding](https://en.wikipedia.org/wiki/Minecraft_modding)). Mods run with **full
process privileges** — no sandbox at all. The result: **200,000+ mods on CurseForge as of March 2025**, a
mod scene widely credited as a reason Minecraft became the best-selling game ever, and Notch's own
conversion: mods are "a huge reason of what Minecraft is" ([Wikipedia](https://en.wikipedia.org/wiki/Minecraft_modding)).

Costs of this model, all realized:

- **Supply-chain incidents**: "Fractureiser" (June 2023) — compromised CurseForge accounts pushed malware
  into popular modpacks; "BleedingPipe" (July 2023) — RCE in Forge mods via unsafe Java deserialization,
  latent since 2017 ([Wikipedia](https://en.wikipedia.org/wiki/Minecraft_modding)). unsigned distribution +
  unsandboxed execution = exactly the attack SpecForge's signed registry (R-4) exists to prevent.
- **Governance fragmentation**: Forge→NeoForge fork (2023), Fabric→Quilt fork (2022) — ecosystems fork when
  governance fails, even around the *same* language ([Wikipedia](https://en.wikipedia.org/wiki/Minecraft_modding)).
- Mojang hired the Bukkit team in 2012 to build an official API; it effectively never shipped, and Mojang's
  eventual fix was to **remove obfuscation** (Oct 2025) rather than to build a plugin API.

**The natural experiment: Bedrock Edition.** The C++-based Bedrock cannot be decompiled-and-patched, so
Microsoft built the **official** add-on system: declarative JSON data plus a sandboxed **JavaScript Script
API** ([learn.microsoft.com](https://learn.microsoft.com/en-us/minecraft/creator/scriptapi/), TS-typed,
IDE-integrated). Same player audience, better distribution (curated Marketplace, ~$350M paid to creators by
2021), far weaker outcome as a modding *culture*: "addons in Bedrock Edition have less flexibility and
features because they can only modify features that Mojang explicitly exposes"
([Wikipedia](https://en.wikipedia.org/wiki/Minecraft_modding)). Java modding won on **expressive power**
(deep host access), Bedrock won on **safety and distribution**. Neither dominates; the ecosystem lives
mostly on the Java side.

### 1.3 Unity / C# — the professional's choice, and the cost of betraying it

Unity chose a **managed runtime (Mono) and C#** as its single scripting language — the same language for
game code, editor plugins, and third-party assets. It shipped with three languages (C#, UnityScript, Boo)
and **consolidated to one**: Boo removed with Unity 5, UnityScript deprecated in August 2017
([Wikipedia: Unity](https://en.wikipedia.org/wiki/Unity_(game_engine))). One language meant one tooling
target (IDE debugger, packages, a giant learning corpus) — C# matched its audience: professional and
semi-professional developers. Scale: 1.3M developers by 2012; by 2020, Unity software on 1.5B+ devices,
half of all mobile games, 15,000 new projects/day; Asset Store (~40M downloads by 2018) — a curated,
monetized distribution channel for third-party code and content
([Wikipedia](https://en.wikipedia.org/wiki/Unity_(game_engine))).

The cautionary half: the 2023 **runtime fee** episode. A unilateral economics change triggered public
backlash, mass migration threats, a 60% stock decline, cancellation of the fee — and measurable ecosystem
decay: Global Game Jam Unity usage fell 61%→36% in one year, and by 2026 Godot overtook Unity at the GMTK
jam ([Wikipedia](https://en.wikipedia.org/wiki/Unity_(game_engine))). **The language was never the
problem; trust in the platform's governance was.** An ecosystem takes a decade to build and one pricing
page to unwind.

### 1.4 WoW / Lua — two decades of interpreter-level sandboxing under R-1-like pressure

WoW embedded Lua (5.0, upgraded to 5.1 with the 2007 expansion) to define its **entire user interface**.
In 2008 Blizzard's own UI was **142 Lua files / 66,357 lines**, plus 68k lines of XML layout — and that
code ships with the client and "serves as model implementation for all aspects of the user interface and
API" ([Whitehead, Lua Workshop 2008](https://www.lua.org/wshop08/lua-whitehead.pdf)). With ~10M
subscribers, WoW was billed as "10,000,000 Lua users and growing" — the single largest embedded-Lua
deployment ever.

**Sandbox (R-2 precedent).** Addon code gets **no file I/O, no OS API**; persistence is buffered
saved-variables applied at login/UI-reload; input requires hardware events; combat actions are locked
during combat ([Whitehead 2008](https://www.lua.org/wshop08/lua-whitehead.pdf)). Then WoW added something
no other platform has: **taint tracking** — a provenance flag on *every Lua value and execution path*
([Warcraft Wiki](https://warcraft.wiki.gg/wiki/Secure_Execution_and_Tainting)). Blizzard's signed code is
untainted; addon code is tainted; "protected" functions (spell casting, targeting) fail when called from a
tainted path. Introduced in patch 2.0 (2006) after addons automated gameplay; Blizzard's stated position:
"we don't want UI mods to make combat-sensitive decisions for players." Addons influence play only through
hardware events the *player* commits. This is capability control by **data provenance inside one runtime**
— not by a native/trusted execution tier.

**The cost of the sandbox, honestly stated.** With `package/require` removed, the community reimplemented
compression (LZW), hashing and crypto (MD5, RC4, TEA, SHA-256+RSA with a bignum library), serialization,
and a versioning layer (LibStub — "our own DLL-hell") **in pure Lua**
([Whitehead 2008](https://www.lua.org/wshop08/lua-whitehead.pdf)). A capability-starved runtime pushes
complexity into the plugin ecosystem.

**Outcome.** A 20+ year old, still-active addon ecosystem (BigWigs: 2.8M downloads on WoWInterface alone,
updated within days of the current patch — [wowinterface.com](https://www.wowinterface.com/downloads/index.php?cid=151)),
and Lua as ["the leading scripting language in games"](https://www.lua.org/about.html). Note the tiering
nuance: Blizzard's own UI is *signed* (secure) while addons are tainted — a two-trust-level system — but
both tiers are **the same language on the same VM, and the trusted tier is itself written in the plugin
language**. The privilege boundary is a runtime-enforced property of data, not a separate execution
mechanism.

---

## 2. What makes game modding ecosystems thrive

Ranked by the strength of evidence across the four cases:

1. **Audience + distribution beat language virtues.** Minecraft has the *worst* security and governance of
   the four and the *largest* mod count, because every player owns the host language's toolchain and
   CurseForge solved distribution. Roblox reached millions of creators because Studio is free and payout is
   built-in — with a language most authors had never heard of before. No case shows language popularity
   driving adoption; every case shows audience + install path doing so.
2. **The host API is the ecosystem; the language is the delivery vehicle.** The WoW API (protected
   functions, events, secure templates), Roblox's Instance API, Unity's MonoBehaviour model — authors
   bind to *these*. Language choice determines who can cross the bridge and how safely; API design
   determines whether the destination is worth reaching.
3. **Iteration speed is table stakes.** WoW's `/reload` re-runs addon code in seconds; Roblox syncs
   scripts into running sessions; Unity's play mode recompiles in seconds. A plugin loop that requires a
   recompile-and-redeploy cycle loses to one that doesn't (R-5 for SpecForge).
4. **Sandboxing must live in the runtime, not in host policy.** Both Lua winners enforce capabilities at
   the interpreter level (WoW: no I/O APIs at all + taint; Luau: library removal + VM write barriers +
   interrupt hooks). Every platform where the *host* was supposed to police plugins via convention
   (Minecraft's trust-the-mod model) eventually had a malware incident (Fractureiser, BleedingPipe).
5. **Types arrive when the author population professionalizes.** Unity chose a typed language upfront for
   professionals. Roblox started untyped-for-kids and *added* a gradual type system when professionals,
   internal teams, and scale demanded robustness. Both converge on: gradual typing, editor tooling, and
   analysis-as-a-CLI (`luau-analyze`) as the ecosystem matures.
6. **Signed, curated distribution prevents the class of failure every unmanaged ecosystem hits.**
   Roblox's marketplace and WoW's in-client addon manager vs. CurseForge account compromises. SpecForge
   already has the right architecture here (R-4); game history validates it as necessary, not optional.
7. **One scripting surface; sprawl gets consolidated.** Unity killed UnityScript and Boo. Roblox kept one
   language baseline (Lua 5.1) for 20 years and forked *within* it rather than fragmenting. Minecraft's
   loader wars (Forge/NeoForge, Fabric/Quilt) show fragmentation is the default failure mode even
   *within* a single language — multi-runtime ecosystems multiply that risk.

## 3. What the language choice actually determines

Language choice does **not** determine whether an ecosystem exists (all four thrived). It determines:

- **Who can author.** Lua/JS: kids, hobbyists, designers, and — today — AI code generators. Full host
  language (Java/C#): only people who accept the toolchain. Compiled intermediate formats (wasm binaries
  as the *authoring* surface, not just distribution format) sit at the restrictive end: authors need a
  compiler toolchain before their first `hello world`. No major game plugin ecosystem has ever required
  plugin authors to operate a Rust/C++→wasm toolchain; every successful one runs *source* through a
  host-controlled compiler/loader at load or install time.
- **What the host must do to stay safe.** With a capability-starved interpreter (WoW, Luau), the runtime
  itself is the sandbox and "deny-by-default" is enforced by omission of APIs — it cannot regress via a
  config bug. With a full-capability runtime, the host must implement and *correctly configure* an
  enforcement layer (wasm imports, permissions) — which is exactly the class of bug SpecForge's audit
  found (C7-04: sandbox `file_system_access` allow-by-default; C7-10: `max_execution_ms` never enforced).
  WoW and Luau make those bugs structurally impossible; their open problems are different (WoW's taint
  leaks, Luau's unproven C++ stack).
- **What "builtins are plugins too" costs (R-1).** When the plugin language is expressive enough for the
  host's own features, dogfooding is *free*: Blizzard's UI (66k LoC) and large parts of Roblox's app run
  in the plugin language, on the same VM, under the same API. When the plugin format is a compiled
  target, "everything is a plugin" generates parallel implementations — SpecForge's C7-11 (three parallel
  implementations of the extension concept) is precisely this cost.
- **Determinism and memory safety, where interpreters are weaker.** The honest counterweight: a VM like
  wasmtime gives memory isolation and (with fuel/epoch interruption) hard determinism guarantees that a
  C/C++ interpreter cannot match; luau.org concedes its sandbox is "not formally proven." No game needed
  determinism; SpecForge does (R-6). This is the one axis where the game evidence has nothing to say in
  favor of the scripting languages — games never carried a snapshot-testing analyze pipeline.

## 4. Transfer to SpecForge

Mapping each lesson to SpecForge's constraints (R-1..R-6) and the known audit record:

| Game-history lesson | SpecForge implication |
| --- | --- |
| Interpreters enforce sandbox by omission (WoW, Luau) | R-2 is best satisfied by a runtime where "no fs/net" is *structural*, not configured. Directly addresses C7-04 (fs allow-by-default) and C7-10 (timeouts unenforced) — a Lua VM with no io/os libraries cannot leak fs; CPU caps need the Luau-style interrupt hook (mlua has memory + hook support). |
| Host dogfoods the plugin language (WoW UI, Roblox app) | R-1's cheapest fulfillment: rewrite the four builtins' logic in the plugin language so there is exactly one implementation — eliminating the C7-11 duplication and the vendored-blob/native-mirror sync machinery (C7-00, C7-06) by construction. |
| Types arrive with professionalization; AI authors benefit more | The AI-agent author audience (SpecForge's primary consumer) leans on machine-checkable structure: keep the declarative `describe_*` manifest as data (SpecForge already has this) and reserve the scripting surface for `validate__*`/passes/collectors. WoW's split (declarative TOC+XML for structure, Lua only for behavior) is the same shape. |
| Plugin languages end up hosting the host's own code | A language chosen for third parties must be one the SpecForge team will accept for builtins. Lua was chosen by Blizzard/Roblox for exactly this dual role; no game platform has standardized on wasm as a plugin authoring surface. |
| Multi-language/surface ecosystems fragment (Unity consolidation; Minecraft loader wars) | `MULTI` carries fragmentation risk with no observed game-ecosystem payoff; every successful platform converged on one scripting surface + one declarative data format. |
| Unsigned distribution eventually ships malware (Fractureiser) | R-4's signed registry is validated as necessary by game history — keep it regardless of runtime choice. |
| Determinism/memory-safety is the scripting languages' weak axis | R-6 (snapshot-testable determinism) and memory isolation are the two claims where KEEP_WASM is strictly stronger than any C-based interpreter. Game history cannot arbitrate this; it must be decided on SpecForge's own analysis-pipeline needs. |
| Ecosystems die by governance, not language (Unity) | Whichever runtime is chosen, the registry's rules (signing, review, compatibility policy) matter more to ecosystem survival than the VM. |

**Workload fit.** SpecForge's own evidence (evidence.md §2) shows the builtin guest payload is ~97%
generated manifest, ~3% real logic (formal's four passes, several hundred Rust lines each). WoW's addon
workload — thousands of-addon event-driven UI logic under frame budgets — is strictly harder than
SpecForge's per-entity validation and graph passes, and it ran on Lua 5.x for 20 years. The workload is
not the bottleneck; authoring surface and sandbox enforcement are.

## Bottom line

Twenty years of game-modding history show that thriving plugin ecosystems are built on: (a) a
source-distributed, capability-starved scripting language the host itself dogfoods, (b) runtime-level —
not policy-level — sandbox enforcement, (c) one scripting surface, and (d) signed curated distribution.
Two of the four platforms (WoW, Roblox) are the largest embedded-Lua deployments in existence and each
rewrote or invested in the language specifically to make it a safe, typed, high-scale plugin surface;
none standardized on a compile-to-wasm authoring model; the multi-language platform (Unity) spent a
decade consolidating to one. For SpecForge this history argues the end-state should be a single embedded
scripting language (the Lua family being the directly evidenced choice, via mlua) carrying *all* plugins
including the builtins — with the explicit caveat that game history cannot arbitrate the two axes where
wasm is objectively stronger: memory-isolation guarantees and deterministic execution for R-6. If those
two axes dominate the evaluation, KEEP_WASM survives this case study; if ecosystem breadth and R-1
integrity dominate, LUA does. What the evidence does rule out is preserving three parallel
implementations, and it argues against MULTI as a destination.

**Verdict:** LUA
**Confidence:** 3/5
**One-line rationale:** Every large-scale, long-lived, safely sandboxed game plugin ecosystem in history
ran on interpreter-enforced embedded scripting — twice on Lua, with the host's own code dogfooding the
plugin language — while no successful game ecosystem ever required a compiler toolchain from plugin
authors; but determinism and memory-isolation guarantees, which games never needed and SpecForge does,
are the axes where wasm remains stronger and keep this short of high confidence.
