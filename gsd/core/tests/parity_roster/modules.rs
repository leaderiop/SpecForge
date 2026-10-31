//! The 288 `get-shit-done/bin/lib` modules of the inventory manifest and what became of each (D-r44-17): the file of
//! `@gsd/core` that ports it, the `specforge` command that is its host half, the extension that owns it, or why it does
//! not apply. A name ending in `*` is a prefix; every module matches exactly one row.

/// What became of one upstream module.
#[derive(Debug, Clone, Copy)]
pub enum Module {
    /// Ported: the file of `gsd/core` (relative to it) that carries it.
    Ported(&'static str),
    /// The host half of the work: the `specforge` command.
    Host(&'static str),
    /// Another `@gsd/*` extension owns it.
    Owned(&'static str),
    /// Not part of `@gsd/core`, with the reason.
    NotApplicable(&'static str),
}

use Module::{Host, NotApplicable, Owned, Ported};

const ROUTER: &str =
    "gsd-tools' argv router; typed commands (`specforge gsd <command>`) replaced it";
const CAPABILITY: &str = "gsd-core's capability loader and ledger; a SpecForge extension is the unit (extension-sdk, the lockfile, `specforge doctor`)";
const GATE_ENGINE: &str =
    "the gate engine (`check.predicate`, `check.auto-mode`); the rules are kernel rules, D-r44-22";
const STATE_MD: &str = "STATE.md is not a file here: its values are computed from the spec (src/commands/state.rs answers the commands)";
const DOCUMENT: &str =
    "reads .planning markdown; the kinds are typed .spec files the kernel parses";
const PLANNING_DIR: &str =
    "the .planning directory's reader and workspace; the spec root is the project's files";
const SUB_REPOS: &str = "per-repository releases over planning.sub_repos; a sub-repository is another SpecForge project (settings.md)";
const RUNTIME: &str = "an upstream runtime this port does not install for (opencode, copilot, windsurf and the others are later renderers, README)";

/// `(name or prefix*, what became of it)`.
pub const MODULES: &[(&str, Module)] = &[
    (
        "active-workstream-store.cjs",
        Ported("src/commands/workstreams.rs"),
    ),
    ("adapter-declarative.cjs", Host("specforge install")),
    ("adapter-imperative.cjs", Host("specforge install")),
    (
        "adr-parser.cjs",
        NotApplicable("parses ADRs for /gsd-ingest-docs; that prompt has the agent read them"),
    ),
    ("agent-command-router.cjs", NotApplicable(ROUTER)),
    ("agent-install-check.cjs", Host("specforge install")),
    ("api-coverage.cjs", Owned("@gsd/ai")),
    ("args-terminator.cjs", Ported("../kit/src/prompt/render.rs")),
    ("artifacts.cjs", Ported("src/kinds/mod.rs")),
    ("assumption-delta.cjs", Ported("src/settings/gates.rs")),
    ("audit-command-router.cjs", NotApplicable(ROUTER)),
    ("audit.cjs", Ported("src/kinds/audit.rs")),
    ("broken-windows.cjs", Ported("src/passes/windows.rs")),
    ("capability-*", NotApplicable(CAPABILITY)),
    ("check-auto-mode.cjs", NotApplicable(GATE_ENGINE)),
    ("check-command-router.cjs", NotApplicable(ROUTER)),
    ("cjs-command-router-adapter.cjs", NotApplicable(ROUTER)),
    (
        "claude-orchestration-command-router.cjs",
        NotApplicable(ROUTER),
    ),
    (
        "claude-orchestration.cjs",
        NotApplicable("runs Claude Code as a child; the harness is `specforged` (README)"),
    ),
    (
        "cli-exit.cjs",
        NotApplicable("gsd-tools' process exit codes; a command returns a typed result"),
    ),
    (
        "cli-skew-check.cjs",
        NotApplicable(
            "checks the CLI against the installed package version; the handshake carries `tracks` (D-r44-20)",
        ),
    ),
    ("clock.cjs", Ported("src/time.rs")),
    ("clusters.cjs", Ported("src/commands/complexity/mod.rs")),
    ("code-review-*", Owned("@gsd/ops")),
    ("codex-agent-toml.cjs", Host("specforge install")),
    ("command-aliases.cjs", Ported("src/slash.rs")),
    (
        "command-arg-projection.cjs",
        Ported("../kit/src/prompt/args.rs"),
    ),
    ("command-roster.cjs", Ported("tests/common/roster.rs")),
    ("command-routing-hub.cjs", NotApplicable(ROUTER)),
    ("commands.cjs", NotApplicable(ROUTER)),
    (
        "commonjs-marker.cjs",
        NotApplicable("a marker file for CommonJS in the config root; there is no node runtime"),
    ),
    (
        "complexity-trigger.cjs",
        Ported("src/commands/complexity/mod.rs"),
    ),
    ("config-schema.cjs", Ported("src/settings/keys.rs")),
    ("config-loader.cjs", Ported("src/settings/mod.rs")),
    ("config-types.cjs", Ported("src/settings/mod.rs")),
    ("config.cjs", Ported("src/settings/mod.rs")),
    ("configuration.cjs", Ported("src/settings/mod.rs")),
    ("context-composer.cjs", Ported("src/commands/digest.rs")),
    (
        "context-predicates.cjs",
        Ported("../kit/src/prompt/compose.rs"),
    ),
    ("context-utilization.cjs", Ported("src/hooks/statusline.rs")),
    (
        "core-utils.cjs",
        NotApplicable("helpers of the CommonJS runtime"),
    ),
    ("coverage.cjs", Ported("src/passes/decision_coverage.rs")),
    (
        "decision-coverage-support.cjs",
        Ported("src/passes/decision_coverage.rs"),
    ),
    ("decisions.cjs", Ported("src/passes/decision_coverage.rs")),
    ("docs.cjs", Owned("@gsd/codebase")),
    ("drift.cjs", Owned("@gsd/codebase")),
    ("edge-probe.cjs", Ported("src/passes/plan_shape.rs")),
    ("embedding-adapter.cjs", Owned("@gsd/codebase")),
    ("estimate-cli.cjs", Ported("src/commands/complexity/mod.rs")),
    ("eval-command-router.cjs", NotApplicable(ROUTER)),
    ("eval.cjs", Owned("@gsd/ai")),
    (
        "exit-code-registry.cjs",
        NotApplicable("gsd-tools' process exit codes; a command returns a typed result"),
    ),
    ("external-descriptor-trust.cjs", NotApplicable(CAPABILITY)),
    (
        "external-job.cjs",
        NotApplicable("runs a job outside the session; the harness is `specforged` (README)"),
    ),
    ("fallow-runner.cjs", Owned("@gsd/codebase")),
    ("federated-config.cjs", Ported("src/settings/mod.rs")),
    (
        "file-overlap-partitioner.cjs",
        Ported("src/passes/waves.rs"),
    ),
    ("frontmatter*", NotApplicable(DOCUMENT)),
    ("gap-checker.cjs", Ported("src/passes/roadmap.rs")),
    ("gate-api-coverage-verify-pre.cjs", Owned("@gsd/ai")),
    ("gate-codebase-drift.cjs", Owned("@gsd/codebase")),
    ("gate-context-drift.cjs", Ported("src/passes/drift.rs")),
    (
        "gate-decision-coverage-*",
        Ported("src/passes/decision_coverage.rs"),
    ),
    (
        "gate-gap-analysis-plan-post.cjs",
        Ported("src/passes/cross_plan.rs"),
    ),
    (
        "gate-prohibition-enforcement.cjs",
        Ported("src/passes/plan_shape.rs"),
    ),
    (
        "gate-schema-drift.cjs",
        Ported("src/passes/schema_drift.rs"),
    ),
    ("gate-tdd-*", Ported("src/passes/nyquist.rs")),
    ("gate-ui-*", Owned("@gsd/ui")),
    (
        "gate-verify-command-paths.cjs",
        Ported("src/passes/verify_paths.rs"),
    ),
    (
        "gate-verify-failure-directions.cjs",
        Ported("src/passes/verify_rules.rs"),
    ),
    ("gate-args.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-config.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-evaluation-scope.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-evidence.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-exit.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-phase-context.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-predicate-evaluator.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-predicate.cjs", NotApplicable(GATE_ENGINE)),
    ("gate-verdict.cjs", NotApplicable(GATE_ENGINE)),
    ("git-base-branch.cjs", Ported("src/collectors/git.rs")),
    ("git-info-exclude.cjs", NotApplicable(PLANNING_DIR)),
    ("graphify-command-router.cjs", NotApplicable(ROUTER)),
    ("graphify.cjs", Owned("@gsd/codebase")),
    ("gsd2-import.cjs", Owned("@gsd/import")),
    (
        "handshake-serialized.cjs",
        NotApplicable("the serialized capability handshake; the extension handshake is the SDK's"),
    ),
    ("health-diagnostic*", Ported("src/commands/reports.rs")),
    ("hook-bus.cjs", Host("specforge hook")),
    ("host-*", Host("specforge install")),
    ("init-command-router.cjs", NotApplicable(ROUTER)),
    ("init.cjs", Ported("src/commands/digest.rs")),
    ("install-*", Host("specforge install")),
    ("installed-surface-resolver.cjs", Host("specforge install")),
    ("installer-*", Host("specforge install")),
    ("intel-command-router.cjs", NotApplicable(ROUTER)),
    ("intel.cjs", Owned("@gsd/codebase")),
    ("io.cjs", NotApplicable("helpers of the CommonJS runtime")),
    ("jsonc-edit.cjs", Host("specforge install")),
    ("learnings.cjs", Ported("src/kinds/learning.rs")),
    ("legacy-cleanup.cjs", Host("specforge uninstall")),
    (
        "loop-*",
        NotApplicable(
            "the loop host contract of gsd-core's autonomous runner; the harness is `specforged` (README)",
        ),
    ),
    ("markdown-*", NotApplicable(DOCUMENT)),
    ("mcp-*", Host("specforge mcp")),
    ("milestone-lock.cjs", Ported("src/commands/milestone.rs")),
    ("milestone-release-map.cjs", NotApplicable(SUB_REPOS)),
    ("milestone-release-tags.cjs", NotApplicable(SUB_REPOS)),
    ("milestone.cjs", Ported("src/commands/milestone.rs")),
    ("model-adapter.cjs", Ported("src/settings/models.rs")),
    ("model-catalog.cjs", Ported("src/settings/models.rs")),
    ("model-profiles.cjs", Ported("src/settings/profiles.rs")),
    ("model-resolver.cjs", Ported("src/settings/models.rs")),
    (
        "normalize-test-command.cjs",
        Ported("src/collectors/junit.rs"),
    ),
    (
        "observability/*",
        NotApplicable("gsd-tools' event log and redaction; the kernel logs and redacts (K-series)"),
    ),
    ("onboard-projection.cjs", Ported("src/prompts/onboard.rs")),
    ("opencode-*", NotApplicable(RUNTIME)),
    ("package-identity.cjs", Ported("src/lib.rs")),
    ("package-legitimacy.cjs", Ported("src/judged.rs")),
    ("pattern.cjs", Ported("src/passes/scan/patterns.rs")),
    ("phase-command-router.cjs", NotApplicable(ROUTER)),
    ("phases-command-router.cjs", NotApplicable(ROUTER)),
    ("phase-estimation.cjs", Ported("src/commands/phase.rs")),
    ("phase-id-card.cjs", Ported("src/commands/phase.rs")),
    ("phase-id-display.cjs", Ported("src/commands/phase.rs")),
    ("phase-id.cjs", Ported("src/commands/phase.rs")),
    ("phase-lifecycle.cjs", Ported("src/commands/phase.rs")),
    ("phase-locator.cjs", Ported("src/commands/phase.rs")),
    ("phase-status.cjs", Ported("src/commands/phase.rs")),
    ("phase.cjs", Ported("src/commands/phase.rs")),
    ("plan-*", Ported("src/passes/plans.rs")),
    ("planning-command-router.cjs", NotApplicable(ROUTER)),
    ("planning-document.cjs", NotApplicable(PLANNING_DIR)),
    ("planning-inspect.cjs", NotApplicable(PLANNING_DIR)),
    ("planning-scope.cjs", NotApplicable(PLANNING_DIR)),
    ("planning-snapshot.cjs", NotApplicable(PLANNING_DIR)),
    ("planning-workspace.cjs", NotApplicable(PLANNING_DIR)),
    ("pr-branch-patterns.cjs", Ported("src/prompts/git.rs")),
    ("pristine-baseline.cjs", Host("specforge install")),
    ("probe-core.cjs", Ported("src/passes/plan_shape.rs")),
    ("profile-output.cjs", Owned("@gsd/ops")),
    ("profile-pipeline*", Owned("@gsd/ops")),
    (
        "prohibition-enforcement.cjs",
        Ported("src/passes/plan_shape.rs"),
    ),
    (
        "project-root.cjs",
        NotApplicable("finds the project root; the kernel resolves `specforge.json`"),
    ),
    ("prompt-budget.cjs", Ported("../kit/src/prompt/compose.rs")),
    ("quick-batch*", Ported("src/prompts/quick.rs")),
    // the kernel's GitCli sets GIT_OPTIONAL_LOCKS=0 on every git spawn (crates/adapters/git/src/lib.rs:68); hooks run it
    // with a 1.5 s timeout (crates/surfaces/cli/src/hook.rs:59)
    ("read-only-git-env.cjs", Host("specforge hook")),
    ("real-home-guard.cjs", Host("specforge install")),
    ("refactor-trigger-command-router.cjs", NotApplicable(ROUTER)),
    ("report-parser.cjs", NotApplicable(DOCUMENT)),
    ("research-provider.cjs", Ported("src/passes/research.rs")),
    ("research-store.cjs", Ported("src/kinds/research.rs")),
    ("resolution.cjs", Ported("src/commands/reference.rs")),
    ("retired-artifact-cleanup.cjs", Host("specforge uninstall")),
    ("review-*", Ported("src/prompts/review.rs")),
    (
        "reviewer-step-dispatch.cjs",
        Ported("src/prompts/review.rs"),
    ),
    ("roadmap-command-router.cjs", NotApplicable(ROUTER)),
    ("roadmap-parser.cjs", Ported("src/passes/roadmap.rs")),
    ("roadmap-upgrade.cjs", Ported("src/passes/roadmap.rs")),
    ("roadmap.cjs", Ported("src/passes/roadmap.rs")),
    ("runtime-*", Host("specforge install")),
    ("schema-detect.cjs", Ported("src/passes/schema_drift.rs")),
    ("secrets.cjs", Ported("src/hooks/secrets.rs")),
    ("section-manifest.cjs", NotApplicable(DOCUMENT)),
    (
        "security.cjs",
        NotApplicable(
            "gsd-tools' hardening library: path containment is the host's (file facts are project-relative, ADR 0293), arguments are typed, display text is escaped by the kernel; scanForInjection is upstream's CI scanner",
        ),
    ),
    (
        "semver-compare.cjs",
        NotApplicable(
            "compares package versions for the updater; specforge resolves extension versions",
        ),
    ),
    ("shell-command-projection.cjs", Host("specforge install")),
    ("smart-entry.cjs", Ported("src/commands/smart_entry.rs")),
    ("spec-section.cjs", NotApplicable(DOCUMENT)),
    (
        "stale-bake-guard.cjs",
        NotApplicable(
            "guards a built bundle against stale generated files; the wasm is built by cargo",
        ),
    ),
    ("state-command-router.cjs", NotApplicable(ROUTER)),
    ("state-contract.cjs", NotApplicable(STATE_MD)),
    ("state-document.cjs", NotApplicable(STATE_MD)),
    ("state-io.cjs", NotApplicable(STATE_MD)),
    ("state-md-schema.cjs", NotApplicable(STATE_MD)),
    ("state-transition.cjs", NotApplicable(STATE_MD)),
    ("state.cjs", NotApplicable(STATE_MD)),
    ("surface.cjs", Ported("src/commands/surface.rs")),
    ("task-command-router.cjs", NotApplicable(ROUTER)),
    ("task-content-resolution.cjs", Ported("src/kinds/plan.rs")),
    ("tdd-*", Ported("src/passes/nyquist.rs")),
    (
        "teams-status.cjs",
        NotApplicable("a team-status report of the Claude Code teams feature"),
    ),
    ("template.cjs", NotApplicable(DOCUMENT)),
    (
        "text-lines.cjs",
        NotApplicable("helpers of the CommonJS runtime"),
    ),
    ("token-scanner.cjs", Ported("src/hooks/guards.rs")),
    ("uat-predicate.cjs", Ported("src/passes/derive/uat.rs")),
    ("uat.cjs", Ported("src/passes/derive/uat.rs")),
    ("ui-*", Owned("@gsd/ui")),
    ("undo-commit-selection.cjs", Ported("src/prompts/git.rs")),
    (
        "unusable-input.cjs",
        NotApplicable("gsd-tools' argv error; typed commands refuse bad input"),
    ),
    ("update-context.cjs", Ported("src/prompts/update.rs")),
    ("user-artifact-staging.cjs", Host("specforge install")),
    ("validate-command-router.cjs", NotApplicable(ROUTER)),
    ("validate.cjs", Ported("src/commands/reports.rs")),
    (
        "vendor/*",
        NotApplicable(
            "vendored third-party JavaScript (js-yaml, re2js, saxes, tap-parser); the Rust crates and the kernel replace them",
        ),
    ),
    ("verification-command-router.cjs", NotApplicable(ROUTER)),
    ("verification.cjs", Ported("src/commands/verify.rs")),
    (
        "verify-command-grounding.cjs",
        Ported("src/passes/verify_paths/grounding.rs"),
    ),
    ("verify-command-router.cjs", NotApplicable(ROUTER)),
    ("verify.cjs", Ported("src/commands/verify.rs")),
    ("workflow-fragments.cjs", Ported("src/prompts/workflow.rs")),
    ("workstream*", Ported("src/commands/workstreams.rs")),
    ("worktree-*", Ported("src/hooks/isolation.rs")),
    ("write-set.cjs", Ported("src/passes/file_scope.rs")),
];
