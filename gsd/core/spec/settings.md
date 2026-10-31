# Settings and profiles

GSD's `.planning/config.json` becomes typed extension settings in `specforge.json` (K6). Each setting keeps
GSD's dotted name, type and default, and is declared by the extension that owns the behaviour, so a project that does
not load `@gsd/ui` has no `workflow.ui_phase`.

```json
{
  "extensions": ["@gsd/core", "@gsd/security"],
  "settings": {
    "@gsd/core": {
      "profile": "gsd-compat",
      "mode": "interactive",
      "granularity": "standard",
      "workflow.tdd_mode": true,
      "parallelization.max_concurrent_agents": 3
    },
    "@gsd/security": { "workflow.security_asvs_level": 2 }
  }
}
```

## The roster is GSD's

The source of truth for names, types and defaults is the configuration of the tracked gsd-core at the tracked tag:
`docs/CONFIGURATION.md` and the `config` blocks of `capabilities/*/capability.json` (some keys, such as
`workflow.windows_enforce` and `workflow.schema_drift_gate`, are only there). The parity suite's roster check asserts that every key there is declared by
one `@gsd/*` extension or listed below as not applicable, with its reason.

| Owner | Key families |
|---|---|
| `@gsd/core` | `mode`, `granularity`, `granularities.*`, `model_profile`, `model_profile_overrides.*`, `model_policy.*`, `models.*`, `dynamic_routing.*`, `runtime`, `phase_id_convention`, `project_code`, `phase_naming`, `response_language`, `context_window`, `context`, `workflow.*` (but the ones below), `executor.*`, `planner.*`, `parallelization.*`, `gates.*`, `git.*`, `worktree.*`, `ship.*`, `review.*`, `agent_skills`, `agent_tools.*`, `hooks.*`, `statusline.*`, `refactor.*`, `model_overrides.*`, `resolve_model_ids`, `effort.*`, `fast_mode.*`, `manager.*`, `plan_review.*`, `audit.*`, `features.*`, `learnings.*`, `claude_orchestration.*`, `external_job.*`, `agent_skills_security.*`, `planning.chunked_parallel`, `parallelization` (the shorthand for `parallelization.enabled`), the search availability flags (`brave_search` … `jina`) |
| `@gsd/security` | `workflow.security_enforcement`, `workflow.security_asvs_level`, `workflow.security_block_on`, `security.*` |
| `@gsd/ui` | `workflow.ui_phase`, `workflow.ui_safety_gate`, `workflow.ui_review`, `workflow.live_dom_uat`, `workflow.ui_interaction_capture` |
| `@gsd/ai` | `workflow.ai_integration_phase`, `workflow.api_coverage_gate` |
| `@gsd/ops` | `workflow.code_review`, `workflow.code_review_*`, `code_quality.*`, `mempalace.*`, `profile-pipeline.*` |
| `@gsd/codebase` | `workflow.drift_threshold`, `workflow.drift_action`, `workflow.plan_drift_precheck`, `intel.*` |

Typed as GSD types them (K6 takes K3's field types), with three shapes GSD's JSON leaves loose:

| GSD key | Declared as |
|---|---|
| `brave_search`, `firecrawl`, `exa_search`, `tavily_search`, `ref_search`, `perplexity`, `jina` (`string \| boolean \| null`) | a bool setting that forces the source on or off, with no default under any of the seven (unset: detected from the key's presence, as GSD does at `config-new-project`; GSD also detects from `~/.gsd/<name>_api_key` files, which the environment-only secrets do not read), and a secret read from `BRAVE_API_KEY`, `FIRECRAWL_API_KEY`, `EXA_API_KEY`, `TAVILY_API_KEY`, `REF_API_KEY`, `PERPLEXITY_API_KEY`, `JINA_API_KEY`, named `<flag>_key` (`brave_search_key`, `firecrawl_key`, `exa_search_key`, `tavily_search_key`, `ref_search_key`, `perplexity_key`, `jina_key`) because the kernel refuses a secret that shares a setting's name; `specforge gsd import` moves a string value out to the environment and reports it |
| `model_profile_overrides.<runtime>.<tier>` (`string \| object`) | the block `{model str R, reasoning_effort enum}`; the import rewrites a bare string to `{model}` |
| a key GSD leaves unset (`(none)`, `null`, _unset_, or a capability's empty string or `-1`) | no default; `null` is unset |
| `resolve_model_ids` (`false \| true \| "omit"`) | an enum `aliases \| ids \| omit`, default `aliases`; the import maps `false`, `true` and `"omit"` |
| `agent_tools.*` with the key `*` (every agent) | the family `agent_tools.*`, whose key is a segment (`[A-Za-z0-9_-]+`): the all-agents grant is `agent_tools.all`, and the import maps `*` to `all` |
| `effort.routing_tier_defaults.<tier>`, `fast_mode.routing_tier_defaults.<tier>` | three static keys each, not a family: a family has no default, and each tier has GSD's |
| a key with a closed set of values GSD validates (`workflow.human_verify_mode`, `statusline.state_format`, …) | an enum of that set; `context` is the enum `dev`, `research`, `review` (GSD's `config-set context` refuses any other value), default unset. `context_profile`, which `docs/CONFIGURATION.md` lists, is read nowhere in GSD's source and is not declared |

An integer is declared with the bounds GSD enforces (`context_window` and `workflow.smart_zone_tokens` at least 1) or, where GSD
states none, the least that makes sense (0 for a count, 1 for a timeout). `hooks.context_warning_threshold` is 1..100 and
`hooks.context_critical_threshold` 0..99 (percent of the window left; GSD refuses a warning of 0 and a critical of 100, as no
partner could satisfy `critical < warning`; the pair itself is not checked, as GSD does not at write time). The reviewer-lane
`review.timeouts.<lane>` and `review.max_prompt_tokens_per_reviewer.<lane>` take `-1`, the capability default GSD reads as unset
(the model leaves them unset; GSD treats any non-positive value as unset too, and only `-1` is accepted here).

A `number` setting (ADR 0305) takes a whole or a decimal number within whole-number bounds: `hooks.context_warning_threshold` (1-100) and `hooks.context_critical_threshold` (0-99) are numbers. The other settings GSD's capabilities declare as `number` (`7.5`, `1.5` seconds) are whole numbers here; the bounds of a number are whole, so `99.5` is within the critical bound. `worktree.baseRef` is declared here, but GSD keeps it in `.claude/settings.local.json`, not `.planning/config.json`
(`docs/CONFIGURATION.md`), so `specforge gsd import` reads it from there.
Where `docs/CONFIGURATION.md` and the defaults manifest disagree, the manifest wins (`workflow.inline_plan_threshold` is 2, the
docs say 3). A host URL whose capability default is empty and whose documented default is a URL (`review.ollama_host`) takes the
documented one.

`workflow.security_asvs_level` is an integer 1..3, default 1.

## Not applicable

| GSD key | Why |
|---|---|
| `planning.commit_docs`, `commit_docs`, `phase_commit_docs.<phase-id>`, `planning.search_gitignored`, `planning.pr_strict`, `search_gitignored` | there is no `.planning/`; `spec/gsd/` is source and is committed like code (and `phase_commit_docs.<phase-id>` has a `.` in its segment, which a setting family key cannot hold) |
| `planning.sub_repos`, `git.repo_tagging` | a sub-repository is another SpecForge project |
| `claude_md_path`, `claude_md_assembly.*` | host files are rendered by `specforge install`, which owns their paths |
| `workflow.auto_prune_state` | `STATE.md` is derived, nothing to prune |
| the API key half of `brave_search` … `jina` | a secret: read from the environment, never from `specforge.json` (K6 E087) |
| `capabilities.*` | third-party capability installs: an extension is enabled by `specforge.json` `extensions`, and installing one is SpecForge's |
| `graphify.*` | graphify is absorbed: SpecForge is the graph, compiled on every change, with no build, timeout or graph file to point at |

## Profiles

`profile` is a setting of `@gsd/core` with three values. A profile presets other settings and raises rule
severities, never lowers them; a rule that is off under a profile is a gate setting the profile presets to false. An
explicit setting wins over its profile. The other `@gsd/*` extensions declare `@gsd/core` as a peer and preset their own
settings from `@gsd/core:profile`. A setting has one preset source, so nothing `profile` presets is preset by another
setting.

| Profile | Behaves as |
|---|---|
| `gsd-compat` (default) | GSD at the tracked version: every rule GSD only warns about warns, every check GSD leaves to a prompt is still judged but does not block. The parity suite runs on it. |
| `gsd-strict` | the same rules, blocking where GSD warns: same-wave file conflicts, a stale plan-check verdict, a backstop truth without evidence, and an open broken window at ship. |
| `gsd-plus` | strict, and what GSD cannot do: requirement → task → commit → test traceability as a gate, stale-verdict blocking at every step, cross-workstream conflict detection. |

A profile presets these settings:

| Setting | gsd-compat | gsd-strict | gsd-plus |
|---|---|---|---|
| `workflow.windows_enforce` | false | true | true |
| `specforge.task_commits` | false | true | true |
| `specforge.traceability_gate` | false | false | true |
| `specforge.stale_verdict_gate` | false | false | true |
| `specforge.cross_workstream_conflicts` | false | false | true |

## Other presets

A setting has one preset source, and these are independent of `profile`:

| Source | Presets |
|---|---|
| `model_profile` (five values: `quality`, `balanced`, `budget`, `adaptive`, `inherit`) | `model_overrides.<agent>` of the 15 core agents, from GSD's model catalog (`src/settings/models.rs`, pinned against the vendored `model-catalog.json`): `quality`, `balanced` and `budget` give the agent's alias for that profile, `adaptive` its routing tier, as a host tier (opus is `heavy`, sonnet `standard`, haiku `light`); `inherit` sets nothing, so an agent file has no model line. An explicit `model_overrides.<agent>` wins, as in GSD. GSD's second step, `models.<phase type>`, is not wired: a preset cannot read another setting |
| `workflow.compact_content` | `true` sets `specforge.agent_variant` to `compact`; `false` sets nothing |
| `workflow.context_drift_action` | `block` raises gsd/W406 to an error (raise only); the other values set nothing |

## SpecForge-only settings

Settings of `@gsd/core` GSD does not have, under `specforge.` so the roster check tells them apart. Four are bool, default
false, and each gates rules ([rules.md](rules.md#setting-gated-rules)). The fifth, `specforge.agent_variant`, gates none:
it is an enum, `full` (the default) or `compact`, that picks the body of the ten agents that have a compact variant
(codex and cursor; Claude Code always renders the full body, as upstream installs no compact agent for it). It is an enum
because the host takes a variant setting only as a string or enum; `workflow.compact_content` presets it.

| Setting | Switches on |
|---|---|
| `specforge.task_commits` | every task of a complete plan has a commit (gsd/W407) |
| `specforge.traceability_gate` | a complete requirement traces to a task, a commit and a passing test (gsd/E515) |
| `specforge.stale_verdict_gate` | a stale verdict blocks every workflow step, not only execution (gsd/E516) |
| `specforge.cross_workstream_conflicts` | active plans of different workstreams share no file (gsd/E517) |

Ten more are bool, default **true**, and show or hide skills: `specforge.surface.<cluster>` for each of GSD's skill
clusters (`core_loop`, `audit_review`, `milestone`, `research_ideate`, `workspace_state`, `docs`, `ui`, `ai_eval`,
`ns_meta`, `utility`; the table is ported from `src/clusters.cts`, `src/settings/surface.rs`). A slash command is shown
while the setting of every cluster that lists it is on (`when` takes several settings): upstream hides a skill in *any*
disabled cluster, so disabling `utility` hides `/gsd-help` (also `core_loop`). `/gsd-surface` prints the
`specforge settings set` line and the reinstall; it changes nothing itself.

Which rule changes with the profile, and which a setting switches off, is in [rules.md](rules.md).
