//! Lines (`(path, first, last)`, inclusive, 0-based body lines) rewritten by ADR 0343 D7–D9 although upstream's names
//! no tool the port rule sees: display text and value lists that read as runs, upstream verbs the port has no command
//! for, routing that probed PATH, a person's chores, and steps that read a key or a file nothing here writes.
//! `port_workflows` reads them as more `REWRITTEN_SPANS` rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    // the sub-repos are `specforge git repos`' children; a sub-repository is a SpecForge project of its own
    ("gsd-core/workflows/new-project.md", 653, 653),
    ("gsd-core/workflows/new-project.md", 656, 657),
    ("gsd-core/workflows/new-project.md", 669, 669),
    ("gsd-core/workflows/new-project.md", 671, 673),
    ("gsd-core/workflows/new-project.md", 675, 675),
];
