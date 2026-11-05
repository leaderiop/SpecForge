//! The repository's check list: every check CI, `cargo gate` and the pre-commit hook run, each written once
//! with the run it joins and the paths that trigger it. `cargo gate list` prints it.

mod baseline;
pub mod blob_inputs;
mod blob_size;
mod change_set;
mod checks;
pub mod crate_map;
mod doc_links;
mod gsd;
pub mod judge;
pub mod layers;
pub mod objects;
mod repo;
mod root_layout;
mod run;
mod size_cap;
mod steps;

pub use change_set::{ChangeSet, Pattern};
pub use checks::{CHECKS, Check, Reach, Selection, Trigger, render_list};
pub use repo::{
    Dependency, DependencyKind, EXTENSION_CRATES, Member, PROJECTS, Project, Repo,
    outside_workspace_patterns,
};
pub use run::{CARGO_AUDIT, Ctx, Mode, Outcome, Report, TREE_SITTER, Tool, baseline, run};
