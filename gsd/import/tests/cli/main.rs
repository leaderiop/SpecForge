//! Black-box tests: the `specforge` binary running `specforge gsd import` on the parity fixtures, before their twin
//! exists (ADR 0391). The harness is core's.

#[allow(
    dead_code,
    unused_imports,
    reason = "core's harness serves every extension; each test crate uses part of it and of its re-exports"
)]
#[path = "../../../core/tests/cli/harness.rs"]
mod harness;

mod compare;
mod hostile;
mod import;
mod parity_import;

use std::collections::BTreeMap;
use std::path::Path;

use harness::Project;

/// The judged rules the harness knows: `@gsd/core`'s (`harness::Book`).
pub static BOOKS: &[harness::Book] = &[harness::Book::CORE];

/// Every file of `project`, by its `/`-separated path from the root, with its bytes: what a test compares before and
/// after a run. The host's own `.specforge/` (its write lock and caches) is left out.
pub fn tree(project: &Project) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("a project directory") {
            let path = entry.expect("a directory entry").path();
            if path == root.join(".specforge") {
                continue;
            } else if path.is_dir() {
                walk(root, &path, into);
            } else {
                let relative = path.strip_prefix(root).expect("under the root");
                let name: Vec<String> = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                into.insert(
                    name.join("/"),
                    std::fs::read(&path).unwrap_or_else(|e| format!("unread: {e}").into_bytes()),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(project.root(), project.root(), &mut files);
    files
}

/// `project` as a GSD project before its import, loading `@gsd/import` by name: the vendored builtin the binary
/// embeds (ADR 0391 D12). Every test builds its project here.
pub fn imported(project: Project) -> Project {
    project.without_spec().with_builtins(&["@gsd/import"])
}
