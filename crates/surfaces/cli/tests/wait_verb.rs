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
