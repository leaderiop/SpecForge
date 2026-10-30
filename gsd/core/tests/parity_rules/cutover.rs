//! The 171 verdicts of gsd-core's `gate-cutover` fixtures, each classified once (D-r44-22). A fixture is a project and
//! an argv for gsd-tools' `check.*` and `verify.*` verbs; its recorded stdout is the verdict.
//!
//! - `Reproduced(test)`: a twin gives the same blocker and the same items, in the named test (its body names the row).
//! - `ByConstruction(shape)`: the verdict is about the shape of an argument that the typed commands and the compile
//!   refuse or cannot express (the shape is one of `SHAPES`).
//! - `Owned(extension)`: the check belongs to an extension, not to the core.
//! - `NotApplicable(why)`: the check is the gate engine or the auto-mode runtime, not a rule.
//! - `NotReproduced(why)`: a core code whose content verdict no twin reaches; each is named in `NotReproduced` here and counted in ACCEPTANCE.md.
//! - `Diverges(why)`: a twin reaches the verdict and answers differently from upstream (REVIEW-r44 M9); each is
//!   listed under "Known divergences" in ACCEPTANCE.md.

pub enum Class {
    Reproduced(&'static str),
    ByConstruction(&'static str),
    Owned(&'static str),
    NotApplicable(&'static str),
    NotReproduced(&'static str),
    Diverges(&'static str),
}

pub const SHAPES: [&str; 12] = [
    "dotted verb",
    "environment or skip flag",
    "file absent or unreadable",
    "flag form",
    "no argument",
    "not json",
    "path escape",
    "path of the wrong kind",
    "phase directory lookup",
    "phase number form",
    "unknown verb",
    "unparsable text",
];

pub const GATE_CUTOVER: [(&str, Class); 171] = [
    (
        "api-coverage-verify-pre-coverage-unreadable",
        Class::Owned("@gsd/ai"),
    ),
    ("api-coverage-verify-pre-detected", Class::Owned("@gsd/ai")),
    (
        "api-coverage-verify-pre-empty-scope",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-matrix-invalid",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-matrix-suffixed-valid",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-matrix-valid",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-missing-arg",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-multiple-coverage-files",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-no-integration",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-no-phases-dir",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-none-declared",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-none-declared-overrides-detection",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-none-declared-scope-read-error",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-phase-unresolved",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-scope-read-error",
        Class::Owned("@gsd/ai"),
    ),
    (
        "api-coverage-verify-pre-token-only",
        Class::Owned("@gsd/ai"),
    ),
    (
        "auto-mode-auto-advance",
        Class::NotApplicable("check.auto-mode reads the auto-mode runtime flags, not a rule"),
    ),
    (
        "auto-mode-auto-chain",
        Class::NotApplicable("check.auto-mode reads the auto-mode runtime flags, not a rule"),
    ),
    (
        "auto-mode-both",
        Class::NotApplicable("check.auto-mode reads the auto-mode runtime flags, not a rule"),
    ),
    (
        "auto-mode-none",
        Class::NotApplicable("check.auto-mode reads the auto-mode runtime flags, not a rule"),
    ),
    ("check-unknown-verb", Class::ByConstruction("unknown verb")),
    (
        "decision-coverage-plan-all-covered",
        Class::Reproduced("gate_cutover_variants_reproduce_the_other_verdicts"),
    ),
    (
        "decision-coverage-plan-context-absent",
        Class::Reproduced("gate_cutover_decision_verdicts_reproduce"),
    ),
    (
        "decision-coverage-plan-context-flag",
        Class::ByConstruction("flag form"),
    ),
    (
        "decision-coverage-plan-context-is-directory",
        Class::ByConstruction("path of the wrong kind"),
    ),
    (
        "decision-coverage-plan-could-not-parse",
        Class::ByConstruction("unparsable text"),
    ),
    (
        "decision-coverage-plan-could-not-parse-partial",
        Class::ByConstruction("unparsable text"),
    ),
    (
        "decision-coverage-plan-disabled",
        Class::Reproduced("gate_cutover_variants_reproduce_the_other_verdicts"),
    ),
    (
        "decision-coverage-plan-dotted-verb",
        Class::ByConstruction("dotted verb"),
    ),
    (
        "decision-coverage-plan-missing-context-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "decision-coverage-plan-no-trackable",
        Class::Reproduced("gate_cutover_variants_reproduce_the_other_verdicts"),
    ),
    (
        "decision-coverage-plan-path-escape",
        Class::ByConstruction("path escape"),
    ),
    (
        "decision-coverage-plan-phase-dir-is-file",
        Class::ByConstruction("path of the wrong kind"),
    ),
    (
        "decision-coverage-plan-phase-dir-no-plans",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "decision-coverage-plan-phase-dir-not-found",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "decision-coverage-plan-uncovered",
        Class::Reproduced("gate_cutover_plan_verdicts_reproduce"),
    ),
    (
        "decision-coverage-verify-context-absent",
        Class::NotReproduced(
            "W506 judges a decision nothing shipped on a verified phase (unit-tested in src/passes/decision_coverage/tests.rs); the gate-cutover tree has no verified phase to run it on, so no twin gives this verdict. Not built in r44.",
        ),
    ),
    (
        "decision-coverage-verify-context-missing",
        Class::ByConstruction("no argument"),
    ),
    (
        "decision-coverage-verify-could-not-parse",
        Class::ByConstruction("unparsable text"),
    ),
    (
        "decision-coverage-verify-could-not-parse-partial",
        Class::ByConstruction("unparsable text"),
    ),
    (
        "decision-coverage-verify-disabled",
        Class::NotReproduced(
            "W506 judges a decision nothing shipped on a verified phase (unit-tested in src/passes/decision_coverage/tests.rs); the gate-cutover tree has no verified phase to run it on, so no twin gives this verdict. Not built in r44.",
        ),
    ),
    (
        "decision-coverage-verify-final",
        Class::NotReproduced(
            "W506 judges a decision nothing shipped on a verified phase (unit-tested in src/passes/decision_coverage/tests.rs); the gate-cutover tree has no verified phase to run it on, so no twin gives this verdict. Not built in r44.",
        ),
    ),
    (
        "decision-coverage-verify-no-trackable",
        Class::NotReproduced(
            "W506 judges a decision nothing shipped on a verified phase (unit-tested in src/passes/decision_coverage/tests.rs); the gate-cutover tree has no verified phase to run it on, so no twin gives this verdict. Not built in r44.",
        ),
    ),
    (
        "decision-coverage-verify-path-escape",
        Class::ByConstruction("path escape"),
    ),
    (
        "decision-coverage-verify-phase-dir-is-file",
        Class::ByConstruction("path of the wrong kind"),
    ),
    (
        "decision-coverage-verify-phase-dir-no-plans",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "decision-coverage-verify-phase-dir-not-found",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "gap-analysis-plan-post-disabled",
        Class::Reproduced("gate_cutover_variants_reproduce_the_other_verdicts"),
    ),
    (
        "gap-analysis-plan-post-enabled",
        Class::Reproduced("gate_cutover_plan_verdicts_reproduce"),
    ),
    (
        "gap-analysis-plan-post-missing-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "gap-analysis-plan-post-no-req-ids",
        Class::ByConstruction("no argument"),
    ),
    (
        "gap-analysis-plan-post-path-escape",
        Class::ByConstruction("path escape"),
    ),
    (
        "predicate-command-blocks",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-command-interpolates-phase-number",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-command-passes",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-frontmatter-artifact-not-found",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-frontmatter-match",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-frontmatter-mismatch",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-usage-evaluator-threw",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-usage-invalid-json",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-usage-missing-predicate",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "predicate-usage-phase-dir-escape",
        Class::NotApplicable(
            "check.predicate is the gate engine's predicate evaluator, not a rule",
        ),
    ),
    (
        "prohibition-enforcement-json-flag",
        Class::Reproduced("gate_cutover_prohibition_verdicts_reproduce"),
    ),
    (
        "prohibition-enforcement-json-flag-invalid",
        Class::ByConstruction("not json"),
    ),
    (
        "prohibition-enforcement-json-flag-without-value",
        Class::ByConstruction("flag form"),
    ),
    (
        "prohibition-enforcement-no-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "prohibition-enforcement-no-arg-contract-v2",
        Class::ByConstruction("no argument"),
    ),
    (
        "prohibition-enforcement-no-arg-json-errors",
        Class::ByConstruction("no argument"),
    ),
    (
        "prohibition-enforcement-request-file-absent",
        Class::ByConstruction("file absent or unreadable"),
    ),
    (
        "prohibition-enforcement-request-file-judgment-mode",
        Class::Diverges(
            "upstream flags a judgment-tier prohibition without evidence as unverified; E319 judges the test tier only, so the twin is silent where upstream flags",
        ),
    ),
    (
        "prohibition-enforcement-request-file-located-false",
        Class::Reproduced("gate_cutover_prohibition_verdicts_reproduce"),
    ),
    (
        "prohibition-enforcement-request-not-json",
        Class::ByConstruction("not json"),
    ),
    (
        "r2-tdd-review-checkpoint-escapes-planning",
        Class::ByConstruction("path escape"),
    ),
    (
        "r2-tdd-review-checkpoint-missing-phase",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "r2-tdd-review-checkpoint-numeric",
        Class::ByConstruction("phase number form"),
    ),
    (
        "r2-tdd-review-checkpoint-zero-padded",
        Class::ByConstruction("phase number form"),
    ),
    ("r2-ui-plan-gate-escapes-planning", Class::Owned("@gsd/ui")),
    ("r2-ui-plan-gate-missing-phase", Class::Owned("@gsd/ui")),
    ("r2-ui-plan-gate-numeric", Class::Owned("@gsd/ui")),
    ("r2-ui-plan-gate-zero-padded", Class::Owned("@gsd/ui")),
    (
        "r2-ui-safety-gate-escapes-planning",
        Class::Owned("@gsd/ui"),
    ),
    ("r2-ui-safety-gate-missing-phase", Class::Owned("@gsd/ui")),
    ("r2-ui-safety-gate-numeric", Class::Owned("@gsd/ui")),
    ("r2-ui-safety-gate-zero-padded", Class::Owned("@gsd/ui")),
    (
        "r4-bytes-cap-limit-minus-1-and-limit-and-limit-plus-1",
        Class::NotReproduced(
            "a byte and file cap on the text of CONTEXT.md, then the verify-time decision check; decisions are typed entities here with no scanned text to cap, and the check is W506 (no black-box twin, as the other decision-coverage-verify rows)",
        ),
    ),
    (
        "r4-files-cap-limit-minus-1-and-limit-and-limit-plus-1",
        Class::NotReproduced(
            "a byte and file cap on the text of CONTEXT.md, then the verify-time decision check; decisions are typed entities here with no scanned text to cap, and the check is W506 (no black-box twin, as the other decision-coverage-verify rows)",
        ),
    ),
    (
        "tdd-red-evidence-empty-object",
        Class::NotReproduced(
            "the red-evidence record is a JSON file of a test run's exit code and test count; SpecForge judges W403 from commits and a junit report and keeps no such record file",
        ),
    ),
    (
        "tdd-red-evidence-invalid-exit-zero",
        Class::NotReproduced(
            "the red-evidence record is a JSON file of a test run's exit code and test count; SpecForge judges W403 from commits and a junit report and keeps no such record file",
        ),
    ),
    (
        "tdd-red-evidence-invalid-zero-tests",
        Class::NotReproduced(
            "the red-evidence record is a JSON file of a test run's exit code and test count; SpecForge judges W403 from commits and a junit report and keeps no such record file",
        ),
    ),
    (
        "tdd-red-evidence-missing-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "tdd-red-evidence-ok",
        Class::NotReproduced(
            "the red-evidence record is a JSON file of a test run's exit code and test count; SpecForge judges W403 from commits and a junit report and keeps no such record file",
        ),
    ),
    (
        "tdd-red-evidence-path-escape",
        Class::ByConstruction("path escape"),
    ),
    (
        "tdd-red-evidence-record-not-found",
        Class::ByConstruction("file absent or unreadable"),
    ),
    (
        "tdd-red-evidence-record-not-json",
        Class::ByConstruction("not json"),
    ),
    (
        "tdd-review-checkpoint-missing-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "tdd-review-checkpoint-mixed-violation",
        Class::Diverges(
            "upstream blocks a TDD plan with no RED and GREEN commit; W403 judges a recorded GREEN with no RED before it and says nothing of a plan with no commits",
        ),
    ),
    (
        "tdd-review-checkpoint-no-tdd-plans",
        Class::Reproduced("gate_cutover_tdd_verdicts_reproduce"),
    ),
    (
        "tdd-review-checkpoint-pass",
        Class::Reproduced("gate_cutover_tdd_verdicts_reproduce"),
    ),
    (
        "tdd-review-checkpoint-red-only",
        Class::Diverges(
            "upstream blocks a TDD plan with a RED commit and no GREEN; W403 judges a GREEN without a RED, and a RED alone leaves it silent (probed: no finding)",
        ),
    ),
    (
        "ui-plan-gate-frontend-no-spec-blocks",
        Class::Owned("@gsd/ui"),
    ),
    ("ui-plan-gate-frontend-with-spec", Class::Owned("@gsd/ui")),
    (
        "ui-plan-gate-frontend-without-evidence",
        Class::Owned("@gsd/ui"),
    ),
    ("ui-plan-gate-missing-arg", Class::Owned("@gsd/ui")),
    ("ui-plan-gate-non-frontend", Class::Owned("@gsd/ui")),
    ("ui-safety-gate-blocks", Class::Owned("@gsd/ui")),
    ("ui-safety-gate-missing-arg", Class::Owned("@gsd/ui")),
    ("ui-safety-gate-no-ui-files", Class::Owned("@gsd/ui")),
    ("ui-safety-gate-non-frontend", Class::Owned("@gsd/ui")),
    ("ui-safety-gate-with-spec", Class::Owned("@gsd/ui")),
    (
        "verify-codebase-drift-below-threshold",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-blocks",
        Class::Owned("@gsd/codebase"),
    ),
    ("verify-codebase-drift-clean", Class::Owned("@gsd/codebase")),
    (
        "verify-codebase-drift-document-unreadable",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-dotted-verb",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-no-mapped-commit",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-no-structure-md",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-non-commit-baseline",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-not-a-git-repo",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-stamp-6-hex-abbreviation",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-stamp-64-hex-unresolvable",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-stamp-7-hex-abbreviation",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-stamp-option-shaped",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-stamp-ref-name",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-stamp-uppercase-hex",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-codebase-drift-unresolvable-mapped-commit",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-command-paths-dir-escapes-root",
        Class::ByConstruction("path escape"),
    ),
    (
        "verify-command-paths-dir-flag",
        Class::ByConstruction("flag form"),
    ),
    (
        "verify-command-paths-no-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-command-paths-phase-clean",
        Class::Reproduced("gate_cutover_plan_verdicts_reproduce"),
    ),
    (
        "verify-command-paths-phase-findings",
        Class::Reproduced("gate_cutover_plan_verdicts_reproduce"),
    ),
    (
        "verify-command-paths-phase-unresolved",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "verify-context-drift-dotted-verb",
        Class::ByConstruction("dotted verb"),
    ),
    (
        "verify-context-drift-in-sync",
        Class::Reproduced("gate_cutover_context_drift_verdicts_reproduce"),
    ),
    (
        "verify-context-drift-no-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-context-drift-no-arg-contract-v2",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-context-drift-no-arg-json-errors",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-context-drift-no-context-md",
        Class::NotReproduced(
            "a phase with research and no CONTEXT.md; SpecForge's W406 compares a recorded digest against a context entity, and a phase without a context has none to compare, so the twin has no such phase",
        ),
    ),
    (
        "verify-context-drift-no-phases-dir",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "verify-context-drift-no-upstream-artifacts",
        Class::Reproduced("gate_cutover_context_drift_verdicts_reproduce"),
    ),
    (
        "verify-context-drift-phase-not-found",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "verify-context-drift-stale-blocks",
        Class::Reproduced("gate_cutover_context_drift_verdicts_reproduce"),
    ),
    (
        "verify-context-drift-stale-warns",
        Class::Reproduced("gate_cutover_context_drift_verdicts_reproduce"),
    ),
    (
        "verify-failure-directions-no-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-failure-directions-phase-clean",
        Class::Reproduced("gate_cutover_plan_verdicts_reproduce"),
    ),
    (
        "verify-failure-directions-phase-findings",
        Class::Reproduced("gate_cutover_plan_verdicts_reproduce"),
    ),
    (
        "verify-failure-directions-phase-unresolved",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "verify-schema-drift-blocks",
        Class::Reproduced("gate_cutover_schema_drift_verdicts_reproduce"),
    ),
    (
        "verify-schema-drift-clean",
        Class::Reproduced("gate_cutover_schema_drift_verdicts_reproduce"),
    ),
    (
        "verify-schema-drift-dotted-verb",
        Class::ByConstruction("dotted verb"),
    ),
    (
        "verify-schema-drift-env-not-true",
        Class::Reproduced("gate_cutover_schema_drift_verdicts_reproduce"),
    ),
    (
        "verify-schema-drift-env-skip",
        Class::Reproduced("gate_cutover_schema_drift_verdicts_reproduce"),
    ),
    (
        "verify-schema-drift-no-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-schema-drift-no-arg-contract-v2",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-schema-drift-no-arg-json-errors",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-schema-drift-no-phases-dir",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "verify-schema-drift-phase-not-found",
        Class::ByConstruction("phase directory lookup"),
    ),
    (
        "verify-schema-drift-plan-unreadable",
        Class::ByConstruction("file absent or unreadable"),
    ),
    (
        "verify-schema-drift-plan-unreadable-skipped",
        Class::ByConstruction("file absent or unreadable"),
    ),
    (
        "verify-schema-drift-pushed",
        Class::Reproduced("gate_cutover_schema_drift_verdicts_reproduce"),
    ),
    (
        "verify-surface-codebase-drift-blocks",
        Class::Owned("@gsd/codebase"),
    ),
    (
        "verify-surface-context-drift-no-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-surface-context-drift-stale-blocks",
        Class::Reproduced("gate_cutover_context_drift_verdicts_reproduce"),
    ),
    (
        "verify-surface-context-drift-stale-warns",
        Class::Reproduced("gate_cutover_context_drift_verdicts_reproduce"),
    ),
    (
        "verify-surface-schema-drift-blocks",
        Class::Reproduced("gate_cutover_schema_drift_verdicts_reproduce"),
    ),
    (
        "verify-surface-schema-drift-env-ignored",
        Class::Reproduced("gate_cutover_schema_drift_verdicts_reproduce"),
    ),
    (
        "verify-surface-schema-drift-no-arg",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-surface-schema-drift-no-arg-contract-v2",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-surface-schema-drift-no-arg-json-errors",
        Class::ByConstruction("no argument"),
    ),
    (
        "verify-surface-schema-drift-skip-flag",
        Class::ByConstruction("environment or skip flag"),
    ),
    (
        "verify-surface-schema-drift-skip-flag-first",
        Class::ByConstruction("environment or skip flag"),
    ),
    (
        "verify-surface-schema-drift-skip-flag-no-phase",
        Class::ByConstruction("environment or skip flag"),
    ),
];
