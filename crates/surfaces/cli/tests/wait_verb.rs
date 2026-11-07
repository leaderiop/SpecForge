//! `specforge wait` (ADR 0342): a wait for a URL on this machine, or for a time. The poll's schedule is tested on a
//! clock that waits no real time (`specforge_ops::verbs::wait`); these runs keep their windows short, with a deadline
//! at least ten times what the thing waited for takes.

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use serde_json::json;
use specforge_effects::testing::server::{Server, response};

use crate::support::{Cli, TestProject};

#[test]
fn with_no_condition_it_waits_its_time_and_exits_0() {
    let project = TestProject::new();
    let answer = Cli::on(&project)
        .run(&["wait", "--for", "200ms"])
        .exits(0)
        .answer();
    assert_eq!(answer["reason"], "elapsed", "{answer}");
    assert_eq!(
        (answer["for_ms"].clone(), answer["polls"].clone()),
        (json!(200), json!(0))
    );
    assert!(answer["waited_ms"].as_u64().unwrap() >= 200, "{answer}");
    assert!(answer["started_at"].as_u64().unwrap() > 0, "{answer}");
    assert!(answer.get("url").is_none(), "{answer}");
}

#[test]
fn a_url_that_answers_ends_the_wait() {
    let project = TestProject::new();
    let server = Server::answering(200, "text/plain", "up");
    let answer = Cli::on(&project)
        .run(&["wait", "--for", "30s", "--url", &server.base])
        .exits(0)
        .answer();
    assert_eq!(answer["reason"], "answered", "{answer}");
    assert_eq!(
        answer["url"],
        json!({"status": 200, "ok": true, "outcome": "ok"})
    );
    assert_eq!(answer["polls"], 1, "{answer}");
}

/// Any HTTP answer is the server answering: an API server with no `/` route is up.
#[test]
fn a_404_is_an_answer() {
    let project = TestProject::new();
    let server = Server::answering(404, "text/plain", "no route");
    let answer = Cli::on(&project)
        .run(&["wait", "--for", "30s", "--url", &server.base])
        .exits(0)
        .answer();
    assert_eq!(answer["reason"], "answered", "{answer}");
    assert_eq!(
        answer["url"],
        json!({"status": 404, "ok": false, "outcome": "status:404"})
    );
}

#[test]
fn a_server_that_comes_up_ends_the_wait() {
    let project = TestProject::new();
    let port = Server::closed_port();
    let late = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        let listener = TcpListener::bind(("127.0.0.1", port)).expect("the port is still free");
        let (mut stream, _) = listener.accept().expect("the wait connects");
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
            head.push(byte[0]);
        }
        let _ = stream.write_all(&response(200, &[], b"up"));
    });
    let answer = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "10s",
            "--url",
            &format!("http://127.0.0.1:{port}/"),
        ])
        .exits(0)
        .answer();
    late.join().unwrap();
    assert_eq!(answer["reason"], "answered", "{answer}");
    assert!(answer["polls"].as_u64().unwrap() > 1, "{answer}");
}

#[test]
fn no_answer_by_the_deadline_is_124() {
    let project = TestProject::new();
    let url = format!("http://127.0.0.1:{}/", Server::closed_port());
    let answer = Cli::on(&project)
        .run(&["wait", "--for", "300ms", "--url", &url])
        .exits(124)
        .answer();
    assert_eq!(answer["reason"], "deadline", "{answer}");
    assert_eq!(
        answer["url"],
        json!({"status": null, "ok": false, "outcome": "unreachable:connect"})
    );
    assert!(answer["polls"].as_u64().unwrap() > 1, "{answer}");
    let picked = Cli::on(&project)
        .run(&["wait", "--for", "0", "--url", &url, "--pick", ".reason"])
        .exits(124);
    assert_eq!(picked.stdout, "deadline\n");
}

#[test]
fn a_url_the_ceiling_refuses_is_refused_before_waiting() {
    let project = TestProject::new().config(|config| {
        config["effects"] = json!({"http": "off"});
    });
    let server = Server::answering(200, "text/plain", "up");
    let began = Instant::now();
    let refused = Cli::on(&project)
        .run(&["wait", "--for", "30s", "--url", &server.base])
        .exits(1);
    assert_eq!(refused.json()["code"], "E090", "{}", refused.stdout);
    assert!(began.elapsed() < Duration::from_secs(15));
    assert!(server.requests().is_empty(), "the server saw a request");
    let not_a_url = Cli::on(&project)
        .run(&["wait", "--for", "30s", "--url", "localhost:3000"])
        .exits(1);
    assert_eq!(
        not_a_url.json()["code"],
        "invalid_input",
        "{}",
        not_a_url.stdout
    );
}

#[test]
fn replay_answers_from_the_journal() {
    let project = TestProject::new();
    let server = Server::answering(200, "text/plain", "up");
    let never = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "30s",
            "--url",
            &server.base,
            "--effects",
            "replay",
        ])
        .exits(1);
    assert_eq!(never.json()["code"], "not_in_journal", "{}", never.stdout);
    Cli::on(&project)
        .run(&["wait", "--for", "30s", "--url", &server.base])
        .exits(0);
    let replayed = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "30s",
            "--url",
            &server.base,
            "--effects",
            "replay",
        ])
        .exits(0)
        .json();
    assert_eq!(replayed["reason"], "answered", "{replayed}");
    assert_eq!(replayed["effect"]["from_journal"], true, "{replayed}");
    // A wait asks about now: cached probes live.
    let cached = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "30s",
            "--url",
            &server.base,
            "--effects",
            "cached",
        ])
        .exits(0)
        .json();
    assert_eq!(cached["effect"]["from_journal"], false, "{cached}");
    assert_eq!(server.requests().len(), 2, "the replay sent nothing");
}

#[test]
fn a_bad_duration_or_a_lone_since_is_a_usage_error() {
    let project = TestProject::new();
    for args in [
        &["wait", "--for", "1d"][..],
        &["wait", "--for", "25h"],
        &["wait"],
        &["wait", "--for", "1", "--since", "5"],
        &["wait", "--for", "1", "--match", "x"],
        &["wait", "--for", "1", "--in", "log.txt"],
    ] {
        Cli::on(&project).run(args).exits(2);
    }
}

/// `project`'s `rel` written after `after`, on a thread the test joins.
fn written_later(
    project: &TestProject,
    rel: &str,
    text: &str,
    after: Duration,
) -> std::thread::JoinHandle<()> {
    let at = project.root().join(rel);
    let text = text.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(after);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&at)
            .and_then(|mut file| file.write_all(text.as_bytes()))
            .unwrap();
    })
}

#[test]
fn a_file_written_during_the_wait_ends_it() {
    let project = TestProject::new().file("watched/a.txt", "before");
    let later = written_later(
        &project,
        "watched/b.txt",
        "after",
        Duration::from_millis(300),
    );
    let answer = Cli::on(&project)
        .run(&["wait", "--for", "10s", "--changed", "watched/*.txt"])
        .exits(0)
        .answer();
    later.join().unwrap();
    assert_eq!(answer["reason"], "changed", "{answer}");
    assert_eq!(answer["changed"], json!(["watched/b.txt"]));
}

#[test]
fn since_counts_writes_before_the_wait() {
    let project = TestProject::new().file("watched/a.txt", "before");
    let answer = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "0",
            "--changed",
            "watched/*.txt",
            "--since",
            "0",
        ])
        .exits(0)
        .answer();
    assert_eq!(answer["changed"], json!(["watched/a.txt"]));
    let quiet = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "0",
            "--changed",
            "**/*.txt",
            "--since",
            "99999999999999",
            "--pick",
            ".reason",
        ])
        .exits(124);
    assert_eq!(quiet.stdout, "deadline\n");
}

/// A glob with no glob character names one file; a file or directory that does not exist yet is "not yet".
#[test]
fn a_glob_with_no_glob_character_watches_one_file_and_a_missing_one_is_not_yet() {
    let project = TestProject::new();
    for glob in ["new/x.txt", "gone/*.txt"] {
        let answer = Cli::on(&project)
            .run(&["wait", "--for", "300ms", "--changed", glob])
            .exits(124)
            .answer();
        assert_eq!(answer["reason"], "deadline", "{answer}");
    }
    let later = written_later(&project, "new/x.txt", "x", Duration::from_millis(300));
    let answer = Cli::on(&project)
        .run(&["wait", "--for", "10s", "--changed", "new/x.txt"])
        .exits(0)
        .answer();
    later.join().unwrap();
    assert_eq!(answer["changed"], json!(["new/x.txt"]));
}

#[test]
fn an_ignored_file_does_not_count() {
    let project = TestProject::new()
        .file(".gitignore", "build/\n")
        .file("build/out.txt", "x");
    let since = [
        "wait",
        "--for",
        "0",
        "--changed",
        "**/*.txt",
        "--since",
        "0",
    ];
    Cli::on(&project).run(&since).exits(124);
    project.write("deep/er/kept.txt", "x");
    let answer = Cli::on(&project).run(&since).exits(0).answer();
    assert_eq!(answer["changed"], json!(["deep/er/kept.txt"]));
}

#[test]
fn a_line_that_matches_ends_the_wait() {
    let project = TestProject::new().file("log.txt", "starting\n");
    let later = written_later(
        &project,
        "log.txt",
        "ready in 30ms\n",
        Duration::from_millis(300),
    );
    let answer = Cli::on(&project)
        .run(&[
            "wait", "--for", "10s", "--match", "ready in", "--in", "log.txt",
        ])
        .exits(0)
        .answer();
    later.join().unwrap();
    assert_eq!(answer["reason"], "matched", "{answer}");
    assert_eq!(
        answer["matched"],
        json!({"path": "log.txt", "line": 2, "text": "ready in 30ms"})
    );
    assert!(answer["polls"].as_u64().unwrap() > 1, "{answer}");
    // A file that does not exist yet is "not yet".
    Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "200ms",
            "--match",
            "x",
            "--in",
            "later.log",
        ])
        .exits(124);
}

#[test]
fn a_path_outside_the_roots_is_refused_before_waiting() {
    let project = TestProject::new().file("log.txt", "ready\n");
    let outside = tempfile::tempdir().unwrap();
    let log = outside.path().join("server.log");
    std::fs::write(&log, "ready\n").unwrap();
    let began = Instant::now();
    for args in [
        &[
            "wait",
            "--for",
            "30s",
            "--match",
            "ready",
            "--in",
            log.to_str().unwrap(),
        ][..],
        &["wait", "--for", "30s", "--changed", "../x/*"],
        // Every condition is judged, the ones that would hold included.
        &[
            "wait",
            "--for",
            "30s",
            "--match",
            "ready",
            "--in",
            "log.txt",
            "--changed",
            "../x/*",
        ],
    ] {
        let refused = Cli::on(&project).run(args).exits(1);
        assert_eq!(refused.json()["code"], "outside_root", "{}", refused.stdout);
    }
    assert!(began.elapsed() < Duration::from_secs(15));
}

#[test]
fn a_bad_glob_or_pattern_is_invalid_input() {
    let project = TestProject::new().file("log.txt", "ready\n");
    for args in [
        &["wait", "--for", "30s", "--changed", "src/[a"][..],
        &["wait", "--for", "30s", "--match", "(", "--in", "log.txt"],
    ] {
        let refused = Cli::on(&project).run(args).exits(1);
        assert_eq!(
            refused.json()["code"],
            "invalid_input",
            "{}",
            refused.stdout
        );
    }
}

/// `--changed` reads its glob from the project's root wherever the wait is typed, and a nested project is its own: a
/// watch cycle names the project's files from any subdirectory. Typed with no `--path`, as a block types it.
#[test]
fn a_glob_is_read_from_the_project_root_from_any_directory() {
    let project = TestProject::new()
        .file("watched/a.txt", "x")
        .file("src/main.rs", "")
        .file(
            "inner/specforge.json",
            r#"{"name": "inner", "version": "0.1.0", "extensions": []}"#,
        )
        .file("inner/watched/b.txt", "x")
        .file("inner/src/lib.rs", "");
    let since = [
        "wait",
        "--for",
        "0",
        "--changed",
        "watched/*.txt",
        "--since",
        "0",
    ];
    let answer = Cli::on(&project)
        .in_dir(project.root().join("src"))
        .bare(&since)
        .exits(0)
        .answer();
    let changed = answer["changed"].as_array().expect("changed").clone();
    assert_eq!(changed.len(), 1, "{answer}");
    assert!(
        changed[0].as_str().unwrap().ends_with("/watched/a.txt")
            && !changed[0].as_str().unwrap().contains("inner"),
        "{answer}"
    );
    let nested = Cli::on(&project)
        .in_dir(project.root().join("inner/src"))
        .bare(&since)
        .exits(0)
        .answer();
    let changed = nested["changed"].as_array().expect("changed").clone();
    assert_eq!(changed.len(), 1, "{nested}");
    assert!(
        changed[0]
            .as_str()
            .unwrap()
            .ends_with("/inner/watched/b.txt"),
        "{nested}"
    );
}

/// `--retry-after` reads a `Retry-After` header's value in every form it takes: seconds, a fraction, an HTTP date (a
/// past one is no wait), each held to a day; it stands in place of `--for`.
#[test]
fn a_retry_after_value_is_read_in_every_form() {
    let project = TestProject::new();
    for (value, for_ms) in [
        ("0.2", 200),
        ("0", 0),
        ("Sun, 06 Nov 1994 08:49:37 GMT", 0),
        ("Sunday, 06-Nov-94 08:49:37 GMT", 0),
        ("Sun Nov  6 08:49:37 1994", 0),
    ] {
        let answer = Cli::on(&project)
            .run(&["wait", "--retry-after", value])
            .exits(0)
            .answer();
        assert_eq!(answer["reason"], "elapsed", "{value}: {answer}");
        assert_eq!(answer["for_ms"], for_ms, "{value}: {answer}");
    }
    for args in [
        &["wait", "--retry-after", "soon"][..],
        &["wait", "--retry-after", "5", "--for", "5"],
    ] {
        Cli::on(&project).run(args).exits(2);
    }
}

/// A condition that holds ends the wait before a probe starts: a file already written ends it with no request sent to
/// a server that would never answer.
#[test]
fn a_condition_that_holds_ends_the_wait_before_any_probe() {
    let project = TestProject::new().file("watched/a.txt", "x");
    let silent = Server::serve(|_, _, _| std::thread::sleep(Duration::from_secs(30)));
    let began = Instant::now();
    let answer = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "30s",
            "--url",
            &silent.base,
            "--changed",
            "watched/*.txt",
            "--since",
            "0",
        ])
        .exits(0)
        .answer();
    assert_eq!(answer["reason"], "changed", "{answer}");
    assert!(answer.get("url").is_none(), "{answer}");
    assert!(began.elapsed() < Duration::from_secs(5));
    assert!(silent.requests().is_empty(), "a probe was sent");
    // A bad pattern is judged before any check, a condition that holds or not.
    let refused = Cli::on(&project)
        .run(&[
            "wait",
            "--for",
            "30s",
            "--changed",
            "watched/*.txt",
            "--since",
            "0",
            "--match",
            "(",
            "--in",
            "log.txt",
        ])
        .exits(1);
    assert_eq!(
        refused.json()["code"],
        "invalid_input",
        "{}",
        refused.stdout
    );
}
