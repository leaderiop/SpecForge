//! The ported workflows, references, templates and command files (D-r42-2), embedded under the path they have in
//! gsd-core v1.18.0. Every `.md` starts with its attribution line; the four JSON files cannot, so this notice covers
//! them: `references/prohibition-probe-fixtures/*/expected.json` and `workflows/discuss-phase/templates/checkpoint.json`
//! are gsd-core v1.18.0 files, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD.

macro_rules! asset {
    ($path:literal) => {
        (
            $path,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)),
        )
    };
}

/// `(path, text)`, sorted by path.
pub static ASSETS: &[(&str, &str)] = &[
    asset!("commands/gsd/new-milestone.md"),
    asset!("commands/gsd/new-project.md"),
    asset!("gsd-core/references/agent-skills-bootstrap.md"),
    asset!("gsd-core/references/compact-content-gate.md"),
    asset!("gsd-core/references/doc-conflict-engine.md"),
    asset!("gsd-core/references/failing-direction.md"),
    asset!("gsd-core/references/few-shot-examples/plan-checker.md"),
    asset!("gsd-core/references/few-shot-examples/verifier.md"),
    asset!("gsd-core/references/mandatory-initial-read.md"),
    asset!("gsd-core/references/model-profile-resolution.md"),
    asset!("gsd-core/references/model-profiles.md"),
    asset!("gsd-core/references/nyquist-compliance.md"),
    asset!("gsd-core/references/plan-checker-examples.md"),
    asset!("gsd-core/references/planner-chunked.md"),
    asset!("gsd-core/references/planner-coupling.md"),
    asset!("gsd-core/references/planner-interface-context.md"),
    asset!("gsd-core/references/planner-load-graph-context.md"),
    asset!("gsd-core/references/planner-quick-batch.md"),
    asset!("gsd-core/references/planner-source-audit.md"),
    asset!("gsd-core/references/planner-verify-command-grounding.md"),
    asset!("gsd-core/references/project-skills-discovery.md"),
    asset!("gsd-core/references/questioning.md"),
    asset!("gsd-core/references/research-documentation-lookup.md"),
    asset!("gsd-core/references/research-philosophy.md"),
    asset!("gsd-core/references/research-verification-protocol.md"),
    asset!("gsd-core/references/runtime-aware-dispatch.md"),
    asset!("gsd-core/references/security-asvs-levels.md"),
    asset!("gsd-core/references/thinking-models-execution.md"),
    asset!("gsd-core/references/thinking-models-research.md"),
    asset!("gsd-core/references/thinking-models-verification.md"),
    asset!("gsd-core/references/ui-brand.md"),
    asset!("gsd-core/references/verification-overrides.md"),
    asset!("gsd-core/references/verification-patterns.md"),
    asset!("gsd-core/references/verifier-evidence-gate.md"),
    asset!("gsd-core/references/verifier-phase-gates.md"),
    asset!("gsd-core/references/verifier-wiring-patterns.md"),
    asset!("gsd-core/references/verify-command-path-resolvability.md"),
    asset!("gsd-core/references/workstream-flag.md"),
    asset!("gsd-core/templates/project.md"),
    asset!("gsd-core/templates/requirements.md"),
    asset!("gsd-core/templates/research-project/ARCHITECTURE.md"),
    asset!("gsd-core/templates/research-project/FEATURES.md"),
    asset!("gsd-core/templates/research-project/PITFALLS.md"),
    asset!("gsd-core/templates/research-project/STACK.md"),
    asset!("gsd-core/templates/research-project/SUMMARY.md"),
    asset!("gsd-core/templates/roadmap.md"),
    asset!("gsd-core/templates/state.md"),
    asset!("gsd-core/templates/verification-report.md"),
    asset!("gsd-core/workflows/new-milestone.md"),
    asset!("gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md"),
    asset!("gsd-core/workflows/new-milestone/steps/reset-phase-safety.md"),
    asset!("gsd-core/workflows/new-project.md"),
    asset!("gsd-core/workflows/new-project/detail/elaboration.md"),
    asset!("gsd-core/workflows/new-project/steps/auto-mode-config.md"),
    asset!("gsd-core/workflows/new-project/steps/auto-mode-detection.md"),
    asset!("gsd-core/workflows/new-project/steps/codebase-map-offer.md"),
];

/// The text of the asset at `path` (its attribution line included).
pub fn get(path: &str) -> Option<&'static str> {
    ASSETS
        .binary_search_by_key(&path, |(p, _)| p)
        .ok()
        .map(|i| ASSETS[i].1)
}

/// The paths of every embedded asset.
pub fn paths() -> impl Iterator<Item = &'static str> {
    ASSETS.iter().map(|(p, _)| *p)
}
