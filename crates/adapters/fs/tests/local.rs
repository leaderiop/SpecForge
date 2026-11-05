//! The `Fs` port's adapter over a real directory (ADR 0335): the contract its fake keeps, containment, the read cap
//! and the walk's skip list and links.
#![allow(
    clippy::unwrap_used,
    reason = "a test fails on the first unexpected value"
)]

use specforge_effects::{Fs, FsChange, FsOp, FsOutcome, FsRequest, OutcomeClass, Place, Roots};
use specforge_fs::LocalFs;

/// A project with `a.txt` (`a\n`) and a `.env`, and its roots.
fn project() -> (tempfile::TempDir, Roots) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("a.txt"), "a\n").unwrap();
    std::fs::write(root.join(".env"), "TOKEN=x\n").unwrap();
    std::fs::create_dir(root.join("spec")).unwrap();
    let roots = Roots::new(&root, root.join("spec"));
    (dir, roots)
}

fn run(roots: &Roots, path: &str, op: FsOp) -> FsOutcome {
    LocalFs.run(
        roots,
        &FsRequest {
            root: "project".into(),
            path: path.into(),
            op,
        },
    )
}

fn walk(roots: &Roots, glob: Option<&str>) -> Vec<String> {
    let walked = run(
        roots,
        "",
        FsOp::Walk {
            glob: glob.map(str::to_string),
            max_entries: 100,
        },
    );
    assert_eq!(walked.class, OutcomeClass::Ok, "{walked:?}");
    walked.entries
}

#[specforge_test_macros::test(port = "Fs", verify = "Fs contract is satisfied")]
fn the_local_fs_keeps_the_fs_contract() {
    let (_dir, roots) = project();
    specforge_effects::testing::fs_contract(&LocalFs, &roots);
}

#[specforge_test_macros::test(port = "Fs", verify = "Fs contract is satisfied")]
fn the_fake_fs_keeps_the_same_contract() {
    let roots = Roots::new("/p", "/p/spec");
    let fake = specforge_effects::testing::FakeFs::new()
        .with_file("/p/a.txt", "a\n")
        .with_file("/p/.env", "TOKEN=x\n");
    specforge_effects::testing::fs_contract(&fake, &roots);
}

#[test]
fn a_read_past_its_max_bytes_is_cut_there_with_the_files_size() {
    let (_dir, roots) = project();
    std::fs::write(roots.project().join("long.txt"), "0123456789").unwrap();
    let read = run(&roots, "long.txt", FsOp::Read { max_bytes: 4 });
    assert_eq!(read.class, OutcomeClass::Ok);
    assert_eq!(read.bytes, b"0123");
    assert!(read.truncated);
    assert_eq!(read.stat.unwrap().size, 10);
}

#[cfg(unix)]
#[test]
fn a_link_out_of_the_root_is_outside() {
    let (_dir, roots) = project();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("secret.txt"), "s").unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), roots.project().join("out")).unwrap();
    let read = run(&roots, "out/secret.txt", FsOp::Read { max_bytes: 10 });
    assert_eq!(read.class, OutcomeClass::Outside, "{read:?}");
}

#[cfg(unix)]
#[test]
fn a_link_to_a_withheld_file_is_not_read() {
    let (_dir, roots) = project();
    std::os::unix::fs::symlink(".env", roots.project().join("notes.txt")).unwrap();
    let read = FsOp::Read { max_bytes: 100 };
    assert_eq!(run(&roots, "notes.txt", read).class, OutcomeClass::Denied);
    assert_eq!(run(&roots, "notes.txt", FsOp::Stat).class, OutcomeClass::Ok);
}

#[test]
fn a_walk_skips_dependency_directories_the_glob_does_not_name() {
    let (_dir, roots) = project();
    let root = roots.project();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.ts"), "").unwrap();
    // Every directory ADR 0293 D4 names.
    for skipped in specforge_effects::fs::SKIPPED {
        std::fs::create_dir_all(root.join(skipped).join("pkg")).unwrap();
        std::fs::write(root.join(skipped).join("pkg/index.ts"), "").unwrap();
    }
    assert_eq!(walk(&roots, Some("**/*.ts")), ["src/main.ts"]);
    assert_eq!(
        walk(&roots, Some("node_modules/**/*.ts")),
        ["node_modules/pkg/index.ts"]
    );
}

/// A named root inside the project's own state, and a link into it, are the same state.
#[test]
fn no_request_reaches_the_hosts_own_state() {
    let (_dir, roots) = project();
    let root = roots.project().to_path_buf();
    let state = root.join(".specforge/effects/journal/ab");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("abc.payload"), "someone's answer").unwrap();
    let held = ".specforge/effects/journal/ab/abc.payload";
    for op in [
        FsOp::Stat,
        FsOp::Read { max_bytes: 1024 },
        FsOp::Walk {
            glob: None,
            max_entries: 10,
        },
    ] {
        assert_eq!(run(&roots, held, op.clone()).class, OutcomeClass::Denied);
        assert_eq!(run(&roots, ".specforge", op).class, OutcomeClass::Denied);
    }
    for glob in [
        ".specforge/**",
        "**/*.payload",
        ".specforge/effects/journal/ab/*",
    ] {
        assert!(walk(&roots, Some(glob)).is_empty(), "{glob}");
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(state.join("abc.payload"), root.join("alias.txt")).unwrap();
        let read = FsOp::Read { max_bytes: 1024 };
        assert_eq!(run(&roots, "alias.txt", read).class, OutcomeClass::Denied);
    }
    let named =
        Roots::new(&root, root.join("spec")).with_named("state", root.join(".specforge/effects"));
    for path in ["", "journal/ab/abc.payload"] {
        let request = FsRequest {
            root: "state".to_string(),
            path: path.to_string(),
            op: FsOp::Walk {
                glob: None,
                max_entries: 10,
            },
        };
        assert_eq!(
            LocalFs.run(&named, &request).class,
            OutcomeClass::Denied,
            "{path:?}"
        );
    }
}

#[test]
fn a_walk_stops_at_its_entry_budget() {
    let (_dir, roots) = project();
    let walked = run(
        &roots,
        "",
        FsOp::Walk {
            glob: None,
            max_entries: 1,
        },
    );
    assert!(walked.truncated);
    assert_eq!(walked.entries.len(), 1);
}

#[cfg(unix)]
#[test]
fn a_walk_does_not_descend_through_a_directory_link() {
    let (_dir, roots) = project();
    let root = roots.project();
    std::fs::create_dir(root.join("real")).unwrap();
    std::fs::write(root.join("real/x.md"), "").unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("linked")).unwrap();
    assert_eq!(walk(&roots, Some("**/*.md")), ["real/x.md"]);
}

/// The listed paths of `dir` under `globs`, files and directories.
fn listed(roots: &Roots, globs: &[&str]) -> Vec<String> {
    let listing = LocalFs.list(
        roots,
        &specforge_effects::ListRequest {
            root: "project".into(),
            path: String::new(),
            globs: globs.iter().map(|glob| (*glob).to_string()).collect(),
            kind: None,
            depth: None,
            ignore: true,
            hidden: false,
            max_entries: 100,
        },
    );
    assert_eq!(listing.class, OutcomeClass::Ok, "{listing:?}");
    listing
        .entries
        .into_iter()
        .map(|entry| entry.path)
        .collect()
}

/// A listing's globs are the `ignore` crate's overrides: gitignore semantics, a `!` glob leaves out a file or prunes
/// a directory, and no glob reaches a hidden name the listing leaves out.
#[test]
fn a_listing_takes_the_walkers_overrides() {
    let (_dir, roots) = project();
    let root = roots.project();
    for dir in ["src/deep", "build/src"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in [
        "src/a.rs",
        "src/deep/b.rs",
        "build/c.rs",
        "build/src/d.rs",
        "README.md",
        ".hidden.rs",
    ] {
        std::fs::write(root.join(file), "").unwrap();
    }
    // A name glob matches at any depth; a hidden name stays out.
    assert_eq!(
        listed(&roots, &["*.rs"]),
        ["build/c.rs", "build/src/d.rs", "src/a.rs", "src/deep/b.rs"]
    );
    // A glob with a `/` is anchored at the listed directory.
    assert_eq!(listed(&roots, &["src/*.rs"]), ["src/a.rs"]);
    // `!build` prunes the directory and what it holds.
    assert_eq!(
        listed(&roots, &["*.rs", "!build"]),
        ["src/a.rs", "src/deep/b.rs"]
    );
    // An exclusion alone keeps the rest, directories included.
    let rest = listed(&roots, &["!*.md", "!*.txt"]);
    assert!(!rest.iter().any(|path| path.ends_with(".md")), "{rest:?}");
    assert!(rest.contains(&"build/src".to_string()), "{rest:?}");
    assert!(rest.contains(&"src/deep/b.rs".to_string()), "{rest:?}");
    // A trailing `/` names a directory, not a file of that name.
    assert_eq!(listed(&roots, &["!build/"]), listed(&roots, &["!build"]));
}

fn at(root: &str, path: &str) -> specforge_effects::Place {
    specforge_effects::Place {
        root: root.into(),
        path: path.into(),
    }
}

/// Roots with no `temp` (every guest's) refuse it like an unknown root; the verbs' own roots reach it, in the host's
/// state, while the project's state stays withheld.
#[test]
fn only_roots_with_temp_reach_the_temp_root() {
    use specforge_effects::FsChange;
    let (dir, roots) = project();
    let root = dir.path().canonicalize().unwrap();
    let temp = root.join(".specforge/tmp");
    std::fs::create_dir_all(&temp).unwrap();
    let create = FsChange::Create {
        at: at("temp", "x.json"),
        dir: false,
    };
    assert_eq!(LocalFs.change(&roots, &create).class, OutcomeClass::Outside);
    let with = roots.clone().with_temp(&temp);
    assert_eq!(LocalFs.change(&with, &create).class, OutcomeClass::Ok);
    assert!(temp.join("x.json").is_file());
    let state = FsChange::Create {
        at: at("project", ".specforge/y"),
        dir: false,
    };
    assert_eq!(LocalFs.change(&with, &state).class, OutcomeClass::Denied);
}

#[test]
fn a_change_never_names_a_root_or_a_withheld_file() {
    use specforge_effects::FsChange;
    let (_dir, roots) = project();
    for path in ["", ".", "spec"] {
        let remove = FsChange::Remove {
            at: at("project", path),
            recursive: true,
        };
        assert_eq!(
            LocalFs.change(&roots, &remove).class,
            OutcomeClass::Denied,
            "{path:?}"
        );
    }
    let leak = FsChange::Copy {
        from: at("project", ".env"),
        to: at("project", "env.txt"),
    };
    assert_eq!(LocalFs.change(&roots, &leak).class, OutcomeClass::Denied);
    let digest = LocalFs.digest(&roots, &at("project", ".env"));
    assert_eq!(digest.class, OutcomeClass::Denied);
}

#[cfg(unix)]
#[test]
fn a_change_through_a_link_out_of_the_root_is_outside() {
    use specforge_effects::FsChange;
    let (dir, roots) = project();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("out")).unwrap();
    let mkdir = FsChange::Mkdir {
        at: at("project", "out/new"),
    };
    assert_eq!(LocalFs.change(&roots, &mkdir).class, OutcomeClass::Outside);
    assert!(!outside.path().join("new").exists());
}

/// A recursive copy, and a move that copies, never write through a link the destination holds: a committed
/// `b/a/x -> ~/.bashrc` is refused, the file it points at untouched, and so is a directory link inside it.
#[cfg(unix)]
#[test]
fn a_copy_never_writes_through_a_link_in_its_destination() {
    use specforge_effects::FsChange;
    let (dir, roots) = project();
    let root = dir.path().canonicalize().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let victim = outside.path().join("bashrc");
    std::fs::write(&victim, "mine\n").unwrap();
    std::fs::create_dir_all(root.join("a/sub")).unwrap();
    std::fs::write(root.join("a/x"), "theirs\n").unwrap();
    std::fs::write(root.join("a/sub/y"), "theirs\n").unwrap();
    std::fs::create_dir_all(root.join("b/a")).unwrap();
    std::os::unix::fs::symlink(&victim, root.join("b/a/x")).unwrap();
    let copy = FsChange::Copy {
        from: at("project", "a"),
        to: at("project", "b"),
    };
    assert_ne!(LocalFs.change(&roots, &copy).class, OutcomeClass::Ok);
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "mine\n");

    std::fs::remove_file(root.join("b/a/x")).unwrap();
    // The first copy may have made `b/a/sub` before it met the link.
    let _ = std::fs::remove_dir_all(root.join("b/a/sub"));
    std::os::unix::fs::symlink(outside.path(), root.join("b/a/sub")).unwrap();
    assert_ne!(LocalFs.change(&roots, &copy).class, OutcomeClass::Ok);
    assert!(!outside.path().join("y").exists());
}

/// A named pipe is judged before it is opened, so a digest of one is refused at once rather than waiting for a writer
/// that never comes (ADR 0264).
#[cfg(unix)]
#[test]
fn a_digest_of_a_named_pipe_is_refused_without_waiting() {
    let (dir, roots) = project();
    let pipe = dir.path().join("pipe");
    let made = std::process::Command::new("mkfifo").arg(&pipe).status();
    if !made.is_ok_and(|status| status.success()) {
        return; // no mkfifo here: nothing to judge
    }
    let asked = roots.clone();
    let (sent, got) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(LocalFs.digest(&asked, &at("project", "pipe")).class);
    });
    let class = got
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the digest waited on the pipe");
    assert_eq!(class, OutcomeClass::Denied);
    let digest = LocalFs.digest(&roots, &at("project", "a.txt"));
    assert_eq!(
        (digest.class, digest.sha256.as_str(), digest.bytes),
        (
            OutcomeClass::Ok,
            "87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7",
            2
        )
    );
}

#[test]
fn a_file_with_more_than_one_name_is_denied_a_read_and_a_digest() {
    let (dir, roots) = project();
    let root = dir.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("sec")).unwrap();
    std::fs::hard_link(root.join(".env"), root.join("sec/hl.txt")).unwrap();

    let read = run(&roots, "sec/hl.txt", FsOp::Read { max_bytes: 1024 });
    assert_eq!(read.class, OutcomeClass::Denied, "{read:?}");
    assert!(read.bytes.is_empty());
    let digest = run(&roots, "sec/hl.txt", FsOp::Digest);
    assert_eq!(digest.class, OutcomeClass::Denied, "{digest:?}");
    let at = Place {
        root: "project".into(),
        path: "sec/hl.txt".into(),
    };
    assert_eq!(LocalFs.digest(&roots, &at).class, OutcomeClass::Denied);
    // Its name stays listed: what a name says is no content.
    let stat = run(&roots, "sec/hl.txt", FsOp::Stat);
    assert_eq!(stat.class, OutcomeClass::Ok, "{stat:?}");
    let to = Place {
        root: "project".into(),
        path: "plain.txt".into(),
    };
    let copied = LocalFs.change(&roots, &FsChange::Copy { from: at, to });
    assert_eq!(copied.class, OutcomeClass::Denied, "{copied:?}");
    assert!(!root.join("plain.txt").exists());
}
