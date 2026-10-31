//! The six `/gsd-ns-*` routers (D-r43-15): each renders upstream's body, which matches the user's intent to one skill
//! of its cluster, followed by the route table of this install. The table reads the slash commands the project serves
//! (`PromptCall::slash_commands`, ADR 0316): a target is served when a slash command named `gsd-<target>` is.

use specforge_extension_sdk::prelude::*;

use super::args::Arg;
use super::workflow::{self, Slash, Spec};

/// The route table: one row per target, in the order of upstream's `requires:`.
fn table(call: &PromptCall<'_>, targets: &[&str]) -> String {
    let mut out = String::from(
        "\n\n## Route table on this install\n\n\
         Invoke the matched skill from this table. A target that is not served on this project is named so: say so \
         and stop; do not run another skill in its place.\n\n| Target | On this install |\n|---|---|\n",
    );
    for target in targets {
        out.push_str(&format!("| `/gsd-{target}` | {} |\n", served(call, target)));
    }
    out
}

/// What the install says of `/gsd-<stem>`: how it is served, or that it is not.
pub(super) fn served(call: &PromptCall<'_>, stem: &str) -> String {
    let name = format!("gsd-{stem}");
    match call.slash_commands().iter().find(|s| s.name == name) {
        Some(s) => match (&s.prompt, &s.tool) {
            (Some(prompt), _) => format!("prompt `{prompt}` (served by {})", s.extension),
            (None, _) => format!("command `{}` (served by {})", s.cli, s.extension),
        },
        None => "not served on this project".to_string(),
    }
}

macro_rules! router {
    ($spec:ident, $tail:ident, $id:literal, $command:literal, $title:literal, $description:literal, $targets:expr) => {
        fn $tail(call: &PromptCall<'_>) -> String {
            table(call, $targets)
        }

        static $spec: Spec = Spec {
            id: $id,
            title: $title,
            description: $description,
            command: $command,
            args: &[Arg::rest("intent")],
            subject: None,
            gates: &[],
            steps: &[],
            slash: Some(Slash {
                hint: "",
                tools: &["read", "skill"],
                effort: None,
            }),
            reads_state: false,
            reads_slash_commands: true,
            tail: Some($tail),
        };
    };
}

router!(
    CONTEXT,
    context,
    "ns_context",
    "ns-context",
    "Route to the codebase and learnings skills",
    "codebase intel | map graphify docs learnings mempalace",
    &[
        "map-codebase",
        "graphify",
        "docs-update",
        "extract-learnings",
        "mempalace-recall",
        "mempalace-capture",
    ]
);
router!(
    IDEATE,
    ideate,
    "ns_ideate",
    "ns-ideate",
    "Route to the exploration and capture skills",
    "exploration capture | explore sketch spike spec capture",
    &["capture", "explore", "sketch", "spike", "spec-phase"]
);
router!(
    MANAGE,
    manage,
    "ns_manage",
    "ns-manage",
    "Route to the configuration and workspace skills",
    "config workspace | workstreams thread update ship inbox",
    &[
        "config",
        "workspace",
        "workstreams",
        "thread",
        "pause-work",
        "resume-work",
        "update",
        "ship",
        "inbox",
        "pr-branch",
        "undo",
        "cleanup",
        "health",
        "manager",
        "settings",
        "stats",
        "surface",
        "help",
    ]
);
router!(
    PROJECT,
    project,
    "ns_project",
    "ns-project",
    "Route to the project lifecycle skills",
    "project lifecycle | milestones audits summary",
    &[
        "new-project",
        "onboard",
        "new-milestone",
        "complete-milestone",
        "audit-milestone",
        "milestone-summary",
        "import",
        "ingest-docs",
        "profile-user",
        "review-backlog",
    ]
);
router!(
    REVIEW,
    review,
    "ns_review",
    "ns-review",
    "Route to the quality gate skills",
    "quality gates | code review debug audit security eval ui",
    &[
        "code-review",
        "audit-uat",
        "secure-phase",
        "eval-review",
        "ui-review",
        "validate-phase",
        "debug",
        "forensics",
        "audit-fix",
        "review",
        "ui-phase",
    ]
);
router!(
    WORKFLOW,
    workflow,
    "ns_workflow",
    "ns-workflow",
    "Route to the phase pipeline skills",
    "workflow | discuss plan execute verify phase progress",
    &[
        "discuss-phase",
        "spec-phase",
        "plan-phase",
        "execute-phase",
        "verify-work",
        "phase",
        "progress",
        "next",
        "ultraplan-phase",
        "plan-review-convergence",
        "add-tests",
        "ai-integration-phase",
        "autonomous",
        "fast",
        "mvp-phase",
        "quick",
        "quick-batch",
    ]
);

pub fn declare(c: &mut ContributionsBuilder) {
    workflow::declare(c, &CONTEXT);
    workflow::declare(c, &IDEATE);
    workflow::declare(c, &MANAGE);
    workflow::declare(c, &PROJECT);
    workflow::declare(c, &REVIEW);
    workflow::declare(c, &WORKFLOW);
}
