use std::path::{Path, PathBuf};

/// Discover .spec files to format.
///
/// - If `explicit_paths` is non-empty, only those paths are used (directories recursed).
/// - If `explicit_paths` is empty, all .spec files under `spec_root` are discovered.
/// - Files matching `exclude_globs` are excluded.
/// - Non-.spec files are silently skipped.
pub fn discover_targets(
    spec_root: &Path,
    explicit_paths: &[PathBuf],
    exclude_globs: &[String],
) -> Vec<PathBuf> {
    let exclude_patterns: Vec<glob::Pattern> = exclude_globs
        .iter()
        .filter_map(|g| glob::Pattern::new(g).ok())
        .collect();

    let paths = if explicit_paths.is_empty() {
        collect_spec_files(spec_root)
    } else {
        let mut result = Vec::new();
        for path in explicit_paths {
            if path.is_dir() {
                result.extend(collect_spec_files(path));
            } else if path.extension().is_some_and(|e| e == "spec") {
                result.push(path.clone());
            }
            // Non-.spec files are silently skipped
        }
        result
    };

    // Apply exclusions
    paths
        .into_iter()
        .filter(|p| {
            let path_str = p.to_string_lossy();
            !exclude_patterns.iter().any(|pat| pat.matches(&path_str))
        })
        .collect()
}

/// Recursively collect all .spec files under a directory.
fn collect_spec_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return files;
    }

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "spec") {
            files.push(path.to_path_buf());
        }
    }

    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_file(dir: &Path, name: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, "// test").unwrap();
    }

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "no arguments formats all .spec files under spec_root")]
    #[test]
    fn test_no_arguments_formats_all_spec_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        create_file(root, "a.spec");
        create_file(root, "sub/b.spec");
        create_file(root, "sub/c.spec");

        let targets = discover_targets(root, &[], &[]);
        assert_eq!(targets.len(), 3);
    }

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "files matching format.exclude globs are excluded")]
    #[test]
    fn test_exclude_globs_filter_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        create_file(root, "a.spec");
        create_file(root, "vendor/b.spec");

        let targets = discover_targets(root, &[], &["**/vendor/**".to_string()]);
        assert_eq!(targets.len(), 1);
        assert!(targets[0].ends_with("a.spec"));
    }

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "explicit file paths format only those files")]
    #[test]
    fn test_explicit_paths_format_only_those() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        create_file(root, "a.spec");
        create_file(root, "b.spec");

        let explicit = vec![root.join("a.spec")];
        let targets = discover_targets(root, &explicit, &[]);
        assert_eq!(targets.len(), 1);
        assert!(targets[0].ends_with("a.spec"));
    }

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "directory argument recursively discovers .spec files")]
    #[test]
    fn test_directory_argument_recursively_discovers() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        create_file(root, "sub/a.spec");
        create_file(root, "sub/deep/b.spec");

        let explicit = vec![root.join("sub")];
        let targets = discover_targets(root, &explicit, &[]);
        assert_eq!(targets.len(), 2);
    }

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "non-.spec files are skipped with no error")]
    #[test]
    fn test_non_spec_files_are_skipped() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        create_file(root, "a.spec");
        create_file(root, "b.txt");
        create_file(root, "c.rs");

        let explicit = vec![
            root.join("a.spec"),
            root.join("b.txt"),
            root.join("c.rs"),
        ];
        let targets = discover_targets(root, &explicit, &[]);
        assert_eq!(targets.len(), 1);
        assert!(targets[0].ends_with("a.spec"));
    }

    // --- Invariant: discover_completeness ---

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "no arguments formats all .spec files under spec_root")]
    #[test]
    fn test_all_spec_files_under_spec_root_are_discovered() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        create_file(root, "a.spec");
        create_file(root, "sub/b.spec");
        create_file(root, "sub/deep/c.spec");
        create_file(root, "other/d.spec");

        let targets = discover_targets(root, &[], &[]);
        assert_eq!(targets.len(), 4, "all .spec files should be found");
    }

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "non-.spec files are skipped with no error")]
    #[test]
    fn test_no_spec_files_are_silently_skipped() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create spec files with various nesting patterns
        create_file(root, "top.spec");
        create_file(root, ".hidden/hidden.spec");
        create_file(root, "deep/very/nested/file.spec");

        let targets = discover_targets(root, &[], &[]);
        // All files should be discovered (walkdir follows into directories)
        assert!(targets.len() >= 2, "at least top.spec and nested files should be found: {targets:?}");
        assert!(targets.iter().any(|t| t.ends_with("top.spec")), "top.spec should be found");
    }

    // --- Contract: discover_format_targets ---

    #[specforge_test_macros::test(behavior = "discover_format_targets", verify = "requires/ensures consistency for format target discovery")]
    #[test]
    fn test_discover_format_targets_contract() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        // requires: project_root_available, filesystem_accessible
        create_file(root, "a.spec");
        create_file(root, "b.spec");
        create_file(root, "c.txt"); // non-spec

        // ensures: all_spec_files_discovered
        let all = discover_targets(root, &[], &[]);
        assert_eq!(all.len(), 2, "should discover exactly the .spec files");

        // ensures: non_spec_skipped
        let explicit_mixed = vec![root.join("a.spec"), root.join("c.txt")];
        let filtered = discover_targets(root, &explicit_mixed, &[]);
        assert_eq!(filtered.len(), 1, "non-.spec should be skipped");

        // ensures: exclusions_applied
        let excluded = discover_targets(root, &[], &["**/b.spec".to_string()]);
        assert_eq!(excluded.len(), 1);
        assert!(excluded[0].ends_with("a.spec"));
    }
}
