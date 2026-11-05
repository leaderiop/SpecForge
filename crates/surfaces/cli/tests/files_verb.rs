//! `specforge files` (ADR 0339): the Fs port for people and agents. Its answers are JSON first and pinned, a path is
//! read from the current directory and held to a root of the project, and only the effects ceiling narrows it.

use serde_json::Value;

use crate::support::{Cli, TestProject};

/// A project with a spec file, a source tree, an ignored build directory and the host's own state.
fn project() -> TestProject {
    TestProject::new()
        .config(|config| config["spec_root"] = "spec".into())
        .files(&[
            ("spec/a.spec", "spec a \"A\" {}\n"),
            ("src/main.rs", "fn main() {\n    println!(\"hello\");\n}\n"),
            ("src/lib/util.rs", "pub fn hello() {}\n"),
            ("build/out.txt", "hello\n"),
            (".gitignore", "build/\n"),
            (".specforge/effects/journal/x.json", "{}"),
        ])
}

/// `run`'s JSON with every `modified_ms` and `real` written as a placeholder: they differ from run to run.
fn steady(mut value: Value) -> Value {
    if let Some(entries) = value.get_mut("entries").and_then(Value::as_array_mut) {
        for entry in entries {
            for key in ["modified_ms", "real"] {
                if let Some(field) = entry.get_mut(key)
                    && !field.is_null()
                {
                    *field = Value::String(format!("<{key}>"));
                }
            }
        }
    }
    value
}

#[test]
fn stat_answers_each_path_kind_and_size() {
    let project = project();
    let run = Cli::on(&project).run(&["files", "stat", "src/main.rs", "src", "nosuch.txt"]);
    let run = run.exits(0);
    insta::assert_json_snapshot!("files_stat", steady(run.answer()));
}

#[test]
fn list_honours_gitignore_and_never_lists_the_hosts_state() {
    let project = project();
    let run = Cli::on(&project).run(&["files", "list"]).exits(0);
    insta::assert_json_snapshot!("files_list", run.answer());
    let all = Cli::on(&project)
        .run(&["files", "list", "--no-ignore", "--pick", ".entries[].path"])
        .exits(0);
    assert!(all.stdout.contains("build/out.txt"), "{}", all.stdout);
    assert!(!all.stdout.contains(".specforge"), "{}", all.stdout);
}

#[test]
fn list_filters_by_glob_kind_and_depth() {
    let project = project();
    let picked = Cli::on(&project)
        .run(&[
            "files",
            "list",
            "src",
            "--glob",
            "*.rs",
            "--pick",
            ".entries[].path",
        ])
        .exits(0);
    assert_eq!(picked.stdout, "src/lib/util.rs\nsrc/main.rs\n");
    let dirs = Cli::on(&project)
        .run(&[
            "files",
            "list",
            "--kind",
            "dir",
            "--depth",
            "1",
            "--pick",
            ".entries[].path",
        ])
        .exits(0);
    assert_eq!(dirs.stdout, "spec\nsrc\n");
    let nested = Cli::on(&project)
        .run(&["files", "list", "--glob", "src/**/*.rs", "--pick", ".count"])
        .exits(0);
    assert_eq!(nested.stdout, "2\n");
}

/// `!GLOB` leaves out a file, or a directory and what it holds, on `list` and `search` alike.
#[test]
fn a_bang_glob_excludes_from_list_and_search() {
    let project = project();
    let listed = Cli::on(&project)
        .run(&[
            "files",
            "list",
            "--kind",
            "file",
            "--glob",
            "!lib",
            "--glob",
            "!*.spec",
            "--glob",
            "!*.json",
            "--pick",
            ".entries[].path",
        ])
        .exits(0);
    assert_eq!(listed.stdout, "src/main.rs\n");
    let searched = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "hello",
            "src",
            "--glob",
            "!main.rs",
            "--pick",
            ".matches[].path",
        ])
        .exits(0);
    assert_eq!(searched.stdout, "src/lib/util.rs\n");
}

#[test]
fn search_answers_each_matching_line() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "search", "hello", "src"])
        .exits(0);
    insta::assert_json_snapshot!("files_search", run.answer());
    let ignored = Cli::on(&project)
        .run(&[
            "files",
            "search",
            "HELLO",
            "-i",
            "--pick",
            ".matches[].path",
        ])
        .exits(0);
    assert!(!ignored.stdout.contains("build/"), "{}", ignored.stdout);
}

/// A line with two matches is answered once, at its first; a CRLF line's text holds no `\r`.
#[test]
fn search_reports_every_line_once_and_the_first_column() {
    let project = project().files(&[("notes/crlf.txt", "say hello, hello\r\nno\r\nhello\r\n")]);
    let run = Cli::on(&project)
        .run(&["files", "search", "hello", "notes"])
        .exits(0)
        .json();
    let matches = run["matches"].as_array().cloned().unwrap_or_default();
    let seen: Vec<(u64, u64, &str)> = matches
        .iter()
        .map(|found| {
            (
                found["line"].as_u64().unwrap_or(0),
                found["column"].as_u64().unwrap_or(0),
                found["text"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(seen, [(1, 5, "say hello, hello"), (3, 1, "hello")], "{run}");
    // A line anchor holds at a CRLF line's end.
    let anchored = Cli::on(&project)
        .run(&["files", "search", "hello$", "notes", "--pick", ".count"])
        .exits(0);
    assert_eq!(anchored.stdout, "2\n");
}

#[test]
fn a_path_from_a_subdirectory_is_read_from_there_and_printed_from_there() {
    let project = project();
    let run = Cli::on(&project)
        .in_dir(project.root().join("src"))
        .run(&[
            "files",
            "list",
            ".",
            "--glob",
            "*.rs",
            "--pick",
            ".entries[].path",
        ])
        .exits(0);
    assert_eq!(run.stdout, "lib/util.rs\nmain.rs\n");
}

#[test]
fn a_path_outside_every_root_is_refused_with_the_json_error() {
    let project = project();
    let outside = tempfile::tempdir().unwrap();
    let typed = outside.path().display().to_string();
    let run = Cli::on(&project).run(&["files", "stat", &typed]).exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    let climbed = Cli::on(&project).run(&["files", "list", ".."]).exits(1);
    assert_eq!(climbed.json()["code"], "outside_root", "{}", climbed.stdout);
}

#[test]
fn the_hosts_own_state_is_withheld() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "stat", ".specforge/effects/journal/x.json"])
        .exits(1);
    assert_eq!(run.json()["code"], "permission_denied", "{}", run.stdout);
}

#[test]
fn a_ceiling_that_leaves_no_root_for_the_path_refuses() {
    let project = project().config(|config| {
        config["effects"] = serde_json::json!({"fs": {"roots": ["spec"]}});
    });
    let run = Cli::on(&project)
        .run(&["files", "stat", "src/main.rs"])
        .exits(1);
    assert_eq!(run.json()["code"], "E090", "{}", run.stdout);
    // The spec root is still read.
    Cli::on(&project)
        .run(&["files", "stat", "spec/a.spec"])
        .exits(0);
    let off = project_with_fs_off();
    let refused = Cli::on(&off).run(&["files", "list"]).exits(1);
    assert_eq!(refused.json()["code"], "E090", "{}", refused.stdout);
}

fn project_with_fs_off() -> TestProject {
    project().config(|config| {
        config["effects"] = serde_json::json!({"fs": "off"});
    })
}

#[test]
fn pick_takes_no_human_format() {
    let project = project();
    Cli::on(&project)
        .run(&["files", "list", "--format", "human", "--pick", ".count"])
        .exits(2);
}

#[test]
fn the_human_listing_names_each_entry() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "list", "src", "--format", "human"])
        .exits(0);
    assert_eq!(run.stdout, "src/lib/\nsrc/lib/util.rs\nsrc/main.rs\n");
}

/// A link is followed to where it lands: out of every root, or into the host's own state, it is refused, and a
/// listing never descends through one.
#[cfg(unix)]
#[test]
fn a_link_out_of_the_roots_or_into_the_hosts_state_is_refused() {
    let project = project();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "hello").unwrap();
    std::os::unix::fs::symlink(outside.path(), project.root().join("src/out")).unwrap();
    std::os::unix::fs::symlink(
        project.root().join(".specforge/effects"),
        project.root().join("src/state"),
    )
    .unwrap();
    let out = Cli::on(&project)
        .run(&["files", "stat", "src/out/secret.txt"])
        .exits(1);
    assert_eq!(out.json()["code"], "outside_root", "{}", out.stdout);
    let state = Cli::on(&project)
        .run(&["files", "search", "x", "src/state"])
        .exits(1);
    assert_eq!(
        state.json()["code"],
        "permission_denied",
        "{}",
        state.stdout
    );
    let listed = Cli::on(&project)
        .run(&["files", "search", "hello", "--pick", ".matches[].path"])
        .exits(0);
    assert!(!listed.stdout.contains("secret"), "{}", listed.stdout);
}

#[test]
fn temp_answers_a_new_private_path_under_the_temp_root() {
    let project = project();
    let first = Cli::on(&project)
        .run(&["files", "temp", "--suffix", ".json", "--pick", ".path"])
        .exits(0);
    let second = Cli::on(&project)
        .run(&["files", "temp", "--suffix", ".json", "--pick", ".path"])
        .exits(0);
    let (first, second) = (first.stdout.trim(), second.stdout.trim());
    assert_ne!(first, second);
    let temp = std::fs::canonicalize(project.root().join(".specforge/tmp")).unwrap();
    for made in [first, second] {
        let made = std::path::Path::new(made);
        assert!(made.starts_with(&temp), "{}", made.display());
        assert!(made.to_string_lossy().ends_with(".json"));
        assert!(made.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(made).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
    // What it made is reachable through the temp root, and removable there.
    let name = std::path::Path::new(first)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let typed = format!(".specforge/tmp/{name}");
    let stat = Cli::on(&project)
        .run(&["files", "stat", &typed, "--pick", ".entries[0].root"])
        .exits(0);
    assert_eq!(stat.stdout, "temp\n");
    Cli::on(&project).run(&["files", "remove", first]).exits(0);
    assert!(!std::path::Path::new(first).exists());
    // A name part takes no separator.
    Cli::on(&project)
        .run(&["files", "temp", "--prefix", "../x"])
        .exits(1);
}

/// A committed `.specforge/tmp` link out of the project is refused before anything is made or chmod-ed there: the
/// directory it points at keeps its mode.
#[cfg(unix)]
#[test]
fn temp_never_makes_or_chmods_through_a_link() {
    use std::os::unix::fs::PermissionsExt as _;
    let project = project();
    let outside = tempfile::tempdir().unwrap();
    std::fs::set_permissions(outside.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    std::os::unix::fs::symlink(outside.path(), project.root().join(".specforge/tmp")).unwrap();
    Cli::on(&project).run(&["files", "temp"]).exits(1);
    let mode = std::fs::metadata(outside.path())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn mkdir_copy_and_move_answer_what_they_changed() {
    let project = project();
    let made = Cli::on(&project)
        .run(&["files", "mkdir", "out/deep/er", "--pick", ".changed[]"])
        .exits(0);
    assert_eq!(made.stdout, "out/deep/er\n");
    let copied = Cli::on(&project)
        .run(&["files", "copy", "src", "out", "--pick", ".changed[]"])
        .exits(0);
    assert_eq!(copied.stdout, "out/src\n");
    assert!(project.root().join("out/src/lib/util.rs").is_file());
    let moved = Cli::on(&project)
        .run(&[
            "files",
            "move",
            "out/src/main.rs",
            "out/deep",
            "--pick",
            ".changed[]",
        ])
        .exits(0);
    assert_eq!(moved.stdout, "out/src/main.rs\nout/deep/main.rs\n");
    assert!(project.root().join("out/deep/main.rs").is_file());
    assert!(!project.root().join("out/src/main.rs").exists());
    // Into itself is refused.
    Cli::on(&project)
        .run(&["files", "copy", "out", "out/deep"])
        .exits(1);
}

#[test]
fn remove_refuses_a_root_a_path_outside_and_the_hosts_state_and_changes_nothing() {
    let project = project();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("keep.txt"), "x").unwrap();
    let typed = outside.path().join("keep.txt").display().to_string();
    // The refused path is named last: nothing before it is removed.
    let run = Cli::on(&project)
        .run(&["files", "remove", "src/main.rs", &typed])
        .exits(1);
    assert_eq!(run.json()["code"], "outside_root", "{}", run.stdout);
    assert!(outside.path().join("keep.txt").exists());
    assert!(project.root().join("src/main.rs").exists());
    for root in [".", "spec"] {
        let run = Cli::on(&project)
            .run(&["files", "remove", "--recursive", root])
            .exits(1);
        assert_eq!(
            run.json()["code"],
            "permission_denied",
            "{root}: {}",
            run.stdout
        );
    }
    assert!(project.root().join("spec/a.spec").exists());
    let state = Cli::on(&project)
        .run(&["files", "remove", ".specforge/effects/journal/x.json"])
        .exits(1);
    assert_eq!(
        state.json()["code"],
        "permission_denied",
        "{}",
        state.stdout
    );
    assert!(
        project
            .root()
            .join(".specforge/effects/journal/x.json")
            .exists()
    );
    // A directory needs --recursive.
    Cli::on(&project)
        .run(&["files", "remove", "src/lib"])
        .exits(1);
    Cli::on(&project)
        .run(&["files", "remove", "-r", "src/lib"])
        .exits(0);
    assert!(!project.root().join("src/lib").exists());
}

/// A link is removed or moved itself, never what it points at.
#[cfg(unix)]
#[test]
fn removing_a_link_leaves_its_target() {
    let project = project();
    std::os::unix::fs::symlink(
        project.root().join("src/main.rs"),
        project.root().join("src/link.rs"),
    )
    .unwrap();
    Cli::on(&project)
        .run(&["files", "remove", "src/link.rs"])
        .exits(0);
    assert!(project.root().join("src/main.rs").is_file());
    assert!(std::fs::symlink_metadata(project.root().join("src/link.rs")).is_err());
}

#[test]
fn digest_is_the_sha256_of_a_file_and_of_text() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "digest", "build/out.txt", "--text", "hello\n"])
        .exits(0);
    insta::assert_json_snapshot!("files_digest", run.answer());
    let digests = &run.json()["digests"];
    assert_eq!(digests[0]["sha256"], digests[1]["sha256"]);
    Cli::on(&project).run(&["files", "digest", "src"]).exits(1);
    Cli::on(&project)
        .run(&["files", "digest", ".specforge/effects/journal/x.json"])
        .exits(1);
}

#[test]
fn a_ceiling_without_write_refuses_every_change() {
    let project = project().config(|config| {
        config["effects"] = serde_json::json!({"fs": {"ops": ["stat", "read", "walk"]}});
    });
    let run = Cli::on(&project).run(&["files", "mkdir", "out"]).exits(1);
    assert_eq!(run.json()["code"], "E090", "{}", run.stdout);
    let run = Cli::on(&project).run(&["files", "temp"]).exits(1);
    assert_eq!(run.json()["code"], "E090", "{}", run.stdout);
    assert!(!project.root().join("out").exists());
    assert!(!project.root().join(".specforge/tmp").exists());
}

/// A shell reads what `--pick` prints: a refusal leaves stdout empty and says why on stderr.
#[test]
fn a_refusal_under_pick_writes_nothing_to_stdout() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "list", "nosuch", "--pick", ".entries[0].path"])
        .exits(1);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.contains("file_not_found"), "{}", run.stderr);
}

#[test]
fn remove_force_skips_a_missing_path() {
    let project = project();
    Cli::on(&project)
        .run(&["files", "remove", "nosuch.txt"])
        .exits(1);
    let run = Cli::on(&project)
        .run(&[
            "files",
            "remove",
            "--force",
            "nosuch.txt",
            "build/out.txt",
            "--pick",
            ".changed[]",
        ])
        .exits(0);
    assert_eq!(run.stdout, "build/out.txt\n");
}

/// fs reads and writes are local: performed in every effects mode, `replay` included, and never journaled (ADR 0337
/// D3, D4), so the answer says it was performed now, with no record.
#[test]
fn files_takes_every_effects_mode_and_performs_the_read_in_each() {
    let project = project();
    for mode in ["online", "auto", "cached", "replay"] {
        let read = Cli::on(&project)
            .run(&["files", "stat", "src/main.rs", "--effects", mode])
            .exits(0)
            .json();
        assert_eq!(read["entries"][0]["kind"], "file", "{mode}: {read}");
        assert_eq!(
            (&read["effect"]["digest"], &read["effect"]["from_journal"]),
            (&Value::Null, &Value::Bool(false)),
            "{mode}"
        );
    }
    let written = Cli::on(&project)
        .run(&["files", "mkdir", "made", "--effects", "replay"])
        .exits(0)
        .answer();
    assert_eq!(
        written["changed"].as_array().map(Vec::len),
        Some(1),
        "{written}"
    );
    assert!(project.root().join("made").is_dir());
    let journal = std::fs::read_dir(project.root().join(".specforge/effects/journal")).unwrap();
    assert_eq!(
        journal.count(),
        1,
        "a local effect writes no record: only the fixture's is there"
    );
}
