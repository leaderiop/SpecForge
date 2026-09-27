# Case Study: Obsidian's No-Sandbox Plugin Ecosystem

**Fleet:** SpecForge plugin-runtime decision (KEEP_WASM vs embed Lua/Python/TypeScript vs MULTI)
**Date:** 2026-09-27 · **Agent:** research-obsidian-plugins
**Method:** primary sources (Obsidian docs/blog, official plugin registry git history) + incident reports. All numbers computed or read directly from cited sources.

---

## 1. How Obsidian handles community plugins

Obsidian plugins are **plain JavaScript bundles (`main.js`) executed inside the Electron host with the full Node.js API**. There is no permission system and no isolation boundary. Obsidian's own docs state: *"Due to technical limitations, Obsidian cannot reliably restrict plugins to specific permissions or access levels... Community plugins can access files on your computer, can connect to internet, can install additional programs"* ([Plugin security, obsidian-help `en/Extending Obsidian/Plugin security.md`](https://raw.githubusercontent.com/obsidianmd/obsidian-help/master/en/Extending%20Obsidian/Plugin%20security.md)).

Compensating controls, all process-based rather than mechanism-based:

| Control | Detail | Source |
|---|---|---|
| Restricted Mode | Third-party code disabled by default; user must opt in per machine | [Plugin security](https://help.obsidian.md/plugins/restricted-mode) |
| No auto-updates | Plugin updates are manual, "for security purposes" | [Community plugins help](https://help.obsidian.md/community-plugins) |
| Remote kill switch | App fetches a "plugin deprecations" file from GitHub every 12h to disable known-bad plugin versions | [Teams security doc](https://raw.githubusercontent.com/obsidianmd/obsidian-help/master/en/Teams/Security%20considerations%20for%20teams.md) |
| Review | Until May 2026: **manual human review of the initial submission only**; all later versions unreviewed. Since May 12, 2026: automated malware/quality scanning of **every version**, safety scorecards, manual review retained only for popular/featured/flagged plugins | ["The future of Obsidian plugins" (May 12, 2026)](https://obsidian.md/blog/future-of-plugins/) |
| Incoming | Capability disclosures (network/filesystem/clipboard), artifact attestation, verified authors, team allowlists, private plugin distribution | [future-of-plugins](https://obsidian.md/blog/future-of-plugins/) |

Note the confession in the official post: *"as Obsidian has grown in popularity we struggled to keep pace with submissions, and **subsequent versions were not reviewed**"* and *"as coding agents accelerate the creation of plugins, the review queue was only getting longer."* The automated launch flushed **2,300 queued submissions** in days.

## 2. Ecosystem size

From the official registry [`obsidianmd/obsidian-releases → community-plugins.json`](https://github.com/obsidianmd/obsidian-releases), entry counts computed from git history at each date:

| Date | Plugins |
|---|---|
| 2021-06 | 210 |
| 2022-06 | 574 |
| 2023-06 | 984 |
| 2024-06 | 1,686 |
| 2025-06 | 2,477 |
| 2026-01 | 2,705 |
| 2026-05-12 (directory relaunch) | 2,750 |
| **2026-09-27 (today, fetched live)** | **8,101** |

- **120 million total plugin downloads**, 4,000+ plugins/themes created since the 2020 API launch ([obsidian.md/blog/future-of-plugins](https://obsidian.md/blog/future-of-plugins/), May 2026).
- Growth regime change: ~500-600 plugins/year organically for five years, then **~5,350 additions in the 4.5 months** after automated review opened the gate — roughly a 3× jump, consistent with the 2,300-item queue flush plus AI-accelerated authoring.
- **68% of today's directory (5,510/8,101 entries) carries the label "This plugin has not been manually reviewed by Obsidian staff"** (computed from `community-plugins.json` description field).
- Enterprise reach: people in **10,000+ organizations** use Obsidian, including Amazon, Apple, Meta, Shopify, Microsoft, UK Government ([obsidian.md/enterprise](https://obsidian.md/enterprise), [free-for-work post](https://obsidian.md/blog/free-for-work/)).
- A single popular plugin (Tasks) has **3.4M downloads** ([ZeroQuarry](https://zeroquarry.com/research/obsidian-tasks-rce/), May 2026).

## 3. Security incidents

1. **PHANTOMPULSE RAT campaign (REF6598), April 2026.** Actors posing as VCs on LinkedIn/Telegram lured finance/crypto targets into a shared cloud vault; the victim was socially engineered into enabling "community plugins sync", which executed malicious versions of the legitimate *Shell Commands* and *Hider* plugins → PowerShell/AppleScript loader → in-memory RAT with C2 addresses resolved via Ethereum blockchain transactions ([CyberNetSec](https://cyber.netsecops.io/articles/obsidian-plugin-abused-in-campaign-to-deploy-phantom-pulse-rat/), [The Hacker News](https://thehackernews.com/2026/04/obsidian-plugin-abuse-delivers.html), [IBM X-Force](https://exchange.xforce.ibmcloud.com/collection/Phantom-in-the-vault-Obsidian-abused-to-deliver-PhantomPulse-RAT-a6b1c4b7-f41e-4c17-9b2f-7c73a1d9c7e0)). HN discussion: 366 points, 229 comments ([hn.algolia.com item 48088576](https://news.ycombinator.com/item?id=48088576)). No Obsidian exploit — pure abuse of the trust model: full machine compromise gated on one human click.
2. **Critical RCE in the Tasks plugin (3.4M downloads), disclosed May 8, 2026.** Opening a malicious Markdown note containing `filter by function require('child_process').execSync('calc')` executed arbitrary code; the plugin treated note *content* as *code* with full Node access. Fix: Tasks 8.0.0 disables JS execution in queries by default. ZeroQuarry's verdict: *"Obsidian plugins are not strongly sandboxed from the local environment, and plugin authors do not have a simple, universal way to sandbox arbitrary JavaScript safely"* ([zeroquarry.com/research/obsidian-tasks-rce](https://zeroquarry.com/research/obsidian-tasks-rce/), fix [PR #3860](https://github.com/obsidian-tasks-group/obsidian-tasks/pull/3860)).
3. **Excalidraw plugin vulnerability batch, May 2026.** 41 high / 32 medium findings in one plugin: `excalidraw-onload-script` frontmatter auto-executing on file open, vault-controlled SVG icons rendered as raw HTML, `cmd://` drawing links reaching the command system, attacker-influenced file deletion, private-URL leakage over HTTP, AI-generated HTML rendered unsandboxed ([zeroquarry.com/research/excalidraw-vulnerabilities](https://zeroquarry.com/research/excalidraw-vulnerabilities/)). Mitigations shipped.
4. **Context:** Obsidian's Cure53 (2023, 2024) and Trail of Bits (2025) audits cover the **app and Sync — not the plugin ecosystem** ([obsidian.md/security](https://obsidian.md/security)). A 2022 forum thread already noted "Plugins are reviewed initially, but not constantly" ([forum.obsidian.md](https://forum.obsidian.md/t/can-obsidian-plugins-have-malware/35661)).

## 4. Is the no-sandbox model working?

**For Obsidian, mostly yes — but only because of a threat model SpecForge does not share.**

Working: 120M installs with zero known directory-level supply-chain catastrophes; the worst campaign required social engineering; content-becomes-execution bugs were found by researchers and fixed in days; process controls (Restricted Mode, kill switch, no auto-update) are cheap and effective for a human-driven desktop app.

Strained: (a) manual review collapsed under scale — officially acknowledged; for ~6 years only v1 of each plugin was ever human-reviewed, so **update-path supply-chain drift was structurally open**; (b) once review automated, the unreviewed share hit 68% in four months; (c) Obsidian is now rebuilding sandbox-equivalents *in process*: per-capability disclosures, artifact attestation, verified authors, org allowlists, per-feature execution gates (Tasks 8.0.0), plus a central kill switch. That is the shape of a capability sandbox, implemented with humans and dashboards instead of hardware.

## 5. What happens if SpecForge takes the same approach

SpecForge today is the opposite bet: Wasm/Extism with deny-by-default host functions (`specforge.query_graph`, `emit_file`, `http_get`) — decision ADR `wasm_extism_plugin_runtime` (`spec/research/lua/RES-21-plugin-runtime-decision.md`, RES-21b) — and enforced capability policies in `crates/specforge-wasm/src/sandbox.rs` (path/domain/output-extension allowlists, memory ceilings, intersection merging) plus signing/trust in `crates/specforge-registry/src/signing.rs` and `src/client/trust.rs`.

Transferring Obsidian's model fails at three load-bearing points:

1. **No human click exists.** Obsidian's real defense is that a person must enable plugins on their machine. In SpecForge's consumption model — AI agents and CI running `specforge` over cloned repos — the "enable" moment is the first CI run. The PHANTOMPULSE lure ("open this shared vault, enable plugins") is not a phishing email for SpecForge; it is the default usage pattern.
2. **Content→execution is SpecForge's core loop.** Every Obsidian incident funneled shared *content* (vault, note, drawing) into plugin execution. SpecForge `.spec` files are exactly such shared content, executed downstream by extensions inside runners holding repo credentials and secrets. The Tasks RCE (`require('child_process')` from a note) is the canonical preview of a no-sandbox SpecForge extension escaping via an untrusted spec's data.
3. **Review cannot scale to it.** Obsidian, with revenue and millions of users, could not sustain manual review and conceded it. SpecForge has no review team; an Obsidian-style process layer would start at 100% unreviewed, and Obsidian's data shows what the unreviewed share becomes under AI-accelerated submission (68% in months).

What Obsidian **does** prove: a universal-runtime ecosystem can reach 120M installs when authoring is familiar and installation is frictionless — so SpecForge should steal the *distribution DX* (one-command install, scorecards, public scan results, fast review turnaround, disclosures), while keeping the *isolation mechanism*. Obsidian's roadmap (disclosures ≈ `SandboxPolicy`, attestation ≈ `signing.rs`, kill switch ≈ registry revocation) converges on what SpecForge already has in code; adopting Obsidian's process layer *on top of* Wasm is complementary, replacing Wasm with it is not.

---

## Bottom line

**KEEP_WASM** — confidence **4/5** — Obsidian is the strongest live experiment in no-sandbox plugins, and its own trajectory (manual review collapse → automated every-version scanning → capability disclosures → attestation → per-feature execution gates → 68% of its directory never human-reviewed within months of opening the gate) demonstrates that process-only trust converges on — slowly, and after real RAT and RCE incidents — exactly the capability-sandbox mechanism SpecForge already shipped.
