//! The settings table of `@gsd/core`: every key of GSD's configuration the extension owns, with GSD's type and
//! default (model: `spec/settings.md`).
//!
//! The roster is the tracked gsd-core's: `config-schema.manifest.json` and the `config` of every capability, as
//! frozen in `gsd/parity/upstream/v1.18.0/`, plus the keys only `docs/CONFIGURATION.md` lists (the last rows).
//! A default is GSD's own (`config-defaults.manifest.json`, then the capability's, then the docs); a key GSD leaves
//! unset, or marks unset with an empty string or `-1` in a capability, has none (a `-1` lane setting is still accepted). Nothing here reads a setting: each
//! is inert until the round that gives its behaviour (README, "Build order").

use specforge_extension_sdk::SettingBuilder;

/// The type of a setting.
pub enum Ty {
    Bool,
    /// Inclusive bounds.
    Int(i64, Option<i64>),
    Enum(&'static [&'static str]),
    Str,
    List,
}

/// The value a setting takes when nothing sets it.
pub enum Dv {
    Unset,
    Bool(bool),
    Int(i64),
    Str(&'static str),
    List(&'static [&'static str]),
}

/// One static key.
pub struct Key {
    pub name: &'static str,
    pub ty: Ty,
    pub default: Dv,
    pub about: &'static str,
}

impl Key {
    pub fn apply(&self, s: &mut SettingBuilder) {
        s.describe(self.about);
        match &self.ty {
            Ty::Bool => {
                s.bool();
            }
            Ty::Int(min, max) => {
                s.integer().minimum(*min);
                if let Some(max) = max {
                    s.maximum(*max);
                }
            }
            Ty::Enum(values) => {
                s.enum_values(values);
            }
            Ty::Str => {
                s.string();
            }
            Ty::List => {
                s.string_list();
            }
        }
        match &self.default {
            Dv::Unset => {}
            Dv::Bool(v) => {
                s.default(*v);
            }
            Dv::Int(v) => {
                s.default(*v);
            }
            Dv::Str(v) => {
                s.default(*v);
            }
            Dv::List(v) => {
                s.default(v.to_vec());
            }
        }
    }
}

const fn b(name: &'static str, default: Option<bool>, about: &'static str) -> Key {
    let default = match default {
        Some(v) => Dv::Bool(v),
        None => Dv::Unset,
    };
    Key {
        name,
        ty: Ty::Bool,
        default,
        about,
    }
}

const fn i(
    name: &'static str,
    min: i64,
    max: Option<i64>,
    default: Option<i64>,
    about: &'static str,
) -> Key {
    let default = match default {
        Some(v) => Dv::Int(v),
        None => Dv::Unset,
    };
    Key {
        name,
        ty: Ty::Int(min, max),
        default,
        about,
    }
}

const fn e(
    name: &'static str,
    values: &'static [&'static str],
    default: Option<&'static str>,
    about: &'static str,
) -> Key {
    let default = match default {
        Some(v) => Dv::Str(v),
        None => Dv::Unset,
    };
    Key {
        name,
        ty: Ty::Enum(values),
        default,
        about,
    }
}

const fn s(name: &'static str, default: Option<&'static str>, about: &'static str) -> Key {
    let default = match default {
        Some(v) => Dv::Str(v),
        None => Dv::Unset,
    };
    Key {
        name,
        ty: Ty::Str,
        default,
        about,
    }
}

const fn l(
    name: &'static str,
    default: Option<&'static [&'static str]>,
    about: &'static str,
) -> Key {
    let default = match default {
        Some(v) => Dv::List(v),
        None => Dv::Unset,
    };
    Key {
        name,
        ty: Ty::List,
        default,
        about,
    }
}

/// The static keys: the manifest's and the capabilities' (alphabetical), then the keys only the docs list.
#[rustfmt::skip]
pub const KEYS: &[Key] = &[
    l("agent_skills_security.trusted_global_roots", Some(&[]), "Extra trusted directories for global: skills."),
    b("audit.enabled", Some(false), "Opt in to the dispatch audit trail."),
    b("brave_search", None, "Override auto-detection of Brave Search API availability."),
    b("claude_orchestration.enabled", Some(false), "Master toggle for the Claude orchestration capability."),
    e("claude_orchestration.execution_backend", &["auto", "workflow", "inline"], Some("auto"), "Which dispatch backend execute-phase uses when the capability is enabled."),
    s("claude_orchestration.min_agent_sdk_version", Some("0.3.149"), "Oldest Agent SDK version the Workflow backend may run on."),
    e("context", &["dev", "research", "review"], None, "Execution context preset injected into every agent prompt: dev, research or review (GSD refuses other values)."),
    i("context_window", 1, None, Some(200000), "Context window size in tokens."),
    e("effort.default", &["minimal", "low", "medium", "high", "xhigh", "max", "inherit"], Some("high"), "Global fallback effort level."),
    b("exa_search", None, "Override auto-detection of Exa Search API availability."),
    i("executor.stall_detect_interval_minutes", 1, None, Some(5), "Minutes between executor stall checks."),
    i("executor.stall_threshold_minutes", 1, None, Some(10), "Minutes without executor progress before execute-phase offers recovery."),
    s("external_job.artifact_dir", Some("Artifacts/jobs"), "Root directory for per-job artifacts."),
    e("external_job.backend", &["slurm"], Some("slurm"), "Scheduler backend."),
    b("external_job.enabled", Some(false), "Master toggle for the async external-job producer capability."),
    i("external_job.poll_timeout_ms", 1, None, Some(15000), "Timeout in milliseconds for the scheduler poll."),
    i("external_job.submit_timeout_ms", 1, None, Some(30000), "Timeout in milliseconds for the scheduler submit."),
    b("fast_mode.enabled", Some(false), "Global fast_mode flag."),
    b("features.global_learnings", Some(false), "Copy learnings across projects and inject them into planning."),
    b("features.thinking_partner", Some(false), "Analyse workflow decision points with a thinking partner."),
    b("firecrawl", None, "Override auto-detection of Firecrawl API availability."),
    b("gates.confirm_milestone_scope", Some(true), "Confirm milestone scope before shipping (complete-milestone.md)."),
    b("gates.confirm_transition", Some(true), "Confirm phase transition (transition.md)."),
    b("gates.execute_next_plan", Some(true), "Confirm before executing next plan (execute-plan.md)."),
    b("git.allow_default_branch_commits", Some(false), "Let the executor commit on the default branch."),
    s("git.base_branch", None, "The branch phase and milestone branches start from and merge back into."),
    e("git.branching_strategy", &["none", "phase", "milestone"], Some("none"), "Branch per phase, per milestone, or neither."),
    b("git.create_tag", Some(true), "Tag the release when a milestone completes."),
    s("git.milestone_branch_template", Some("gsd/{milestone}-{slug}"), "Branch name template for milestone strategy."),
    s("git.phase_branch_template", Some("gsd/phase-{phase}-{slug}"), "Branch name template for phase strategy."),
    l("git.protected_branches", None, "Extra shared branches that trigger protected-branch warnings."),
    s("git.quick_branch_template", None, "Optional branch name template for /gsd-quick tasks."),
    e("granularity", &["coarse", "standard", "fine"], Some("standard"), "How many phases a roadmap has: coarse 2-4, standard 4-6, fine 6-10."),
    l("hooks.commit_types", Some(&[]), "Extra Conventional Commits types the commit-message hook accepts."),
    b("hooks.community", Some(false), "Turn on the opt-in community hooks."),
    i("hooks.context_critical_threshold", 0, Some(99), Some(25), "Percent of context left at or below which the monitor reports critical (GSD refuses 100: no warning could be above it)."),
    i("hooks.context_warning_threshold", 1, Some(100), Some(35), "Percent of context left at or below which the monitor warns (GSD refuses 0: no critical could be below it)."),
    b("hooks.context_warnings", Some(true), "Warn about context window usage."),
    b("hooks.workflow_guard", Some(false), "Warn when files are edited outside a GSD workflow."),
    b("jina", None, "Override auto-detection of Jina API availability."),
    i("learnings.max_inject", 0, None, Some(10), "Maximum number of cross-project learnings injected into each planner prompt."),
    s("manager.flags.discuss", None, "Flags added to the manager's discuss-phase commands."),
    s("manager.flags.execute", None, "Flags added to the manager's execute-phase commands."),
    s("manager.flags.plan", None, "Flags added to the manager's plan-phase commands."),
    e("mode", &["interactive", "yolo"], Some("interactive"), "yolo approves decisions on its own; interactive confirms each step."),
    e("model_policy.budget", &["high", "medium", "low"], None, "Budget tier of a known provider."),
    s("model_policy.high", None, "Model ID of the high tier for a generic provider."),
    s("model_policy.low", None, "Model ID of the low tier for a generic provider."),
    s("model_policy.medium", None, "Model ID of the medium tier for a generic provider."),
    s("model_policy.provider", None, "Model provider: openai, anthropic, anthropic-fable, google, qwen or generic."),
    e("model_profile", &["quality", "balanced", "budget", "adaptive", "inherit"], Some("balanced"), "Model tier of each agent: quality, balanced, budget, adaptive or inherit."),
    b("parallelization", Some(true), "Shorthand for parallelization.enabled."),
    b("perplexity", None, "Override auto-detection of Perplexity API availability."),
    e("phase_id_convention", &["sequential", "milestone-prefixed", "bracket"], None, "How phase IDs are formed; unset and sequential use plain numbers."),
    s("phase_naming", Some("sequential"), "How phase directories are named."),
    b("plan_review.source_grounding", Some(true), "Enable the plan drift guard."),
    e("plan_review.source_grounding_authority", &["grep", "intel", "treesitter", "lsp", "scip"], Some("grep"), "Selects the resolver adapter used to verify symbol existence."),
    i("planner.stall_detect_interval_minutes", 1, None, Some(5), "Minutes between planner stall checks."),
    b("planner.stall_detection_enabled", Some(true), "Detect stalled planner and plan-checker agents."),
    i("planner.stall_threshold_minutes", 1, None, Some(10), "Minutes without planner progress before plan-phase offers recovery."),
    b("planning.chunked_parallel", Some(false), "Opt-in concurrent per-plan planners in chunked mode."),
    s("project_code", None, "Prefix for phase directory names."),
    b("ref_search", None, "Override auto-detection of Ref search API availability."),
    i("refactor.complexity_jump_delta", 1, None, Some(5), "Complexity growth that triggers a refactor proposal below the threshold."),
    i("refactor.complexity_threshold", 1, None, Some(15), "Absolute per-function complexity above which a refactor proposal is surfaced."),
    b("refactor.trigger_enabled", Some(false), "Enable the complexity-triggered refactor hook."),
    b("refactor.trigger_strict", Some(false), "Record an untriaged refactor proposal as an open broken-windows entry."),
    e("resolve_model_ids", &["aliases", "ids", "omit"], Some("aliases"), "How agent models are named: tier aliases, full model IDs, or left out for a non-Claude runtime."),
    s("response_language", None, "Language of agent responses."),
    l("review.default_reviewers", None, "Reviewers /gsd-review uses when none are named."),
    s("review.effort.claude", None, "Reasoning effort for the Claude reviewer lane."),
    s("review.effort.codex", None, "Reasoning effort for the Codex reviewer lane."),
    s("review.effort.opencode", None, "Reasoning effort for the OpenCode reviewer lane."),
    s("review.llama_cpp_host", Some("http://localhost:8080"), "Base URL of the llama.cpp OpenAI-compatible server."),
    s("review.lm_studio_host", Some("http://localhost:1234"), "Base URL of the LM Studio OpenAI-compatible server."),
    i("review.max_prompt_tokens", 1, None, None, "Default maximum estimated tokens for the assembled review prompt."),
    i("review.max_prompt_tokens_per_reviewer.antigravity", -1, None, None, "Prompt-token budget for the Antigravity reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.claude", -1, None, None, "Prompt-token budget for the Claude reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.coderabbit", -1, None, None, "Prompt-token budget for the CodeRabbit reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.codex", -1, None, None, "Prompt-token budget for the Codex reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.cursor", -1, None, None, "Prompt-token budget for the Cursor reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.kimi-code", -1, None, None, "Prompt-token budget for the Kimi Code reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.llama_cpp", -1, None, None, "Prompt-token budget for the llama.cpp reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.lm_studio", -1, None, None, "Prompt-token budget for the LM Studio reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.ollama", -1, None, None, "Prompt-token budget for the Ollama reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.opencode", -1, None, None, "Prompt-token budget for the OpenCode reviewer lane."),
    i("review.max_prompt_tokens_per_reviewer.qwen", -1, None, None, "Prompt-token budget for the Qwen Code reviewer lane."),
    s("review.models.agy", None, "Model passed to the Antigravity reviewer lane."),
    s("review.models.claude", None, "Model passed to the Claude reviewer lane."),
    s("review.models.codex", None, "Model passed to the Codex reviewer lane."),
    s("review.models.cursor", None, "Model passed to the Cursor reviewer lane."),
    s("review.models.kimi-code", None, "Model passed to the Kimi Code reviewer lane."),
    s("review.models.llama_cpp", None, "Model requested from the llama.cpp reviewer lane."),
    s("review.models.lm_studio", None, "Model requested from the LM Studio reviewer lane."),
    s("review.models.ollama", None, "Model requested from the Ollama reviewer lane."),
    s("review.models.opencode", None, "Model passed to the OpenCode reviewer lane."),
    s("review.ollama_host", Some("http://localhost:11434"), "Base URL of the Ollama OpenAI-compatible server."),
    b("review.parallel_lanes", Some(false), "Run independent reviewer lanes at the same time."),
    i("review.timeouts.antigravity", -1, None, None, "Outer wall-clock timeout override (seconds) for the Antigravity reviewer lane."),
    i("review.timeouts.claude", -1, None, None, "Outer wall-clock timeout override (seconds) for the Claude reviewer lane."),
    i("review.timeouts.codex", -1, None, None, "Outer wall-clock timeout override (seconds) for the Codex reviewer lane."),
    i("review.timeouts.kimi-code", -1, None, None, "Outer wall-clock timeout override (seconds) for the Kimi Code reviewer lane."),
    i("review.timeouts.llama_cpp", -1, None, None, "Outer wall-clock timeout override (seconds) for the llama.cpp reviewer lane."),
    i("review.timeouts.lm_studio", -1, None, None, "Outer wall-clock timeout override (seconds) for the LM Studio reviewer lane."),
    i("review.timeouts.ollama", -1, None, None, "Outer wall-clock timeout override (seconds) for the Ollama reviewer lane."),
    i("review.timeouts.opencode", -1, None, None, "Outer wall-clock timeout override (seconds) for the OpenCode reviewer lane."),
    s("runtime", None, "Active runtime for runtime-aware profile resolution."),
    l("ship.pr_body_sections", Some(&[]), "Extra sections the ship step adds to the pull request body."),
    e("statusline.context_position", &["end", "front"], Some("end"), "Where the context meter sits: end of the line or front."),
    b("statusline.show_context_tokens", Some(false), "Show the absolute token count after the context meter."),
    b("statusline.show_git", Some(false), "Show the git branch and work-state markers."),
    b("statusline.show_last_command", Some(false), "Show the last slash command used."),
    b("statusline.show_state_freshness", Some(false), "Show how many commits back the GSD state is."),
    e("statusline.state_format", &["full", "compact"], Some("full"), "GSD-state segment format: full or compact."),
    b("tavily_search", None, "Override auto-detection of Tavily Search API availability."),
    b("workflow.agent_hint_routing", Some(true), "Route each plan to a specialist executor."),
    b("workflow.assumption_delta", Some(true), "Enable the assumption-delta architecture checkpoint during planning."),
    b("workflow.auto_advance", Some(false), "Auto-chain discuss → plan → execute without stopping."),
    s("workflow.build_command", None, "Shell command for the post-merge build gate."),
    b("workflow.compact_content", Some(false), "Compact content mode."),
    b("workflow.context_coverage_gate", Some(true), "Toggle for both decision-coverage gates."),
    e("workflow.context_drift_action", &["warn", "block"], Some("warn"), "What the context-drift gate does on a stale artifact: warn or block."),
    b("workflow.context_drift_precheck", Some(true), "Enable the non-blocking context-drift pre-check at plan:pre."),
    e("workflow.context_guard_mode", &["auto", "warn", "off"], Some("warn"), "Context exhaustion guard before each execute wave: auto, warn or off."),
    s("workflow.cross_ai_command", None, "Shell command template for cross-AI execution."),
    b("workflow.cross_ai_execution", Some(false), "Delegate phase execution to an external AI CLI instead of spawning local executor agents."),
    i("workflow.cross_ai_timeout", 1, None, Some(300), "Timeout in seconds for cross-AI execution commands."),
    e("workflow.discuss_mode", &["discuss", "assumptions"], Some("discuss"), "How discuss-phase gathers context: discuss asks, assumptions reads the code first."),
    e("workflow.human_verify_mode", &["mid-flight", "end-of-phase"], Some("end-of-phase"), "When human verification happens: end-of-phase or mid-flight."),
    i("workflow.inline_plan_threshold", 0, None, Some(2), "Most tasks a phase may have before the planner writes a separate PLAN.md."),
    i("workflow.max_discuss_passes", 1, None, Some(3), "Maximum number of question rounds in discuss-phase before the workflow stops asking."),
    b("workflow.mvp_mode", Some(false), "Plan every phase as an MVP slice by default."),
    b("workflow.node_repair", Some(true), "Autonomous task repair on verification failure."),
    i("workflow.node_repair_budget", 0, None, Some(2), "Max repair attempts per failed task."),
    b("workflow.nyquist_validation", Some(true), "Enable Nyquist validation coverage auditing."),
    b("workflow.pattern_mapper", Some(true), "Run the pattern mapper before planning when context or research is available."),
    b("workflow.plan_bounce", Some(false), "Run external validation script against generated plans."),
    i("workflow.plan_bounce_passes", 1, None, Some(2), "Number of sequential bounce passes to run."),
    s("workflow.plan_bounce_script", None, "Path to the external script invoked for plan bounce validation."),
    b("workflow.plan_check", Some(true), "Run the plan verification loop."),
    b("workflow.plan_chunked", Some(false), "Enable chunked planning mode."),
    b("workflow.plan_review_convergence", Some(false), "Enable the /gsd-plan-review-convergence command."),
    b("workflow.post_planning_gaps", Some(true), "Run the post-planning gap analysis report after plans are generated."),
    b("workflow.research", Some(true), "Run phase research before planning when research artifacts are missing."),
    b("workflow.research_before_questions", Some(false), "Run research before discussion questions instead of after."),
    b("workflow.schema_drift_gate", Some(true), "Enable the drift gates at execute:wave:post."),
    b("workflow.schema_push_detection", Some(true), "Enable ORM schema push detection during planning."),
    b("workflow.skip_discuss", Some(false), "Autonomous runs skip discuss-phase."),
    i("workflow.smart_zone_tokens", 1, None, Some(100000), "Token budget above which a phase is flagged for splitting; advisory."),
    b("workflow.specless_probe_fallback", Some(true), "Author edge and prohibition predicates for a phase that has no spec."),
    i("workflow.subagent_timeout", 1, None, Some(300000), "Timeout in milliseconds for parallel subagent tasks."),
    b("workflow.tdd_mode", Some(false), "Plan eligible tasks as TDD and enforce the red, green, refactor sequence."),
    s("workflow.test_command", None, "Shell command for the post-merge test gate."),
    i("workflow.test_gate_timeout", 1, None, Some(600), "Timeout in seconds for a verification test gate."),
    b("workflow.text_mode", Some(false), "Replaces AskUserQuestion TUI menus with plain-text numbered lists."),
    b("workflow.use_worktrees", Some(true), "Isolate parallel execution in git worktrees."),
    b("workflow.verifier", Some(true), "Post-execution verification against phase goals."),
    b("workflow.windows_enforce", Some(false), "Block ship while the broken-windows ledger has open entries."),
    b("workflow.worktree_skip_hooks", Some(false), "Executors in worktrees skip pre-commit hooks."),
    b("dynamic_routing.enabled", Some(false), "Master switch for dynamic routing with failure-tier escalation."),
    b("dynamic_routing.escalate_on_failure", Some(true), "Escalate one tier up after a soft failure."),
    i("dynamic_routing.max_escalations", 0, None, Some(1), "Hard cap on retries per agent invocation."),
    l("dynamic_routing.provider_escalation", None, "Fallback model IDs tried when a run hits a quota limit."),
    e("effort.routing_tier_defaults.heavy", &["minimal", "low", "medium", "high", "xhigh", "max", "inherit"], Some("xhigh"), "Effort for heavy-tier agents (deep reasoning)."),
    e("effort.routing_tier_defaults.light", &["minimal", "low", "medium", "high", "xhigh", "max", "inherit"], Some("low"), "Effort for light-tier agents (fast mappers/scanners)."),
    e("effort.routing_tier_defaults.standard", &["minimal", "low", "medium", "high", "xhigh", "max", "inherit"], Some("high"), "Effort for standard-tier agents (workhorse agents)."),
    b("fast_mode.routing_tier_defaults.heavy", Some(false), "Fast mode for heavy-tier agents."),
    b("fast_mode.routing_tier_defaults.light", Some(true), "Fast mode for light-tier agents."),
    b("fast_mode.routing_tier_defaults.standard", Some(false), "Fast mode for standard-tier agents."),
    b("parallelization.enabled", Some(true), "Run independent plans simultaneously."),
    i("parallelization.max_concurrent_agents", 1, None, Some(3), "Maximum simultaneous agents."),
    i("parallelization.min_plans_for_parallel", 1, None, Some(2), "Minimum plans to trigger parallel execution."),
    b("parallelization.plan_level", Some(true), "Parallelize at plan level."),
    b("parallelization.skip_checkpoints", Some(true), "Skip checkpoints during parallel execution."),
    b("parallelization.task_level", Some(false), "Parallelize tasks within a plan."),
    s("worktree.baseRef", None, "Ref new parallel worktrees start from; head uses the local HEAD."),
];

pub const EFFORTS: &[&str] = &[
    "minimal", "low", "medium", "high", "xhigh", "max", "inherit",
];
const PHASE_TYPES: &[&str] = &[
    "planning",
    "discuss",
    "research",
    "execution",
    "verification",
    "completion",
];
const TIERS: &[&str] = &["light", "standard", "heavy"];
const PROFILE_TIERS: &[&str] = &["opus", "sonnet", "haiku"];

/// A family: a name with `*` segments, each standing for one key. A family has no default.
pub struct Family {
    pub name: &'static str,
    pub ty: Ty,
    /// The keys each `*` may take, in order; an empty list takes any segment.
    pub keys: &'static [&'static [&'static str]],
    pub about: &'static str,
    /// A `{model, reasoning_effort}` block rather than a plain value.
    pub model_block: bool,
}

const fn family(
    name: &'static str,
    ty: Ty,
    keys: &'static [&'static [&'static str]],
    about: &'static str,
) -> Family {
    Family {
        name,
        ty,
        keys,
        about,
        model_block: false,
    }
}

const fn model_block(name: &'static str, about: &'static str) -> Family {
    Family {
        name,
        ty: Ty::Str,
        keys: &[&[], PROFILE_TIERS],
        about,
        model_block: true,
    }
}

/// The families: GSD's dynamic key patterns. `agent_tools.*` and `effort.routing_tier_defaults.*` need care: a
/// family cannot hold the literal `*` key, and a family has no default, so the tier defaults are static keys above.
#[rustfmt::skip]
pub const FAMILIES: &[Family] = &[
    family("agent_skills.*", Ty::List, &[&[]], "Skills added to one agent type's prompt."),
    family("agent_tools.*", Ty::List, &[&[]], "Tools granted to one agent at install time; `all` grants every agent."),
    family("dynamic_routing.tier_models.*", Ty::Enum(PROFILE_TIERS), &[TIERS], "Tier alias used for a routing tier."),
    family("effort.agent_overrides.*", Ty::Enum(EFFORTS), &[&[]], "Effort for one agent; beats the tier defaults."),
    family("fast_mode.agent_overrides.*", Ty::Bool, &[&[]], "Fast mode for one agent; beats the tier defaults."),
    family("features.*", Ty::Bool, &[&[]], "A feature toggle."),
    family("granularities.*", Ty::Enum(&["coarse", "standard", "fine"]), &[PHASE_TYPES], "Granularity of one phase type."),
    family("model_overrides.*", Ty::Str, &[&[]], "Model ID for one agent."),
    family("models.*", Ty::Enum(&["opus", "sonnet", "haiku", "inherit"]), &[PHASE_TYPES], "Model tier of one phase type."),
    family("review.reviewer_instances.*.*", Ty::Str, &[&[], &["cli", "model", "agent"]], "A named reviewer instance: its adapter, model or agent."),
    model_block("model_policy.runtime_tiers.*.*", "Model entry for one runtime and tier; beats model_profile_overrides."),
    model_block("model_profile_overrides.*.*", "Model entry for one runtime and tier."),
];

impl Family {
    pub fn apply(&self, s: &mut SettingBuilder) {
        s.describe(self.about);
        if self.model_block {
            s.block()
                .member("model", |f| {
                    f.field_type(specforge_extension_sdk::prelude::FieldType::String)
                        .required();
                })
                .member("reasoning_effort", |f| {
                    f.vocabulary(&specforge_extension_sdk::prelude::Vocabulary::new(EFFORTS));
                });
        } else {
            match &self.ty {
                Ty::Bool => s.bool(),
                Ty::Int(..) => s.integer(),
                Ty::Enum(values) => s.enum_values(values),
                Ty::Str => s.string(),
                Ty::List => s.string_list(),
            };
        }
        s.keys(self.keys);
    }
}
