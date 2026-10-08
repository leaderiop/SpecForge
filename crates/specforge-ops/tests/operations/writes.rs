//! What each writing operation reports it wrote ([`Writes`]) is what it
//! changed on disk: a snapshot diff of the project, file by file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use specforge_ops::Writes;
use specforge_ops::extension::{self, AddOutcome, AddRequest, RemoveRequest, Source, Trust};
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use tempfile::TempDir;

type Snapshot = BTreeMap<PathBuf, Vec<u8>>;

/// Every file under `root` with its bytes; empty when `root` is absent.
fn files_under(root: &Path) -> Snapshot {
    let mut files = Snapshot::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.map(|entry| entry.unwrap().path()) {
            if path.is_dir() {
                dirs.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                files.insert(path, bytes);
            }
        }
    }
    files
}

/// The files whose bytes differ between `before` and now, added and
/// removed ones included, relative to `root`, sorted.
fn changed_since(root: &Path, before: &Snapshot) -> Vec<String> {
    let after = files_under(root);
    let mut changed: Vec<String> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .map(|path| path.strip_prefix(root).unwrap().display().to_string())
        .collect();
    changed.sort();
    changed.dedup();
    changed
}

/// `writes` as the root-relative names a surface lists.
fn listed(writes: &Writes, root: &Path) -> Vec<String> {
    writes.names_under(root)
}

/// A project directory whose `specforge.json` enables `extensions`, with
/// `files` written (path, text).
fn project(extensions: &[&str], files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({"name": "w", "version": "0.1.0", "extensions": extensions});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

/// The extension blob the build vendors: `@sdk/greet`, declaring the kind
/// `greeting`.
fn greet_blob() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/greet-extension/greet.wasm")
}

const GREET: &str = "@sdk/greet";
const MODULE: &str = ".specforge/extensions/@sdk/greet/extension.wasm";

fn add(root: &Path, source: Source) -> Result<extension::Added, specforge_ops::OpError> {
    extension::add(
        &AddRequest {
            root,
            source,
            allow_unsigned: false,
            trust: Trust::Refuse,
            dry_run: false,
        },
        &specforge_ops::registry::Unconfigured("test"),
    )
}

/// The project at `root`, compiled with its own component runtime.
fn compiled(root: &Path) -> CompiledProject {
    let runtime = specforge_component::project_runtime(root);
    CompiledProject::compile(root, Some(&runtime))
}

fn remove(root: &Path, name: &str, dry_run: bool) -> extension::RemoveOutcome {
    let project = compiled(root);
    let request = RemoveRequest {
        name,
        force: false,
        dry_run,
    };
    extension::remove(&ProjectView::of(&project), &request).unwrap()
}

const MESSY: &str = "behavior messy \"Messy\" {\ncontract \"The system MUST work\"\n}\n";
const TIDY: &str = "behavior tidy \"Tidy\" {\n  contract \"The system MUST work\"\n}\n";

#[cfg(unix)]
#[test]
fn format_writes_are_the_files_it_rewrote() {
    use specforge_ops::format::{self, Mode, Request};
    use std::os::unix::fs::PermissionsExt;
    let dir = project(
        &[],
        &[
            ("a.spec", MESSY),
            ("b.spec", &MESSY.replace("messy", "other")),
            ("c.spec", TIDY),
        ],
    );
    let root = dir.path();
    let locked = root.join("b.spec");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444)).unwrap();
    let before = files_under(root);

    let outcome = format::run(&Request {
        root,
        paths: &[],
        mode: Mode::Write,
    });
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

    // c.spec is already formatted; b.spec could not be written.
    assert_eq!(outcome.failures.len(), 1, "{:?}", outcome.failures);
    assert_eq!(listed(&outcome.writes(), root), ["a.spec"]);
    assert_eq!(changed_since(root, &before), ["a.spec"]);

    // A check writes nothing.
    let outcome = format::run(&Request {
        root,
        paths: &[],
        mode: Mode::Check,
    });
    assert!(outcome.writes().is_empty());
}

#[test]
fn rename_writes_are_the_files_it_edited() {
    use specforge_ops::navigate::Navigator;
    use specforge_ops::rename;
    let limit = "invariant session_limit \"Limit\" {\n  guarantee \"x\"\n}\n";
    let login = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";
    let other = "behavior other \"Other\" {\n}\n";
    let dir = project(
        &["@specforge/software"],
        &[
            ("limit.spec", limit),
            ("login.spec", login),
            ("other.spec", other),
        ],
    );
    let root = dir.path();
    let project = compiled(root);
    let read = |f: &str| std::fs::read_to_string(root.join(f)).ok();
    let plan = rename::plan(
        &Navigator::new(ProjectView::of(&project), read),
        "session_limit",
        "session_cap",
    )
    .unwrap();
    let before = files_under(root);

    let writes = rename::apply(&plan, root).unwrap();

    assert_eq!(listed(&writes, root), ["limit.spec", "login.spec"]);
    assert_eq!(changed_since(root, &before), ["limit.spec", "login.spec"]);
}

/// What `init` scaffolds in `dir` with `extensions`.
fn init(dir: &Path, extensions: &[String]) -> specforge_ops::init::Outcome {
    use specforge_ops::init;
    let request = init::Request {
        dir,
        name: Some("demo"),
        version: init::DEFAULT_VERSION,
        extensions,
        forbid_inside: None,
    };
    let plan = init::plan(&request).unwrap();
    init::apply(dir, &plan).unwrap()
}

#[test]
fn init_writes_include_gitignore_only_when_it_changed() {
    let scratch = TempDir::new().unwrap();

    let fresh = scratch.path().join("fresh");
    let outcome = init(&fresh, &[]);
    assert_eq!(
        listed(&outcome.writes, &fresh),
        [".gitignore", "spec/hello.spec", "specforge.json"]
    );
    assert_eq!(
        changed_since(&fresh, &Snapshot::new()),
        listed(&outcome.writes, &fresh)
    );

    // A .gitignore holding every entry is left as it is.
    let beside = scratch.path().join("beside");
    std::fs::create_dir_all(&beside).unwrap();
    std::fs::write(
        beside.join(".gitignore"),
        "specforge-infer.json\nspecforge-report.json\n.specforge/\n",
    )
    .unwrap();
    let before = files_under(&beside);
    let outcome = init(&beside, &[]);
    assert_eq!(
        listed(&outcome.writes, &beside),
        ["spec/hello.spec", "specforge.json"]
    );
    assert_eq!(
        changed_since(&beside, &before),
        listed(&outcome.writes, &beside)
    );
}

#[specforge_test_macros::test(
    behavior = "scaffold_starter_spec_file",
    verify = "the starter spec's version is the project's"
)]
fn the_starter_states_the_project_version() {
    use specforge_ops::init;
    let scratch = TempDir::new().unwrap();
    let starter_of = |version: &str| {
        let request = init::Request {
            dir: &scratch.path().join(version),
            name: Some("demo"),
            version,
            extensions: &[],
            forbid_inside: None,
        };
        let plan = init::plan(&request).unwrap();
        assert_eq!(plan.config["version"], version);
        plan.starter
    };

    assert!(starter_of("2.3.0").contains("version \"2.3.0\""));
    assert!(starter_of(init::DEFAULT_VERSION).contains("version \"0.1.0\""));
}

#[test]
fn init_with_a_local_extension_writes_its_module_and_lock() {
    let scratch = TempDir::new().unwrap();
    let dir = scratch.path().join("with-greet");

    let outcome = init(&dir, &[greet_blob().display().to_string()]);

    assert_eq!(
        listed(&outcome.writes, &dir),
        [
            ".gitignore",
            MODULE,
            "spec/hello.spec",
            "specforge.json",
            "specforge.lock"
        ]
    );
    assert_eq!(
        changed_since(&dir, &Snapshot::new()),
        listed(&outcome.writes, &dir)
    );
}

#[test]
fn enabling_a_builtin_writes_the_config_only() {
    let dir = project(&[], &[]);
    let root = dir.path();
    let before = files_under(root);

    let added = add(root, Source::Builtin("@specforge/product")).unwrap();

    assert!(matches!(
        added.outcome,
        AddOutcome::Builtin { changed: true, .. }
    ));
    assert_eq!(listed(&added.writes, root), ["specforge.json"]);
    assert_eq!(changed_since(root, &before), ["specforge.json"]);
    assert_eq!(
        added.extensions_enabled,
        specforge_common::load_project_config(root).extensions.len()
    );
    assert!(added.extensions_enabled >= 1);
}

#[test]
fn enabling_an_enabled_builtin_writes_nothing() {
    let dir = project(&["@specforge/product"], &[]);
    let root = dir.path();
    let before = files_under(root);

    let added = add(root, Source::Builtin("@specforge/product")).unwrap();

    assert!(matches!(
        added.outcome,
        AddOutcome::Builtin { changed: false, .. }
    ));
    assert!(added.writes.is_empty());
    assert_eq!(changed_since(root, &before), Vec::<String>::new());
    assert_eq!(added.extensions_enabled, 1);
}

#[test]
fn installing_a_local_extension_writes_module_lock_and_config() {
    let dir = project(&[], &[]);
    let root = dir.path();
    let before = files_under(root);

    let added = add(root, Source::Local(greet_blob())).unwrap();

    assert!(matches!(added.outcome, AddOutcome::Installed { .. }));
    let three = [MODULE, "specforge.json", "specforge.lock"];
    assert_eq!(listed(&added.writes, root), three);
    assert_eq!(changed_since(root, &before), three);
    assert_eq!(added.extensions_enabled, 1);

    // The same blob again: already present, nothing written.
    let before = files_under(root);
    let again = add(root, Source::Local(greet_blob())).unwrap();
    assert!(matches!(again.outcome, AddOutcome::AlreadyPresent { .. }));
    assert!(again.writes.is_empty());
    assert_eq!(changed_since(root, &before), Vec::<String>::new());
}

#[test]
fn an_add_that_fails_after_placing_its_module_reports_it() {
    let dir = project(&[], &[]);
    let root = dir.path();
    // The lock is written through a sibling file; a directory in its place
    // makes the lock write fail after the module is placed.
    std::fs::create_dir_all(root.join("specforge.lock.tmp")).unwrap();
    let before = files_under(root);

    let error = add(root, Source::Local(greet_blob())).unwrap_err();

    assert_eq!(error.code, "E033", "{error:?}");
    assert_eq!(listed(&error.writes, root), [MODULE]);
    assert_eq!(changed_since(root, &before), [MODULE]);
}

#[test]
fn removing_an_install_writes_its_module_lock_and_config() {
    let dir = project(&[], &[]);
    let root = dir.path();
    add(root, Source::Local(greet_blob())).unwrap();
    let before = files_under(root);

    let outcome = remove(root, GREET, false);

    let three = [MODULE, "specforge.json", "specforge.lock"];
    assert_eq!(listed(&outcome.writes, root), three);
    assert_eq!(changed_since(root, &before), three);
}

#[test]
fn disabling_a_builtin_writes_the_config_only() {
    let dir = project(&["@specforge/product"], &[]);
    let root = dir.path();
    let before = files_under(root);

    let outcome = remove(root, "@specforge/product", false);

    assert_eq!(listed(&outcome.writes, root), ["specforge.json"]);
    assert_eq!(changed_since(root, &before), ["specforge.json"]);

    // A dry run writes nothing.
    let dir = project(&["@specforge/product"], &[]);
    let before = files_under(dir.path());
    let preview = remove(dir.path(), "@specforge/product", true);
    assert!(preview.writes.is_empty());
    assert_eq!(changed_since(dir.path(), &before), Vec::<String>::new());
}

#[test]
fn orphaned_lists_the_entity_ids() {
    let dir = project(
        &[],
        &[
            ("b.spec", "greeting zeta \"Zeta\" {\n  style warm\n}\n"),
            ("a.spec", "greeting hello \"Hello\" {\n  style warm\n}\n"),
        ],
    );
    let root = dir.path();
    add(root, Source::Local(greet_blob())).unwrap();

    let outcome = remove(root, GREET, true);

    assert_eq!(outcome.orphaned, ["hello", "zeta"]);
    assert_eq!(
        outcome.orphan_warnings.len(),
        2,
        "{:?}",
        outcome.orphan_warnings
    );
}

const OLD: &str = "// specforge-format: 0.9\nbehavior gamma \"Gamma\" {\n}\n";

fn migration(root: &Path, dry_run: bool) -> specforge_ops::migrate::Request<'_> {
    specforge_ops::migrate::Request {
        root,
        target: specforge_migrate::CURRENT_FORMAT_VERSION,
        dry_run,
        no_backup: false,
    }
}

#[test]
fn a_migration_writes_each_file_and_its_backup() {
    let dir = project(
        &[],
        &[
            ("old.spec", OLD),
            ("older.spec", &OLD.replace("gamma", "delta")),
            ("current.spec", TIDY),
        ],
    );
    let root = dir.path();
    let before = files_under(root);

    let preview = specforge_ops::migrate::run(&migration(root, true), None);
    assert!(preview.writes.is_empty());
    assert_eq!(changed_since(root, &before), Vec::<String>::new());

    let outcome = specforge_ops::migrate::run(&migration(root, false), None);

    let written = ["old.spec", "old.spec.bak", "older.spec", "older.spec.bak"];
    assert_eq!(listed(&outcome.writes, root), written);
    assert_eq!(changed_since(root, &before), written);
}

#[test]
fn a_rolled_back_migration_keeps_only_its_backups() {
    let dir = project(&[], &[("old.spec", OLD)]);
    let root = dir.path();
    let before = files_under(root);

    let outcome =
        specforge_ops::migrate::run_with_hooks(&migration(root, false), None, &mut |_, _| {
            (vec!["@t/x:hook".into()], vec!["the hook failed".into()])
        });

    assert!(outcome.rollback.is_some());
    assert_eq!(listed(&outcome.writes, root), ["old.spec.bak"]);
    assert_eq!(changed_since(root, &before), ["old.spec.bak"]);
}
