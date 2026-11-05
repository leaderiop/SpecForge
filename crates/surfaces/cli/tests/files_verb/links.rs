//! A file with more than one name (ADR 0146): its other name may be withheld or outside every root, so the verbs read
//! it through no bound, typed or walked, and the remedy says what to do.

use crate::support::{Cli, TestProject};

/// A project with a `.env`, `sec/hl.txt` a second name of it, and `sec/out.txt` a second name of a file outside.
fn linked() -> (TestProject, tempfile::TempDir) {
    let project = TestProject::new().files(&[
        (".env", "TOKEN=hunter2\n"),
        ("sec/plain.txt", "nothing here\n"),
    ]);
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    std::fs::write(&secret, "TOKEN=hunter2\n").unwrap();
    let root = project.root();
    std::fs::hard_link(root.join(".env"), root.join("sec/hl.txt")).unwrap();
    std::fs::hard_link(&secret, root.join("sec/out.txt")).unwrap();
    (project, outside)
}

#[test]
fn a_typed_read_of_a_second_name_is_refused_and_names_the_remedy() {
    let (project, _outside) = linked();
    for path in ["sec/hl.txt", "sec/out.txt"] {
        let run = Cli::on(&project)
            .run(&["files", "read", path, "--pick", ".text"])
            .exits(1);
        assert!(!run.stdout.contains("hunter2"), "{}", run.stdout);
        assert!(!run.stderr.contains("hunter2"), "{}", run.stderr);
        assert!(run.stderr.contains("copy it in"), "{}", run.stderr);
    }
}

#[test]
fn a_digest_of_a_second_name_is_refused() {
    let (project, _outside) = linked();
    Cli::on(&project)
        .run(&["files", "digest", "sec/out.txt"])
        .exits(1);
}

#[test]
fn a_copy_of_a_second_name_is_refused() {
    let (project, _outside) = linked();
    Cli::on(&project)
        .run(&["files", "copy", "sec/hl.txt", "plain.txt"])
        .exits(1);
    assert!(!project.root().join("plain.txt").exists());
}

#[test]
fn a_search_never_answers_from_a_second_name() {
    let (project, _outside) = linked();
    for args in [
        &["files", "search", "hunter2", "sec/hl.txt"][..],
        &["files", "search", "-q", "hunter2", "sec/out.txt"],
    ] {
        let run = Cli::on(&project).run(args);
        assert!(!run.stdout.contains("hunter2"), "{}", run.stdout);
        assert_ne!(run.code, 0, "{args:?}");
    }
    let walked = Cli::on(&project)
        .run(&["files", "search", "hunter2", "sec"])
        .exits(0);
    assert_eq!(walked.answer()["count"], 0, "{}", walked.stdout);
    assert!(!walked.stdout.contains("hunter2"), "{}", walked.stdout);
}

#[test]
fn list_lines_counts_no_second_name() {
    let (project, _outside) = linked();
    let run = Cli::on(&project)
        .run(&["files", "list", "sec", "--lines"])
        .exits(0);
    let answer = run.answer();
    assert_eq!(answer["lines"], 1, "{answer}");
    let entry = |name: &str| {
        answer["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == name)
            .unwrap()
            .clone()
    };
    assert!(entry("sec/hl.txt")["lines"].is_null(), "{answer}");
    assert_eq!(entry("sec/plain.txt")["lines"], 1, "{answer}");
}

#[test]
fn http_sends_no_second_name_as_a_body() {
    let (project, _outside) = linked();
    // Refused reading the body, before any request: the address needs no server.
    let run = Cli::on(&project)
        .run(&[
            "http",
            "post",
            "http://127.0.0.1:9/",
            "--body-file",
            "sec/hl.txt",
        ])
        .exits(1);
    assert!(run.stdout.contains("cannot be sent"), "{}", run.stdout);
    assert!(!run.stdout.contains("hunter2"), "{}", run.stdout);
}
