//! The root of this SpecForge checkout: the nearest directory above this crate whose
//! `Cargo.toml` declares the workspace.
//!
//! Tests and build tools read files outside their own crate (`spec/`, `examples/`, `docs/`,
//! `schema/`, the committed blobs under `extensions/` and `fixtures/`, another crate's
//! fixtures) by a path from this root. The root is found by walking up from this crate's own
//! directory, so no caller depends on how deep it or any other crate sits under `crates/`.
//! Nothing a user runs reads it: the shipped binaries never depend on this crate.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The variable a run of test binaries in a copy of a checkout sets to the checkout the copy was made from (the gate's
/// `self-spec-sample` runs cited tests in one). Cargo found every unit fresh against the copy's sources, which are that
/// checkout's own, so the binaries built under it are no stale target there, and they read that checkout's files.
pub const COPY_OF_ENV: &str = "SPECFORGE_CHECKOUT_COPY_OF";

/// The variable that asks a test to rewrite its goldens instead of comparing with them.
pub const BLESS_ENV: &str = "SPECFORGE_BLESS";

/// Whether this test rewrites its goldens: [`BLESS_ENV`] is set and not empty. Every test that writes a golden asks
/// here, so none writes one from a run in a copy of the checkout.
///
/// # Panics
///
/// When [`BLESS_ENV`] is set while [`COPY_OF_ENV`] is: a test in a copy names its goldens by the paths it was built
/// under, which are the checkout's, so a bless there would rewrite the checkout from a copy's run.
pub fn bless() -> bool {
    blessing(
        std::env::var_os(BLESS_ENV).as_deref(),
        std::env::var_os(COPY_OF_ENV).as_deref(),
    )
    .unwrap_or_else(|refused| panic!("{refused}"))
}

/// [`bless`] from the two variables' values: Err when a bless is asked in a declared copy.
fn blessing(
    bless: Option<&std::ffi::OsStr>,
    copy_of: Option<&std::ffi::OsStr>,
) -> Result<bool, String> {
    let asked = bless.is_some_and(|value| !value.is_empty());
    match copy_of {
        Some(checkout) if asked => Err(format!(
            "{BLESS_ENV} is set in a copy of {} ({COPY_OF_ENV}): this test's goldens are that checkout's, so it \
             refuses to write them; bless in the checkout itself",
            Path::new(checkout).display()
        )),
        _ => Ok(asked),
    }
}

/// The checkout's root, canonical (absolute, no `..`, symlinks resolved).
///
/// # Panics
///
/// When no directory above this crate has a `Cargo.toml` with a `[workspace]` table: the crate
/// was built outside a SpecForge checkout.
pub fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = workspace_above(here).unwrap_or_else(|| {
            panic!(
                "no Cargo.toml with a [workspace] table above {}: specforge-repo-root reads a \
                 SpecForge checkout and must be built inside one",
                here.display()
            )
        });
        let root = root
            .canonicalize()
            .unwrap_or_else(|e| panic!("{}: {e}", root.display()));
        if let Some(running) = std::env::var_os("CARGO_MANIFEST_DIR") {
            let copy_of = std::env::var_os(COPY_OF_ENV).map(PathBuf::from);
            assert!(
                !runs_in_another_checkout(&root, Path::new(&running), copy_of.as_deref()),
                "specforge-repo-root was built under {} but runs for {}: the shared target \
                 directory is stale, touch crates/leaf/repo-root/src/lib.rs and rebuild",
                root.display(),
                running.to_string_lossy(),
            );
        }
        root
    })
}

/// Whether `running` (the manifest directory cargo sets for the process that runs now) lies
/// outside `baked`, the root this crate was compiled under, and is no declared copy of it. A nested
/// workspace such as `integrations/rust` is inside it.
///
/// A copy is declared by `copy_of` (from [`COPY_OF_ENV`]) naming `baked`, and is honoured only when it
/// is one: `baked` is a checkout (its workspace manifest and its `.git`), and the workspace `running`
/// belongs to is another directory with no `.git` of its own (a copy of the tracked files). Another
/// checkout naming this one is still another checkout, with its own stale target.
fn runs_in_another_checkout(baked: &Path, running: &Path, copy_of: Option<&Path>) -> bool {
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let running = canonical(running);
    if running.starts_with(baked) {
        return false;
    }
    let checkout = |root: &Path| root.join(".git").exists();
    let declared_copy = copy_of.map(canonical).as_deref() == Some(baked)
        && checkout(baked)
        && declares_workspace(&baked.join("Cargo.toml"))
        && running
            .ancestors()
            .find(|dir| declares_workspace(&dir.join("Cargo.toml")))
            .is_some_and(|copy| copy != baked && !checkout(copy));
    !declared_copy
}

/// The nearest ancestor of `dir`, `dir` excluded, whose `Cargo.toml` has a `[workspace]` table.
fn workspace_above(dir: &Path) -> Option<&Path> {
    dir.ancestors()
        .skip(1)
        .find(|ancestor| declares_workspace(&ancestor.join("Cargo.toml")))
}

/// Whether the manifest has a line that is exactly `[workspace]` (a `[workspace.*]` table alone
/// is not one).
fn declares_workspace(manifest: &Path) -> bool {
    std::fs::read_to_string(manifest)
        .is_ok_and(|text| text.lines().any(|line| line.trim_end() == "[workspace]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_holds_the_workspace_manifest_and_the_crates() {
        let root = repo_root();
        assert!(
            declares_workspace(&root.join("Cargo.toml")),
            "{}",
            root.display()
        );
        assert!(root.join("crates").is_dir(), "{}", root.display());
        assert!(root.is_absolute());
        assert_eq!(root, root.canonicalize().unwrap());
    }

    #[test]
    fn a_target_built_under_another_checkout_is_stale() {
        let here = repo_root();
        let other = tempfile::TempDir::new().unwrap();
        std::fs::write(other.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        let member = other.path().join("crates/x");
        std::fs::create_dir_all(&member).unwrap();
        assert!(runs_in_another_checkout(here, &member, None));
        assert!(!runs_in_another_checkout(
            here,
            &here.join("crates/leaf/repo-root"),
            None
        ));
        assert!(!runs_in_another_checkout(here, here, None));
    }

    #[test]
    fn a_run_in_a_declared_copy_of_the_checkout_is_not_stale() {
        let here = repo_root();
        let copy = tempfile::TempDir::new().unwrap();
        std::fs::write(copy.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        let member = copy.path().join("crates/x");
        std::fs::create_dir_all(&member).unwrap();
        assert!(!runs_in_another_checkout(here, &member, Some(here)));
        // A copy of another checkout is still another checkout.
        assert!(runs_in_another_checkout(here, &member, Some(copy.path())));
    }

    #[test]
    fn another_checkout_naming_this_one_as_its_original_is_still_stale() {
        let here = repo_root();
        let other = tempfile::TempDir::new().unwrap();
        std::fs::write(other.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::create_dir(other.path().join(".git")).unwrap();
        let member = other.path().join("crates/x");
        std::fs::create_dir_all(&member).unwrap();
        assert!(
            runs_in_another_checkout(here, &member, Some(here)),
            "a checkout of its own is no copy"
        );
        // Nor is a run that names, as its original, a tree that is no checkout.
        let tree = tempfile::TempDir::new().unwrap();
        std::fs::write(tree.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        let baked = tree.path().canonicalize().unwrap();
        std::fs::remove_dir(other.path().join(".git")).unwrap();
        assert!(runs_in_another_checkout(&baked, &member, Some(&baked)));
        // And a member of no workspace at all.
        let loose = tempfile::TempDir::new().unwrap();
        assert!(runs_in_another_checkout(here, loose.path(), Some(here)));
    }

    #[test]
    fn a_bless_is_asked_by_a_value_and_refused_in_a_copy() {
        let os = |text: &'static str| Some(std::ffi::OsStr::new(text));
        assert_eq!(blessing(None, None), Ok(false));
        assert_eq!(blessing(os(""), None), Ok(false));
        assert_eq!(blessing(os("1"), None), Ok(true));
        assert_eq!(blessing(None, os("/checkout")), Ok(false));
        let refused = blessing(os("1"), os("/checkout")).unwrap_err();
        assert!(
            refused.contains(BLESS_ENV) && refused.contains("/checkout"),
            "{refused}"
        );
    }

    #[test]
    fn a_crate_at_any_depth_finds_the_same_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        for crate_dir in ["crates/flat", "crates/layer/nested", "crates/a/b/deeper"] {
            let crate_dir = root.join(crate_dir);
            std::fs::create_dir_all(&crate_dir).unwrap();
            std::fs::write(crate_dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
            assert_eq!(
                workspace_above(&crate_dir),
                Some(root),
                "{}",
                crate_dir.display()
            );
        }
    }

    #[test]
    fn only_a_workspace_table_marks_the_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        // A manifest that inherits from the workspace is not one.
        let layer = root.join("crates/layer");
        std::fs::create_dir_all(layer.join("x")).unwrap();
        std::fs::write(
            layer.join("Cargo.toml"),
            "[workspace.package]\nedition = \"2024\"\n",
        )
        .unwrap();
        assert_eq!(workspace_above(&layer.join("x")), Some(root));
    }
}
