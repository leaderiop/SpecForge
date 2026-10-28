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
| `@gsd/core` | `mode`, `granularity`, `granularities.*`, `model_profile`, `model_profile_overrides.*`, `model_policy.*`, `models.*`, `dynamic_routing.*`, `runtime`, `phase_id_convention`, `project_code`, `phase_naming`, `response_language`, `context_window`, `context_profile`, `context`, `workflow.*` (but the ones below), `executor.*`, `planner.*`, `parallelization.*`, `gates.*`, `git.*`, `worktree.*`, `ship.*`, `review.*`, `agent_skills`, `agent_tools.*`, `hooks.*`, `statusline.*`, `refactor.*`, `model_overrides.*`, `resolve_model_ids`, `effort.*`, `fast_mode.*`, `manager.*`, `plan_review.*`, `audit.*`, `features.*`, `learnings.*`, `claude_orchestration.*`, `external_job.*`, `agent_skills_security.*`, `planning.chunked_parallel`, `parallelization` (the shorthand for `parallelization.enabled`), the search availability flags (`brave_search` … `jina`) |
| `@gsd/security` | `workflow.security_enforcement`, `workflow.security_asvs_level`, `workflow.security_block_on`, `security.*` |
| `@gsd/ui` | `workflow.ui_phase`, `workflow.ui_safety_gate`, `workflow.ui_review`, `workflow.live_dom_uat`, `workflow.ui_interaction_capture` |
| `@gsd/ai` | `workflow.ai_integration_phase`, `workflow.api_coverage_gate` |
| `@gsd/ops` | `workflow.code_review`, `workflow.code_review_*`, `code_quality.*`, `mempalace.*`, `profile-pipeline.*` |
| `@gsd/codebase` | `workflow.drift_threshold`, `workflow.drift_action`, `workflow.plan_drift_precheck`, `intel.*` |

Typed as GSD types them (K6 takes K3's field types), with three shapes GSD's JSON leaves loose:

| GSD key | Declared as |
|---|---|
| `brave_search`, `firecrawl`, `exa_search`, `tavily_search`, `ref_search`, `perplexity`, `jina` (`string \| boolean \| null`) | a bool setting that forces the source on or off (unset: detected from the key's presence), and a secret read from `BRAVE_API_KEY`, `FIRECRAWL_API_KEY`, `EXA_API_KEY`, `TAVILY_API_KEY`, `REF_API_KEY`, `PERPLEXITY_API_KEY`, `JINA_API_KEY`; `specforge gsd import` moves a string value out to the environment and reports it |
| `model_profile_overrides.<runtime>.<tier>` (`string \| object`) | the block `{model str R, reasoning_effort enum}`; the import rewrites a bare string to `{model}` |
| a key GSD leaves unset (`(none)`, `null`, _unset_) | no default; `null` is unset |

`workflow.security_asvs_level` is an integer 1..3, default 1.

## Not applicable

| GSD key | Why |
|---|---|
| `planning.commit_docs`, `commit_docs`, `phase_commit_docs.<phase-id>`, `planning.search_gitignored`, `planning.pr_strict`, `search_gitignored` | there is no `.planning/`; `spec/gsd/` is source and is committed like code (and `phase_commit_docs.<phase-id>` has a `.` in its segment, which a setting family key cannot hold) |
| `planning.sub_repos` | a sub-repository is another SpecForge project |
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

A setting has one preset source, and these two are independent of `profile`:

| Source | Presets |
|---|---|
| `model_profile` (four values) | `model_overrides.<agent>` from GSD's model-profile table (round r40, with the agents) |
| `workflow.context_drift_action` | `block` raises gsd/W406 to an error (raise only); the other values set nothing |

## SpecForge-only settings

Settings of `@gsd/core` GSD does not have, under `specforge.` so the roster check tells them apart. All are bool, default
false, and each gates rules ([rules.md](rules.md#setting-gated-rules)).

| Setting | Switches on |
|---|---|
| `specforge.task_commits` | every task has a commit (gsd/W407) |
| `specforge.traceability_gate` | a complete requirement traces to a task, a commit and a passing test (gsd/E515) |
| `specforge.stale_verdict_gate` | a stale verdict blocks every workflow step, not only execution (gsd/E516) |
| `specforge.cross_workstream_conflicts` | active plans of different workstreams share no file (gsd/E517) |

Which rule changes with the profile, and which a setting switches off, is in [rules.md](rules.md).
