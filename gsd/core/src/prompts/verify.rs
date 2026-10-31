//! `verify_work`, `validate_phase` and `add_tests` (D-r42-1 rows 11-13): the audit-and-review prompts of one phase.
//! Their gate is E516 alone: under `gsd-plus` a stale verdict refuses them.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const GATES: &[&str] = &["E516"];

const VERIFY_STEPS: &[Step] = &[
    Step {
        id: "uat",
        title: "Ask the person to run `specforge gsd set-uat-result <uat_test> <result> --by <who>` in their terminal (an agent's call is refused: it records what a person says)",
        does: Do::Command("set_uat_result"),
    },
    Step {
        id: "plan_gaps",
        title: "Plan the fixes of the gaps the tests found",
        does: Do::Agent("gsd-planner"),
    },
    Step {
        id: "check",
        title: "Check the fix plans",
        does: Do::Agent("gsd-plan-checker"),
    },
];

const VALIDATE_STEPS: &[Step] = &[
    Step {
        id: "audit",
        title: "Audit the validation and fill its gaps",
        does: Do::Agent("gsd-nyquist-auditor"),
    },
    Step {
        id: "record",
        title: "Record the validation's context digest (specforge gsd record-validation <validation>)",
        does: Do::Command("record_validation"),
    },
    Step {
        id: "approve",
        title: "Ask the person to run `specforge gsd approve-validation <validation> <who>` in their terminal (an agent's call is refused: it records what a person says)",
        does: Do::Command("approve_validation"),
    },
];

/// The UAT file is the `uat_session` entity: a person's answer to a test is recorded with the command.
fn uat(_: &PromptCall<'_>) -> String {
    "\n\nA test that passing tests already cover is not put to the person: record it yourself with \
     `specforge gsd record-automated-uat <uat_test> <pass|issue> --by <who>` (an issue takes `--reported \"<what the run showed>\"`); \
     it is written `source: automated`.\n\nFor each answer of the person, ask them to run `specforge gsd set-uat-result <uat_test> <pass|issue|skipped|blocked|pending> \
     --by <who>` (an issue takes `--reported \"<their words>\"`, a skipped takes `--reason \"<why>\"`, a blocked takes \
     `--blocked-by <server|physical_device|release_build|third_party|prior_phase>`) in their terminal (an agent's call is refused: it records what a person says): the UAT file's `result:` lines are not \
     edited by hand.\n"
        .to_string()
}

static VERIFY_WORK: Spec = Spec {
    id: "verify_work",
    title: "Verify the built work with the person",
    description: "Validate built features through conversational UAT",
    command: "verify-work",
    args: &[Arg::reference("phase", "phase"), Arg::text("ws")],
    subject: Some("phase"),
    gates: GATES,
    steps: VERIFY_STEPS,
    slash: Some(Slash {
        hint: "<phase> [--ws <name>]",
        tools: &["read", "shell", "search", "edit", "write", "agent"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(uat),
};

static VALIDATE_PHASE: Spec = Spec {
    id: "validate_phase",
    title: "Validate a phase",
    description: "Retroactively audit and fill Nyquist validation gaps for a completed phase",
    command: "validate-phase",
    args: &[Arg::reference("phase", "phase")],
    subject: Some("phase"),
    gates: GATES,
    steps: VALIDATE_STEPS,
    slash: Some(Slash {
        hint: "<phase>",
        tools: &[
            "read", "write", "edit", "shell", "search", "agent", "ask_user",
        ],
        effort: None,
    }),
    reads_state: false,
    tail: None,
};

static ADD_TESTS: Spec = Spec {
    id: "add_tests",
    title: "Add tests for a phase",
    description: "Generate tests for a completed phase based on UAT criteria and implementation",
    command: "add-tests",
    args: &[Arg::reference("phase", "phase"), Arg::text("instructions")],
    subject: Some("phase"),
    gates: GATES,
    steps: &[],
    slash: Some(Slash {
        hint: "<phase> [--instructions \"...\"]",
        tools: &[
            "read", "write", "edit", "shell", "search", "agent", "ask_user",
        ],
        effort: None,
    }),
    reads_state: false,
    tail: None,
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &VERIFY_WORK);
    workflow::declare(c, &VALIDATE_PHASE);
    workflow::declare(c, &ADD_TESTS);
}
