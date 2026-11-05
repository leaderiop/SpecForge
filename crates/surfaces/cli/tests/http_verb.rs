//! `specforge http` (ADR 0339): the `Http` port for people and agents, against a one-route server on this machine.

use serde_json::json;

use crate::support::server::Server;
use crate::support::{Cli, TestProject};

const MODELS: &str = r#"{"models":["a","b"]}"#;

#[test]
fn a_json_answer_is_read_whole_and_picked() {
    let project = TestProject::new();
    let server = Server::answering(200, "application/json", MODELS);
    let url = format!("{}/v1/models", server.base);
    let answer = Cli::on(&project)
        .run(&["http", "get", &url])
        .exits(0)
        .answer();
    assert_eq!(
        answer,
        json!({
            "method": "GET",
            "url": url,
            "ok": true,
            "status": 200,
            "outcome": "ok",
            "final_url": url,
            "headers": {"content-type": "application/json"},
            "content_type": "application/json",
            "bytes": MODELS.len(),
            "body": MODELS,
            "json": {"models": ["a", "b"]},
            "truncated": false,
        })
    );
    let picked = Cli::on(&project)
        .run(&["http", "get", &url, "--pick", ".json.models[0]"])
        .exits(0);
    assert_eq!(picked.stdout, "a\n");
    assert!(server.requests()[0].starts_with("GET /v1/models HTTP/1.1"));
}

#[test]
fn a_status_outside_2xx_is_an_answer_not_a_failure() {
    let project = TestProject::new();
    let server = Server::answering(404, "text/plain", "no such thing");
    let answer = Cli::on(&project)
        .run(&["http", "get", &format!("{}/x", server.base)])
        .exits(0)
        .answer();
    assert_eq!(
        (answer["ok"].clone(), answer["status"].clone()),
        (json!(false), json!(404))
    );
    assert_eq!(answer["body"], "no such thing");
    assert!(answer.get("json").is_none(), "{answer}");
}

#[test]
fn a_server_that_does_not_answer_is_a_refusal() {
    let project = TestProject::new();
    let url = format!("http://127.0.0.1:{}/", Server::closed_port());
    let refused = Cli::on(&project).run(&["http", "get", &url]).exits(1);
    assert_eq!(refused.json()["code"], "unreachable", "{}", refused.stdout);
    // Under --pick the refusal is on stderr, and a shell reading the value reads nothing.
    let picked = Cli::on(&project)
        .run(&["http", "get", &url, "--pick", ".ok"])
        .exits(1);
    assert_eq!(picked.stdout, "");
    assert!(
        picked.stderr.contains("error[unreachable]"),
        "{}",
        picked.stderr
    );
}

#[test]
fn a_ceiling_with_http_off_refuses_before_connecting() {
    let project = TestProject::new().config(|config| {
        config["effects"] = json!({"http": "off"});
    });
    let server = Server::answering(200, "application/json", MODELS);
    let refused = Cli::on(&project)
        .run(&["http", "get", &server.base])
        .exits(1);
    assert_eq!(refused.json()["code"], "E090", "{}", refused.stdout);
    assert!(server.requests().is_empty(), "the server saw a request");
}

#[test]
fn a_body_is_built_from_fields() {
    let project = TestProject::new();
    let server = Server::answering(201, "application/json", "{}");
    Cli::on(&project)
        .run(&[
            "http",
            "post",
            &server.base,
            "--field",
            "title=x",
            "--field-json",
            "n=1",
            "-H",
            "X-Trace: t1",
        ])
        .exits(0);
    let seen = &server.requests()[0];
    assert!(seen.starts_with("POST / HTTP/1.1"), "{seen}");
    let lower = seen.to_ascii_lowercase();
    assert!(lower.contains("content-type: application/json"), "{seen}");
    assert!(lower.contains("x-trace: t1"), "{seen}");
    assert!(seen.ends_with(r#"{"n":1,"title":"x"}"#), "{seen}");
    let typed = Cli::on(&project)
        .run(&[
            "http",
            "get",
            &server.base,
            "-H",
            "Authorization: Bearer s3cr3t",
        ])
        .exits(1);
    assert!(!typed.stdout.contains("s3cr3t"), "{}", typed.stdout);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_long_text_field_is_read_from_a_file_in_the_project() {
    let project = TestProject::new().file("pr.md", "Says \"why\".\n\n- a\n");
    let server = Server::answering(201, "application/json", "{}");
    Cli::on(&project)
        .run(&[
            "http",
            "post",
            &server.base,
            "--field",
            "title=x",
            "--field-file",
            "body=pr.md",
        ])
        .exits(0);
    let seen = &server.requests()[0];
    assert!(
        seen.ends_with(r#"{"body":"Says \"why\".\n\n- a\n","title":"x"}"#),
        "{seen}"
    );
    // a file outside the project's roots is never read, and nothing is sent
    let outside = tempfile::NamedTempFile::new().unwrap();
    let field = format!("body={}", outside.path().display());
    Cli::on(&project)
        .run(&["http", "post", &server.base, "--field-file", &field])
        .exits(1);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_secret_is_declared_rides_https_alone_and_is_never_shown() {
    let project = TestProject::new();
    let server = Server::answering(200, "application/json", "{}");
    for (url, says) in [
        (server.base.clone(), "only an https request"),
        (
            "https://api.example.com/".to_string(),
            "loads no extension '@acme/tools'",
        ),
    ] {
        let refused = Cli::on(&project)
            .env("ACME_TOKEN", "s3cr3t-value")
            .run(&["http", "get", &url, "--secret", "@acme/tools:token"])
            .exits(1);
        let message = refused.json()["error"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        assert!(message.contains(says), "{message}");
        assert!(!refused.stdout.contains("s3cr3t") && !refused.stderr.contains("s3cr3t"));
    }
    assert!(server.requests().is_empty());
}

#[test]
fn cached_answers_a_get_from_the_journal_and_replay_refuses_what_was_never_sent() {
    let project = TestProject::new();
    let server = Server::answering(200, "application/json", MODELS);
    let url = format!("{}/v1/models", server.base);
    let never = Cli::on(&project)
        .run(&["http", "get", &url, "--effects", "replay"])
        .exits(1);
    assert_eq!(never.json()["code"], "not_in_journal", "{}", never.stdout);
    let ask = [
        "http",
        "get",
        url.as_str(),
        "--effects",
        "cached",
        "--pick",
        ".effect.from_journal",
    ];
    assert_eq!(Cli::on(&project).run(&ask).exits(0).stdout, "false\n");
    assert_eq!(Cli::on(&project).run(&ask).exits(0).stdout, "true\n");
    let replayed = Cli::on(&project)
        .run(&[
            "http",
            "get",
            &url,
            "--effects",
            "replay",
            "--pick",
            ".json.models[1]",
        ])
        .exits(0);
    assert_eq!(replayed.stdout, "b\n");
    assert_eq!(server.requests().len(), 1, "the server answered once");
}
