//! What `git diff` calls a spec file: one predicate over a repository-relative path. A spec file is a file under the
//! project's spec root; with the spec root at the project root, a `.spec` file under it; or the harness ledger at the
//! project root. A spec root outside the repository makes nothing a spec file.

use std::path::Path;

use specforge_effects::git::{DiffStatus, GitFileDiff};
use specforge_project::harness::LEDGER_FILE;

use super::{DiffFile, slashed};

/// `dir` from `top`, `/` separated, the empty path when `dir` is `top` or holds it (the repository is nested in the
/// project); none when `dir` lies apart from it.
fn from_top(top: &Path, dir: &Path) -> Option<String> {
    match dir.strip_prefix(top) {
        Ok(rel) => Some(slashed(rel)),
        Err(_) => top.starts_with(dir).then(String::new),
    }
}

/// `path` is `dir` or lies under it; the empty `dir` holds every path.
fn under(dir: &str, path: &str) -> bool {
    dir.is_empty()
        || path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// `path` is `dir` or lies under it: what a filter over paths leaves out.
pub(super) fn at_or_under(dir: &str, path: &str) -> bool {
    dir == path || under(dir, path)
}

/// The spec files of one repository, for one project.
pub(crate) struct SpecPaths {
    /// The spec root from the repository's top; none when it lies outside.
    root: Option<String>,
    /// The spec root is the project root: only `.spec` files under it count.
    named_only: bool,
    /// The harness ledger from the repository's top; none when the project root is outside it.
    ledger: Option<String>,
}

impl SpecPaths {
    /// The spec files of the project at `project` with its spec root at `spec_root`, in the repository at `top`. All
    /// three land where the disk says, links resolved.
    pub(crate) fn of(top: &Path, project: &Path, spec_root: &Path) -> Self {
        SpecPaths {
            root: from_top(top, spec_root),
            named_only: spec_root == project,
            ledger: project
                .strip_prefix(top)
                .ok()
                .map(|rel| slashed(&rel.join(LEDGER_FILE))),
        }
    }

    /// The spec root as a pathspec from the repository's top, what [`Self::holds`] reads beside the ledger, none when
    /// it lies outside.
    pub(super) fn pathspec(&self) -> Option<String> {
        let root = self.root.as_deref()?;
        Some(match (self.named_only, root.is_empty()) {
            (true, true) => ":(glob)**/*.spec".to_string(),
            (true, false) => format!(":(glob){root}/**/*.spec"),
            (false, true) => ":(glob)**".to_string(),
            (false, false) => format!("{root}/"),
        })
    }

    /// Whether the repository-relative `path` is a spec file.
    pub(super) fn holds(&self, path: &str) -> bool {
        self.ledger.as_deref() == Some(path)
            || self.root.as_deref().is_some_and(|root| {
                under(root, path) && (!self.named_only || path.ends_with(".spec"))
            })
    }

    /// `file` with what it is to the project.
    pub(super) fn flagged(&self, file: GitFileDiff) -> DiffFile {
        let spec = self.holds(&file.path);
        let old = file.old_path.as_deref().is_some_and(|old| self.holds(old));
        let spec_removed = match file.status {
            DiffStatus::Deleted => spec,
            DiffStatus::Renamed => old,
            _ => false,
        };
        DiffFile {
            spec,
            spec_only: spec && (file.old_path.is_none() || old),
            spec_removed,
            file,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(top: &str, project: &str, spec: &str) -> SpecPaths {
        SpecPaths::of(Path::new(top), Path::new(project), Path::new(spec))
    }

    #[test]
    fn the_pathspec_says_what_holds_says() {
        for (top, project, spec, written) in [
            ("/r", "/r", "/r/spec", Some("spec/")),
            ("/r", "/r/sub", "/r/sub/spec", Some("sub/spec/")),
            ("/r", "/r", "/r", Some(":(glob)**/*.spec")),
            ("/r", "/r/sub", "/r/sub", Some(":(glob)sub/**/*.spec")),
            ("/r/in", "/r", "/r", Some(":(glob)**/*.spec")),
            ("/r/in", "/r", "/r/spec", None),
        ] {
            let got = paths(top, project, spec).pathspec();
            assert_eq!(got.as_deref(), written, "{top} {project} {spec}");
        }
    }

    #[test]
    fn a_spec_root_dir_holds_every_file_under_it() {
        let specs = paths("/r", "/r", "/r/spec");
        assert!(specs.holds("spec/a.spec"));
        assert!(specs.holds("spec/notes/readme.md"));
        assert!(!specs.holds("spec.txt"));
        assert!(!specs.holds("specs/a.spec"));
        assert!(
            !specs.holds("src/a.spec"),
            "a `.spec` outside the root is code"
        );
    }

    #[test]
    fn a_spec_root_at_the_project_root_holds_its_spec_files() {
        let specs = paths("/r", "/r", "/r");
        assert!(specs.holds("a.spec"));
        assert!(specs.holds("deep/b.spec"));
        assert!(!specs.holds("src/main.rs"));
        assert!(!specs.holds("a.spec.txt"));
    }

    #[test]
    fn the_harness_ledger_is_a_spec_file() {
        assert!(paths("/r", "/r", "/r/spec").holds("specforge-harness"));
        assert!(!paths("/r", "/r", "/r/spec").holds("sub/specforge-harness"));
        assert!(paths("/r", "/r/app", "/r/app/spec").holds("app/specforge-harness"));
    }

    #[test]
    fn a_project_nested_in_its_repository_is_read_from_its_top() {
        let specs = paths("/r", "/r/app", "/r/app/spec");
        assert!(specs.holds("app/spec/a.spec"));
        assert!(!specs.holds("spec/a.spec"));
        let specs = paths("/r", "/r/app", "/r/app");
        assert!(specs.holds("app/a.spec"));
        assert!(!specs.holds("other/a.spec"), "outside the project");
    }

    #[test]
    fn a_spec_root_outside_the_repository_makes_nothing_a_spec_file() {
        let specs = paths("/r", "/r", "/elsewhere/spec");
        assert!(!specs.holds("spec/a.spec"));
        assert!(!specs.holds("a.spec"));
        assert!(
            specs.holds("specforge-harness"),
            "the ledger stays the project's"
        );
    }

    #[test]
    fn a_repository_nested_in_the_project_is_held_whole_by_a_root_that_holds_it() {
        // The repository lies under the spec root: every file in it is under the root.
        assert!(paths("/p/spec/r", "/p", "/p/spec").holds("anything.md"));
        // With the spec root at the project root, only its `.spec` files.
        let specs = paths("/p/sub", "/p", "/p");
        assert!(specs.holds("a.spec"));
        assert!(!specs.holds("a.rs"));
        assert!(
            !specs.holds("specforge-harness"),
            "the project's ledger is above this repository"
        );
    }

    fn file(path: &str, old: Option<&str>, status: DiffStatus) -> GitFileDiff {
        GitFileDiff {
            path: path.to_string(),
            old_path: old.map(str::to_string),
            status,
            added: 0,
            removed: 0,
            binary: false,
            truncated: false,
            patch: None,
            hunks: None,
        }
    }

    #[test]
    fn each_flag_answers_its_own_question() {
        let specs = paths("/r", "/r", "/r/spec");
        let flags = |f: GitFileDiff| {
            let d = specs.flagged(f);
            (d.spec, d.spec_only, d.spec_removed)
        };
        use DiffStatus::*;
        assert_eq!(flags(file("spec/a", None, Added)), (true, true, false));
        assert_eq!(flags(file("spec/a", None, Modified)), (true, true, false));
        assert_eq!(flags(file("spec/a", None, Deleted)), (true, true, true));
        assert_eq!(flags(file("src/a", None, Modified)), (false, false, false));
        assert_eq!(flags(file("src/a", None, Deleted)), (false, false, false));
        // renamed within, out of and into the spec root
        assert_eq!(
            flags(file("spec/b", Some("spec/a"), Renamed)),
            (true, true, true)
        );
        assert_eq!(
            flags(file("src/a", Some("spec/a"), Renamed)),
            (false, false, true)
        );
        assert_eq!(
            flags(file("spec/a", Some("src/a"), Renamed)),
            (true, false, false)
        );
        // a copy leaves its source
        assert_eq!(
            flags(file("spec/b", Some("spec/a"), Copied)),
            (true, true, false)
        );
        assert_eq!(
            flags(file("src/b", Some("spec/a"), Copied)),
            (false, false, false)
        );
    }
}
