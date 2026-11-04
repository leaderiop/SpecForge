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
    insta::assert_json_snapshot!("files_stat", steady(run.json()));
}

#[test]
fn list_honours_gitignore_and_never_lists_the_hosts_state() {
    let project = project();
    let run = Cli::on(&project).run(&["files", "list"]).exits(0);
    insta::assert_json_snapshot!("files_list", run.json());
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

#[test]
fn search_answers_each_matching_line() {
    let project = project();
    let run = Cli::on(&project)
        .run(&["files", "search", "hello", "src"])
        .exits(0);
    insta::assert_json_snapshot!("files_search", run.json());
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
fn pick_takes_no_human_format_and_files_take_no_effects_mode() {
    let project = project();
    Cli::on(&project)
        .run(&["files", "list", "--format", "human", "--pick", ".count"])
        .exits(2);
    // A local read is never journaled (ADR 0337 D3): there is no mode to choose.
    Cli::on(&project)
        .run(&["files", "list", "--effects", "replay"])
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
