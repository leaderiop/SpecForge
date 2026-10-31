//! `audit_uat` and `audit_milestone` (D-r42-1 rows 14-15): the audits that read across phases. `audit_uat` has no
//! subject and no gate; `audit_milestone` is about one milestone and refuses under `gsd-plus` while a verdict is stale.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Do, Slash, Spec, Step};

const UAT_STEPS: &[Step] = &[Step {
    id: "uat",
    title: "Ask the person to run `specforge gsd set-uat-result <uat_test> <result> --by <who>` in their terminal (an agent's call is refused: it records what a person says)",
    does: Do::Command("set_uat_result"),
}];

const MILESTONE_STEPS: &[Step] = &[
    Step {
        id: "integrate",
        title: "Check the wiring across the phases (gsd-integration-checker)",
        does: Do::Agent("gsd-integration-checker"),
    },
    Step {
        id: "record_verdict",
        title: "Record the integration checker's verdicts (specforge gsd record-verdict <phase> E518 <outcome> <model> '<basis>')",
        does: Do::Command("record_verdict"),
    },
    Step {
        id: "plan_gaps",
        title: "Plan the closure of the gaps the audit found (the prompt plan_phase)",
        does: Do::Prompt("plan_phase"),
    },
];

/// The integration check is a judged rule: the checker records its verdict, and the text says which.
fn integration(_: &PromptCall<'_>) -> String {
    "\n\nThe integration checker's finding is the judged rule E518: record its verdict with `specforge gsd record-verdict \
     <phase> E518 <outcome> <model> '<basis>'` once per phase it checked.\n"
        .to_string()
}

static AUDIT_UAT: Spec = Spec {
    id: "audit_uat",
    title: "Audit the outstanding UAT items",
    description: "Cross-phase audit of all outstanding UAT and verification items",
    command: "audit-uat",
    args: &[],
    subject: None,
    gates: &[],
    steps: UAT_STEPS,
    slash: Some(Slash {
        hint: "",
        tools: &["read", "search", "shell"],
        effort: None,
    }),
    reads_state: false,
    tail: None,
};

static AUDIT_MILESTONE: Spec = Spec {
    id: "audit_milestone",
    title: "Audit a milestone",
    description: "Audit milestone completion against original intent before archiving",
    command: "audit-milestone",
    args: &[Arg::reference("milestone", "milestone")],
    subject: Some("milestone"),
    gates: &["E516"],
    steps: MILESTONE_STEPS,
    slash: Some(Slash {
        hint: "<milestone>",
        tools: &["read", "search", "shell", "agent", "write"],
        effort: None,
    }),
    reads_state: false,
    tail: Some(integration),
};

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &AUDIT_UAT);
    workflow::declare(c, &AUDIT_MILESTONE);
}
