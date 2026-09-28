//! Shared `.spec` discovery policy (C14-16): one walker, one skip-list, one
//! symlink rule for every surface that scans a workspace (resolver, LSP,
//! formatter, CLI). Divergent per-crate walkers made the same job run at
//! three speeds with three results.

use std::path::{Path, PathBuf};

/// Directories never traversed during `.spec` discovery: build artifacts,
/// dependency trees, and VCS internals.
pub const SKIP_DIRS: &[&str] = &["target", "node_modules", ".git", ".hg", "dist", "build"];

/// Discover `.spec` files under `root` with the shared policy: skip
/// [`SKIP_DIRS`], never follow symlinks, and drop paths matching any
/// `exclude` pattern. Patterns are matched as substrings against the
/// workspace-relative path (project `specforge.json` `exclude` entries,
/// C4-04). Returned paths are sorted for deterministic downstream behavior.
pub fn discover_spec_files(root: &Path, exclude: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false) // symlink cycles and out-of-tree traversal are never wanted here
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir()
                && let Some(name) = e.file_name().to_str()
            {
                return !SKIP_DIRS.contains(&name);
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "spec") {
            let relative = path.strip_prefix(root).unwrap_or(path);
            let rel_str = relative.to_string_lossy();
            if exclude
                .iter()
                .any(|pattern| rel_str.contains(pattern.as_str()))
            {
                continue;
            }
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn skips_build_dirs_and_filters_by_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.spec"), "").unwrap();
        fs::write(root.join("src/notes.txt"), "").unwrap();
        fs::write(root.join("target/generated.spec"), "").unwrap();

        let found = discover_spec_files(root, &[]);
        let names: Vec<String> = found
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["src/main.spec".to_string()]);
    }

    #[test]
    fn exclude_patterns_drop_matching_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("specs/legacy")).unwrap();
        fs::write(root.join("specs/current.spec"), "").unwrap();
        fs::write(root.join("specs/legacy/old.spec"), "").unwrap();

        let found = discover_spec_files(root, &["legacy".to_string()]);
        let names: Vec<String> = found
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["specs/current.spec".to_string()]);
    }
}
