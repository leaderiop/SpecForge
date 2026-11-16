//! One command that leaves the tree consistent after a change to what the
//! committed blobs compile from (their sources, the SDK and its macros,
//! protocol-types, coverage, any `inputs.json` input). See docs/agents/revendor.md.
//!
//!   cargo run -p xtask --bin revendor
//!   cargo run -p xtask --bin revendor -- --vocabulary
//!
//! With no argument, in order: refuse a place it cannot work from; refresh the lockfile of every
//! committed blob's crate (`specforge_component::committed::COMMITTED`); rebuild
//! and vendor every blob with its `inputs.json` (`build-blobs --install`);
//! rewrite the declaration snapshots of every SDK blob, in process; write the builtins'
//! candidate vocabulary from those snapshots (ADR 0122, ADR 0369); check what it
//! did not write (`build-blobs --check` and the declarations test) and print
//! the paths to stage.
//!
//! With `--vocabulary`: only write the builtins' candidate vocabulary from the snapshots as
//! they are committed. It builds nothing, so it works from any checkout. For a change to the
//! vocabulary's rule or written form that changes no blob input.
#![expect(
    clippy::print_stdout,
    reason = "a developer tool: its report is its stdout"
)]

use specforge_component::ComponentRuntime;
use specforge_component::committed::{
    COMMITTED, pinned_builtin_declarations, snapshots, wire_answers,
};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::CandidateVocabulary;
use specforge_wasm::runtime::WasmRuntime;
use std::path::Path;
use std::process::Command;

/// The directory of the generated vocabulary, from the repository root: what a run stages.
const VOCABULARY_DIR: &str = "crates/runtime/component/data";
/// The generated vocabulary's file name in [`VOCABULARY_DIR`].
const VOCABULARY_FILE: &str = "builtin-vocabulary.json";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let vocabulary_only = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => false,
        ["--vocabulary"] => true,
        other => {
            eprintln!("usage: revendor [--vocabulary] (got {other:?})");
            std::process::exit(2);
        }
    };
    let root = specforge_repo_root::repo_root();
    let done = if vocabulary_only {
        run_vocabulary(root)
    } else {
        let env_target = std::env::var_os("CARGO_TARGET_DIR").is_some();
        preflight(root, env_target).and_then(|()| run(root))
    };
    if let Err(message) = done {
        eprintln!("revendor: {message}");
        std::process::exit(1);
    }
}

/// The `--vocabulary` run: the vocabulary step alone, from the pinned declarations as they are
/// committed. No lockfile, blob, snapshot or check, and no `preflight`, since it builds nothing.
fn run_vocabulary(root: &Path) -> Result<(), String> {
    println!("== builtin vocabulary");
    write_vocabulary(
        &root.join(VOCABULARY_DIR).join(VOCABULARY_FILE),
        &pinned_builtin_declarations()?,
    )?;
    println!("[ok]     builtin vocabulary written");
    println!("\nReview `git diff --stat`, then stage:");
    println!("  git add {VOCABULARY_DIR}");
    Ok(())
}

/// Where a re-vendor cannot work: with `CARGO_TARGET_DIR` set (a staged
/// build and the guest crates would share it), or in a checkout nested in
/// another Cargo workspace (cargo then reads the outer one).
fn preflight(root: &Path, target_dir_set: bool) -> Result<(), String> {
    if target_dir_set {
        return Err(
            "CARGO_TARGET_DIR is set: unset it (`env -u CARGO_TARGET_DIR cargo run -p xtask --bin revendor`); \
             the builtin blobs build in target/blob-stage/"
                .to_string(),
        );
    }
    for ancestor in root.ancestors().skip(1) {
        let manifest = ancestor.join("Cargo.toml");
        if std::fs::read_to_string(&manifest).is_ok_and(|text| text.contains("[workspace]")) {
            return Err(format!(
                "{} sits inside the Cargo workspace at {}: run it from the main checkout, or a worktree outside the repository",
                root.display(),
                ancestor.display()
            ));
        }
    }
    Ok(())
}

fn run(root: &Path) -> Result<(), String> {
    println!("== lockfiles");
    let mut changed = Vec::new();
    for blob in COMMITTED {
        if refresh_lockfile(root, blob.crate_dir)? {
            changed.push(format!("{}/Cargo.lock", blob.crate_dir));
        }
    }
    println!("== blobs");
    xtask(root, "build-blobs", &["--install"])?;
    println!("== declaration snapshots");
    let written = write_snapshots()?;
    println!("[ok]     {written} snapshot files written");
    println!("== builtin vocabulary");
    write_vocabulary(
        &root.join(VOCABULARY_DIR).join(VOCABULARY_FILE),
        &pinned_builtin_declarations()?,
    )?;
    println!("[ok]     builtin vocabulary written");
    println!("== checks");
    xtask(root, "build-blobs", &["--check"])?;
    declarations_test(root)?;

    println!("\nre-vendored. Review `git diff --stat`, then stage:");
    let paths: Vec<String> = COMMITTED
        .iter()
        .flat_map(|blob| {
            [
                format!("{}/wasm", blob.crate_dir),
                format!("{}/Cargo.lock", blob.crate_dir),
            ]
        })
        .collect();
    println!(
        "  git add {} crates/runtime/component/tests/declarations {VOCABULARY_DIR}",
        paths.join(" ")
    );
    if !changed.is_empty() {
        println!("lockfiles refreshed: {}", changed.join(", "));
    }
    Ok(())
}

/// `cargo metadata --locked` for `guest`; when the lockfile is stale, once
/// without `--locked` (a stale guest lockfile follows a workspace dependency
/// change, never a decision). True when the lockfile changed.
fn refresh_lockfile(root: &Path, guest: &str) -> Result<bool, String> {
    let manifest = root.join(guest).join("Cargo.toml");
    let lock = root.join(guest).join("Cargo.lock");
    let before = std::fs::read(&lock).ok();
    let metadata = |locked: bool| {
        let mut cmd = Command::new("cargo");
        cmd.args([
            "metadata",
            "--format-version",
            "1",
            "--quiet",
            "--manifest-path",
        ])
        .arg(&manifest)
        .stdout(std::process::Stdio::null());
        if locked {
            cmd.arg("--locked");
        }
        cmd.status().is_ok_and(|s| s.success())
    };
    if !metadata(true) && !metadata(false) {
        return Err(format!("cargo metadata failed for {guest}"));
    }
    let changed = std::fs::read(&lock).ok() != before;
    println!("[{}] {guest}", if changed { "lock" } else { "ok  " });
    Ok(changed)
}

/// Rewrite the pinned wire answers of every SDK blob from the blobs just vendored, and remove a
/// snapshot directory no SDK blob names any more. Returns the number of files written.
fn write_snapshots() -> Result<usize, String> {
    let root = snapshots();
    let runtime = ComponentRuntime::new();
    let mut written = 0;
    let mut kept = Vec::new();
    for blob in COMMITTED.iter().filter(|blob| blob.is_sdk()) {
        runtime
            .load(
                blob.name,
                &blob.read(),
                specforge_wasm::Provenance::third_party(&blob.read()),
            )
            .map_err(|e| format!("{}: {e}", blob.name))?;
        let dir = root.join(blob.key());
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for answer in wire_answers(&runtime, blob)? {
            let path = dir.join(&answer.file);
            std::fs::write(&path, &answer.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            written += 1;
        }
        kept.push(blob.key());
    }
    for entry in std::fs::read_dir(&root).map_err(|e| format!("{}: {e}", root.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() && !kept.contains(&name) {
            std::fs::remove_dir_all(entry.path())
                .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        }
    }
    Ok(written)
}

/// Write the candidate vocabulary of `declarations` to `path` (ADR 0122, ADR 0369). A keyword
/// several of them declare is listed with each; a field two of them add to one kind stops it,
/// with an error that names both. The file is replaced whole, by a rename of a file written beside
/// it, so a write that fails leaves the tracked file as it was.
fn write_vocabulary(path: &Path, declarations: &[ExtensionDeclaration]) -> Result<(), String> {
    let vocabulary = CandidateVocabulary::of(declarations)?;
    specforge_common::fresh::replace_file(path, vocabulary.to_json().as_bytes())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The cargo arguments of the declarations test: the `declarations` module of the component's `suite` target.
const DECLARATIONS_TEST: [&str; 9] = [
    "test",
    "-q",
    "-p",
    "specforge-component",
    "--test",
    "suite",
    "--",
    "declarations::",
    "--include-ignored",
];

/// The declarations test: the snapshot set, the typed round trip and the protocol freshness test
/// can fail on a fresh write. (The comparison with the files just written cannot.)
fn declarations_test(root: &Path) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(DECLARATIONS_TEST)
        .current_dir(root)
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("the declarations test failed".to_string())
    }
}

/// Run a sibling xtask bin from the repository root.
fn xtask(root: &Path, bin: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(["run", "-q", "-p", "xtask", "--bin", bin, "--"])
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{bin} {} failed", args.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_committed_wasm_is_listed() {
        let root = specforge_repo_root::repo_root();
        let out = Command::new("git")
            .args(["ls-files", "-z", "*.wasm"])
            .current_dir(root)
            .output()
            .expect("git runs");
        assert!(out.status.success(), "git ls-files failed");
        let tracked: std::collections::BTreeSet<String> = String::from_utf8(out.stdout)
            .unwrap()
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect();
        let listed: std::collections::BTreeSet<String> = specforge_component::committed::COMMITTED
            .iter()
            .map(|blob| blob.repo_path())
            .collect();
        assert_eq!(
            tracked, listed,
            "git tracks these .wasm files; list each in specforge_component::committed"
        );
    }

    /// The step runs a test target that exists (it was `declarations` after the target became `suite`).
    #[test]
    fn the_declarations_test_names_a_real_target_and_module() {
        let root = specforge_repo_root::repo_root();
        let target = DECLARATIONS_TEST[DECLARATIONS_TEST
            .iter()
            .position(|a| *a == "--test")
            .unwrap()
            + 1];
        let manifest =
            std::fs::read_to_string(root.join("crates/runtime/component/Cargo.toml")).unwrap();
        assert!(
            manifest.contains(&format!("name = \"{target}\"")),
            "no [[test]] {target}"
        );
        let suite = std::fs::read_to_string(
            root.join(format!("crates/runtime/component/tests/{target}.rs")),
        )
        .unwrap();
        assert!(
            suite.contains("mod declarations;"),
            "{target} has no declarations module"
        );
    }

    #[test]
    fn refuses_when_cargo_target_dir_is_set() {
        let err = preflight(Path::new("/nowhere/repo"), true).unwrap_err();
        assert!(err.contains("CARGO_TARGET_DIR"), "{err}");
    }

    #[test]
    fn refuses_a_checkout_nested_in_another_workspace() {
        let outer = std::env::temp_dir().join(format!("revendor-nested-{}", std::process::id()));
        let inner = outer.join("worktrees/repo");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(outer.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        let err = preflight(&inner, false).unwrap_err();
        std::fs::remove_dir_all(&outer).unwrap();
        assert!(err.contains("inside the Cargo workspace"), "{err}");
    }

    #[test]
    fn accepts_a_standalone_checkout() {
        let dir = std::env::temp_dir().join(format!("revendor-alone-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ok = preflight(&dir, false);
        std::fs::remove_dir_all(&dir).unwrap();
        // /tmp has no enclosing workspace manifest
        assert!(ok.is_ok(), "{ok:?}");
    }

    /// The kinds `@gsd/core` declares, as a declaration named `@gsd/core` with nothing else in it.
    /// They are read from `gsd/core/tests/golden/kinds.json`, which the gsd suite's own golden test
    /// keeps equal to the extension's declaration, so this follows every later change to those
    /// kinds with no edit here.
    fn gsd_core_kinds_golden() -> ExtensionDeclaration {
        let path = specforge_repo_root::repo_root().join("gsd/core/tests/golden/kinds.json");
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let golden: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let entities: Vec<specforge_protocol_types::EntityKindDescriptor> =
            serde_json::from_value(golden["entities"]["items"].clone())
                .unwrap_or_else(|e| panic!("{}: .entities.items: {e}", path.display()));
        assert!(!entities.is_empty(), "{}: no kind", path.display());
        ExtensionDeclaration {
            handshake: specforge_protocol_types::HandshakeResponse {
                name: "@gsd/core".to_string(),
                version: "0.0.0".to_string(),
                protocol_version: specforge_protocol_types::PROTOCOL_VERSION.to_string(),
                ..Default::default()
            },
            entities,
            ..Default::default()
        }
    }

    /// The `fields` map of a written vocabulary.
    fn written_fields(text: &str) -> serde_json::Value {
        let written: serde_json::Value = serde_json::from_str(text).unwrap();
        written["fields"].clone()
    }

    /// What the vocabulary step does with `gsd_core`, a declaration of `@gsd/core`, beside the
    /// pinned builtins (ADR 0369 D1): it does not stop and it writes one vocabulary. The keywords
    /// `@gsd/core` shares with the old builtins are computed from the declarations with the
    /// vocabulary's own keyword rule, never listed here; `decision`, `deliverable` and `milestone`
    /// are among them, and stay shared when later changes remove kinds from `@gsd/core`.
    fn the_step_takes_gsd_core_beside_the_builtins(gsd_core: ExtensionDeclaration) {
        assert_eq!(gsd_core.name(), "@gsd/core");
        let mut declarations = pinned_builtin_declarations().unwrap();
        // Once `@gsd/core` is a builtin itself, the declaration under test replaces its pinned one.
        declarations.retain(|declaration| declaration.name() != "@gsd/core");
        let old = CandidateVocabulary::of(&declarations).unwrap();
        let staged = CandidateVocabulary::of(std::slice::from_ref(&gsd_core)).unwrap();
        let shared: Vec<&str> = staged
            .kind_keywords()
            .filter(|keyword| !old.kind_owners(keyword).is_empty())
            .collect();
        println!(
            "shared kind keywords ({}): {}",
            shared.len(),
            shared.join(", ")
        );
        for keyword in ["decision", "deliverable", "milestone"] {
            assert!(
                shared.contains(&keyword),
                "'{keyword}' is not shared: {shared:?}"
            );
        }

        declarations.push(gsd_core);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("builtin-vocabulary.json");
        write_vocabulary(&path, &declarations).expect("the step does not stop");
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            1,
            "one vocabulary"
        );
        let text = std::fs::read_to_string(&path).unwrap();
        let written = CandidateVocabulary::from_json(&text);

        for keyword in staged.kind_keywords() {
            let owners = written.kind_owners(keyword);
            if shared.contains(&keyword) {
                // `@gsd/core` first, then the old builtins that declare it.
                let mut expected = vec!["@gsd/core"];
                expected.extend(old.kind_owners(keyword));
                assert_eq!(owners, expected, "the shared keyword '{keyword}'");
            } else {
                assert_eq!(owners, ["@gsd/core"], "the keyword '{keyword}'");
            }
        }
        for keyword in old.kind_keywords().filter(|k| !shared.contains(k)) {
            assert_eq!(
                written.kind_owners(keyword),
                old.kind_owners(keyword),
                "the old builtins' keyword '{keyword}'"
            );
        }
        // `@gsd/core` declares no enhancement: the fields are the old builtins'.
        assert_eq!(written_fields(&text), written_fields(&old.to_json()));
    }

    /// The step takes `@gsd/core`'s kinds, read from the tracked golden, beside the pinned builtins.
    /// It runs in every test run; the staged test below makes the same assertions on a built
    /// component.
    #[test]
    fn the_vocabulary_step_takes_gsd_cores_kinds_golden_beside_the_builtins() {
        the_step_takes_gsd_core_beside_the_builtins(gsd_core_kinds_golden());
    }

    /// A field two builtins add to one kind still stops the step, naming both and the field
    /// (ADR 0369 D4): a copy of `@specforge/formal` under another name adds every field it adds.
    #[test]
    fn the_vocabulary_step_still_stops_on_a_field_two_builtins_add_to_one_kind() {
        let mut declarations = pinned_builtin_declarations().unwrap();
        let mut copy = declarations
            .iter()
            .find(|declaration| declaration.name() == "@specforge/formal")
            .expect("@specforge/formal is a builtin")
            .clone();
        copy.handshake.name = "@staged/formal".to_string();
        let (kind, field) = copy
            .enhancements
            .iter()
            .find_map(|enhancement| {
                let field = enhancement.fields.first()?;
                Some((enhancement.target_kind.clone(), field.name.clone()))
            })
            .expect("@specforge/formal adds a field to another extension's kind");
        declarations.push(copy);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("builtin-vocabulary.json");
        let err = write_vocabulary(&path, &declarations).unwrap_err();
        assert!(
            err.contains("@specforge/formal and @staged/formal both add")
                && err.contains(&format!("'{field}'"))
                && err.contains(&format!("'{kind}'")),
            "{err}"
        );
        assert!(!path.exists(), "a step that stops writes nothing");
    }

    /// The tracked file is replaced by a rename of a file written beside it: the old text is there until
    /// the new one is, no temporary file is left, and a name that cannot be replaced is left as it was.
    #[test]
    fn the_vocabulary_step_replaces_the_tracked_file_whole() {
        let declarations = pinned_builtin_declarations().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("builtin-vocabulary.json");
        std::fs::write(&path, "{\"truncated\"").unwrap();
        write_vocabulary(&path, &declarations).expect("the step does not stop");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            CandidateVocabulary::of(&declarations).unwrap().to_json()
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "no temp");

        let blocked = dir.path().join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        write_vocabulary(&blocked, &declarations).unwrap_err();
        assert!(blocked.is_dir(), "a name that cannot be replaced stays");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2, "no temp");
    }

    /// The step takes the declaration of a built `@gsd/core` component beside the pinned builtins.
    /// The build of `gsd/core` stands for the staged blob: its declaration comes from the same
    /// source. The vendoring change of ADR 0366 replaces this test with the session's drift test
    /// over fifteen builtins.
    #[test]
    #[ignore = "needs GSD_CORE_WASM"]
    fn the_vocabulary_step_takes_the_staged_gsd_core_beside_the_builtins() {
        let path = std::path::PathBuf::from(std::env::var_os("GSD_CORE_WASM").expect(
            "GSD_CORE_WASM names gsd_core.wasm: (cd gsd && cargo build --locked -p gsd-core --release --target wasm32-wasip2)",
        ));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let runtime = ComponentRuntime::new();
        runtime
            .load(
                "@gsd/core",
                &bytes,
                specforge_wasm::Provenance::third_party(&bytes),
            )
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let loaded = specforge_wasm::protocol::load_declaration(&runtime, "@gsd/core")
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(loaded.declaration.name(), "@gsd/core");
        the_step_takes_gsd_core_beside_the_builtins(loaded.declaration);
    }
}
