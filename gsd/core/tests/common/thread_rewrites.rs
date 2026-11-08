//! The body lines (0-based, inclusive runs) rewritten because a plan lists its decisions (`decisions`, ADR 0351) and
//! a task commit ends with its `Gsd-Task: {phase}-{plan}/{task}` trailer. Paths are under `assets/`; an agent's is
//! `agents/<file>`. `port` and `port_workflows` read them as more rewritten rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    // the planner lists the decisions a plan implements in its `decisions`; the frontmatter's computed `autonomous`
    // gives way to it
    ("agents/gsd-planner.md", 47, 47),
    ("agents/gsd-planner.md", 55, 56),
    ("agents/gsd-planner.md", 304, 304),
    // the plan checker reads coverage from the plans' `decisions`
    ("agents/gsd-plan-checker.md", 378, 379),
    ("gsd-core/workflows/plan-phase.md", 1401, 1401),
];
