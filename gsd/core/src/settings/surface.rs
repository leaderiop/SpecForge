//! Ported from gsd-core v1.18.0 `src/clusters.cts` (MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD).
//!
//! The skill clusters (D-r41-13): ten named groups of skill stems. Upstream's `/gsd:surface` enables and disables a
//! cluster without a reinstall; here each cluster is a bool setting `specforge.surface.<cluster>` (default on) and a
//! slash command is shown while the setting of the first cluster that lists it is on.

use specforge_extension_sdk::prelude::*;

/// The clusters in upstream's order, each with its members. A stem may be in two clusters.
pub const CLUSTERS: &[(&str, &[&str])] = &[
    (
        "core_loop",
        &[
            "next",
            "new-project",
            "onboard",
            "discuss-phase",
            "plan-phase",
            "execute-phase",
            "help",
            "update",
        ],
    ),
    (
        "audit_review",
        &[
            "code-review",
            "review",
            "audit-fix",
            "audit-milestone",
            "audit-uat",
            "verify-work",
            "validate-phase",
            "plan-review-convergence",
            "eval-review",
            "add-tests",
            "secure-phase",
        ],
    ),
    (
        "milestone",
        &[
            "new-milestone",
            "complete-milestone",
            "milestone-summary",
            "health",
        ],
    ),
    (
        "research_ideate",
        &[
            "sketch",
            "spike",
            "forensics",
            "explore",
            "graphify",
            "ns-ideate",
        ],
    ),
    (
        "workspace_state",
        &[
            "pause-work",
            "resume-work",
            "workspace",
            "workstreams",
            "thread",
            "capture",
            "inbox",
        ],
    ),
    ("docs", &["docs-update", "ingest-docs"]),
    ("ui", &["ui-phase", "ui-review"]),
    ("ai_eval", &["ai-integration-phase", "eval-review"]),
    (
        "ns_meta",
        &[
            "ns-context",
            "ns-ideate",
            "ns-manage",
            "ns-project",
            "ns-review",
            "ns-workflow",
        ],
    ),
    (
        "utility",
        &[
            "health",
            "stats",
            "settings",
            "cleanup",
            "pr-branch",
            "ship",
            "undo",
            "fast",
            "quick",
            "quick-batch",
            "autonomous",
            "config",
            "progress",
            "phase",
            "review",
            "update",
            "help",
            "code-review",
            "import",
            "manager",
            "map-codebase",
            "profile-user",
            "spec-phase",
            "ultraplan-phase",
            "mvp-phase",
            "execute-phase",
            "review-backlog",
            "debug",
            "extract-learnings",
            "mempalace-recall",
            "mempalace-capture",
            "surface",
        ],
    ),
];

/// The setting that shows or hides `cluster`.
pub fn setting(cluster: &str) -> String {
    format!("specforge.surface.{cluster}")
}

/// The first cluster that lists the skill `stem`, upstream's order: the one whose setting hides it (a slash command
/// takes one `when`, upstream hides a skill in any disabled cluster).
pub fn first_cluster(stem: &str) -> Option<&'static str> {
    CLUSTERS
        .iter()
        .find(|(_, members)| members.contains(&stem))
        .map(|(name, _)| *name)
}

pub fn declare(c: &mut ContributionsBuilder) {
    for (cluster, members) in CLUSTERS {
        c.setting(&setting(cluster), |s| {
            s.bool().default(true).describe(&format!(
                "Show the skills of the {cluster} cluster ({} skills): turn it off and reinstall to hide them",
                members.len()
            ));
        });
    }
}
