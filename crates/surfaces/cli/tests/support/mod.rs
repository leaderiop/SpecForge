//! What the CLI's tests share: `specforge` as a user runs it ([`cli::Cli`]), the children and clients the harnesses
//! drive, the pairing of a command line with an MCP call ([`pairing`]), and the registry and written-file helpers. Nothing else in the suite spells the binary, `--path`,
//! `--format json` or the environment a run has (ADR 0078).

pub mod child;
pub mod cli;
pub mod lsp;
pub mod mcp_process;
pub mod pairing;
pub mod registry;
pub mod written;

pub use cli::{Cli, Run};
pub use specforge_project::testing::TestProject;

/// Software and testing, with governance for the `invariant` a project writes (ADR 0280).
pub const SOFTWARE_GOVERNANCE_TESTING: &[&str] = &[
    "@specforge/software",
    "@specforge/governance",
    "@specforge/testing",
];

/// `path` as a command line takes it.
pub fn at(path: &std::path::Path) -> String {
    path.to_str().unwrap().to_string()
}

/// The command naming the machine root `workspaces`; the directory follows.
pub const APPROVE_WORKSPACES: [&str; 5] = ["grants", "approve", "--root", "workspaces", "--at"];

/// `~/gsd-workspaces` approved as the root `workspaces` on `project`'s machine, and where it lies.
pub fn workspaces_root(project: &TestProject) -> std::path::PathBuf {
    Cli::on(project)
        .run(&[&APPROVE_WORKSPACES[..], &["~/gsd-workspaces"]].concat())
        .exits(0);
    let home = project.home().canonicalize().unwrap();
    home.join("gsd-workspaces")
}

/// Every file under `dir`, at any depth; none when it is missing.
pub fn files_under(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found
}
