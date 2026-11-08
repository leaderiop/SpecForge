//! The body lines (0-based, inclusive runs) rewritten because a plan lists its decisions (`decisions`, ADR 0351) and
//! a task commit ends with its `Gsd-Task: {phase}-{plan}/{task}` trailer. Paths are under `assets/`; an agent's is
//! `agents/<file>`. `port` and `port_workflows` read them as more rewritten rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    // the planner lists the decisions a plan implements in its `decisions` (not required: E310 says when one is
    // missing); `wave` and `autonomous` are computed, so the template and the fields table name them as such
    ("agents/gsd-planner.md", 47, 47),
    ("agents/gsd-planner.md", 55, 56),
    ("agents/gsd-planner.md", 301, 301),
    ("agents/gsd-planner.md", 304, 304),
    ("agents/gsd-planner.md", 391, 391),
    ("agents/gsd-planner.md", 394, 394),
    ("agents/gsd-planner.md", 400, 400),
    ("agents/gsd-planner.md", 759, 759),
    ("agents/gsd-planner.md", 856, 856),
    ("agents/gsd-planner.md", 969, 969),
    // the plan checker reads coverage from the plans' `decisions`
    ("agents/gsd-plan-checker.md", 378, 379),
    ("gsd-core/workflows/plan-phase.md", 1401, 1401),
    // a task commit's message ends with its `Gsd-Task:` trailer, a second `-m` (the last paragraph)
    ("agents/gsd-executor.md", 534, 534),
    ("agents/gsd-executor.md", 556, 556),
    ("gsd-core/references/git-integration.md", 61, 61),
    ("gsd-core/references/git-integration.md", 74, 75),
    ("gsd-core/references/git-integration.md", 96, 96),
    ("gsd-core/references/git-integration.md", 105, 105),
    ("gsd-core/references/git-integration.md", 114, 114),
    ("gsd-core/references/tdd.md", 106, 106),
    ("gsd-core/references/tdd.md", 112, 112),
    ("gsd-core/references/tdd.md", 117, 117),
    ("gsd-core/references/tdd.md", 270, 270),
];
