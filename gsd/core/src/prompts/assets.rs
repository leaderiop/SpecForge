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
    asset!("gsd-core/references/agent-skills-bootstrap.md"),
    asset!("gsd-core/references/doc-conflict-engine.md"),
    asset!("gsd-core/references/failing-direction.md"),
    asset!("gsd-core/references/few-shot-examples/plan-checker.md"),
    asset!("gsd-core/references/few-shot-examples/verifier.md"),
    asset!("gsd-core/references/mandatory-initial-read.md"),
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
    asset!("gsd-core/references/research-documentation-lookup.md"),
    asset!("gsd-core/references/research-philosophy.md"),
    asset!("gsd-core/references/research-verification-protocol.md"),
    asset!("gsd-core/references/security-asvs-levels.md"),
    asset!("gsd-core/references/thinking-models-execution.md"),
    asset!("gsd-core/references/thinking-models-research.md"),
    asset!("gsd-core/references/thinking-models-verification.md"),
    asset!("gsd-core/references/verification-overrides.md"),
    asset!("gsd-core/references/verification-patterns.md"),
    asset!("gsd-core/references/verifier-evidence-gate.md"),
    asset!("gsd-core/references/verifier-phase-gates.md"),
    asset!("gsd-core/references/verifier-wiring-patterns.md"),
    asset!("gsd-core/references/verify-command-path-resolvability.md"),
    asset!("gsd-core/templates/roadmap.md"),
    asset!("gsd-core/templates/state.md"),
    asset!("gsd-core/templates/verification-report.md"),
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
