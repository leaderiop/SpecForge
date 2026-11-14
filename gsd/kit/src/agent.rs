//! Agents shared by the extensions: the attribution of a ported agent, and the roles that say which of SpecForge's
//! engine tools each agent is granted (ADR 0334: an extension's own table, built into each agent's `tools`).

use specforge_extension_sdk::prelude::*;

/// The notice every ported agent carries: its upstream file (`path`, as upstream's tree names it), version and licence.
pub fn attribution(path: &str) -> Attribution {
    Attribution {
        upstream: "gsd-core".to_string(),
        path: path.to_string(),
        version: concat!("v", crate::tracked_version!()).to_string(),
        license: "MIT".to_string(),
        copyright: "Copyright (c) 2026 Open GSD".to_string(),
    }
}

/// What an agent's work needs of SpecForge's engine tools, as `specforge:` classes named one by one (never a group,
/// so no role gains a tool through an expansion).
#[derive(Debug, PartialEq, Eq)]
pub struct Role {
    pub id: &'static str,
    /// Granted to every agent of the role.
    pub reads: &'static [&'static str],
    /// Granted only to an agent whose upstream tools write: an upstream read-only agent stays one (#2119).
    pub writes: &'static [&'static str],
}

/// Reads the code and the graph around a question, and reports what it found.
pub const RESEARCHER: Role = Role {
    id: "researcher",
    reads: &[
        "specforge:query",
        "specforge:search",
        "specforge:inspect",
        "specforge:trace",
        "specforge:find_spec_for_source",
        "specforge:find_implementation",
        "specforge:explain",
        "specforge:schema",
        "specforge:context",
    ],
    writes: &["specforge:edit", "specforge:collect"],
};

/// Turns a phase into plans and checks them against the graph.
pub const PLANNER: Role = Role {
    id: "planner",
    reads: &[
        "specforge:query",
        "specforge:search",
        "specforge:trace",
        "specforge:find_spec_for_source",
        "specforge:find_implementation",
        "specforge:validate",
        "specforge:explain",
        "specforge:export",
        "specforge:schema",
        "specforge:find_references",
        "specforge:context",
    ],
    writes: &["specforge:edit", "specforge:collect", "specforge:gsd.phase"],
};

/// Judges a plan or a whole against the graph; writes nothing.
pub const CHECKER: Role = Role {
    id: "checker",
    reads: &[
        "specforge:query",
        "specforge:search",
        "specforge:trace",
        "specforge:find_implementation",
        "specforge:find_references",
        "specforge:validate",
        "specforge:explain",
        "specforge:context",
    ],
    writes: &[],
};

/// Carries out a plan: edits, records test results and the plan's progress.
pub const EXECUTOR: Role = Role {
    id: "executor",
    reads: &[
        "specforge:query",
        "specforge:find_implementation",
        "specforge:find_spec_for_source",
        "specforge:validate",
        "specforge:explain",
        "specforge:gsd.branch_check",
        "specforge:context",
    ],
    writes: &[
        "specforge:edit",
        "specforge:collect",
        "specforge:gsd.start_plan",
        "specforge:gsd.complete_plan",
        "specforge:gsd.raise_blocker",
        "specforge:gsd.resolve_blocker",
        "specforge:gsd.record_window",
        "specforge:gsd.pause",
    ],
};

/// Checks that what was built is what the graph asks, and records the tests' results.
pub const VERIFIER: Role = Role {
    id: "verifier",
    reads: &[
        "specforge:query",
        "specforge:trace",
        "specforge:find_implementation",
        "specforge:find_references",
        "specforge:validate",
        "specforge:explain",
        "specforge:gsd.user_story_validate",
        "specforge:context",
        "specforge:find_spec_for_source",
    ],
    writes: &["specforge:collect"],
};

/// Every role.
pub const ROLES: &[&Role] = &[&RESEARCHER, &PLANNER, &CHECKER, &EXECUTOR, &VERIFIER];

/// What an agent that judges rules is granted besides its role: the rule's inputs and the command recording a verdict.
pub const JUDGE: &[&str] = &["specforge:verdict_inputs", "specforge:gsd.record_verdict"];

/// The tools an agent declares: its upstream classes, its role's reads, its role's writes when an upstream class
/// writes (`write` or `edit`), and [`JUDGE`] when it judges rules.
pub fn tools(upstream: &[&'static str], role: &Role, judges: bool) -> Vec<&'static str> {
    let writes = upstream.iter().any(|c| matches!(*c, "write" | "edit"));
    let mut out = upstream.to_vec();
    out.extend(role.reads);
    if writes {
        out.extend(role.writes);
    }
    if judges {
        out.extend(JUDGE);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_extension_sdk::engine_tools::{EngineClass, EngineGroup, parse_class};
    use std::collections::BTreeSet;

    fn classes() -> impl Iterator<Item = &'static str> {
        ROLES
            .iter()
            .flat_map(|r| r.reads.iter().chain(r.writes))
            .chain(JUDGE)
            .copied()
    }

    #[test]
    fn every_role_class_is_an_engine_tool_or_a_gsd_command() {
        for class in classes() {
            match parse_class(class) {
                Some(EngineClass::Tool(_) | EngineClass::Command { short: "gsd", .. }) => {}
                other => panic!("{class} is {other:?}"),
            }
        }
    }

    #[test]
    fn no_role_grants_coverage_or_a_setup_tool() {
        for class in classes() {
            if let Some(EngineClass::Tool(tool)) = parse_class(class) {
                assert_ne!(tool.name, "coverage", "{class}");
                assert_ne!(tool.group, EngineGroup::Setup, "{class}");
            }
        }
    }

    #[test]
    fn a_role_writes_only_for_an_agent_whose_upstream_tools_write() {
        let reader = tools(&["read", "shell", "search"], &EXECUTOR, false);
        assert_eq!(&reader[3..], EXECUTOR.reads);
        let writer = tools(&["read", "edit"], &EXECUTOR, false);
        let (reads, writes) = writer[2..].split_at(EXECUTOR.reads.len());
        assert_eq!((reads, writes), (EXECUTOR.reads, EXECUTOR.writes));
        assert!(
            !tools(&["read", "write"], &CHECKER, false).contains(&"specforge:edit"),
            "a checker never writes"
        );
    }

    #[test]
    fn a_judge_reads_its_inputs_and_records_its_verdict() {
        let judge = tools(&["read"], &CHECKER, true);
        assert!(judge.ends_with(JUDGE), "{judge:?}");
        assert!(!tools(&["read"], &CHECKER, false).contains(&"specforge:verdict_inputs"));
    }

    #[test]
    fn the_roles_name_fifteen_graph_tools() {
        let graph: BTreeSet<&str> = classes()
            .filter_map(|c| match parse_class(c) {
                Some(EngineClass::Tool(t)) if t.group.graph() => Some(t.name),
                _ => None,
            })
            .collect();
        assert_eq!(graph.len(), 15, "{graph:?}");
    }
}
