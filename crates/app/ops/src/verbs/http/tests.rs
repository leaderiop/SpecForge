use std::sync::Arc;

use specforge_effects::Ports;
use specforge_effects::testing::{FakeHttp, MemoryConsent, ports};
use specforge_project::testing::TestProject;
use specforge_protocol_types::{HandshakeResponse, SecretDescriptor};

use super::*;

/// `@acme/tools`, declaring the secret `token` read from `ACME_TOKEN`.
fn acme() -> ExtensionDeclaration {
    ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: "@acme/tools".to_string(),
            ..HandshakeResponse::default()
        },
        settings: vec![SettingsItem::Secret(SecretDescriptor {
            name: "token".to_string(),
            env: "ACME_TOKEN".to_string(),
            description: None,
        })],
        ..ExtensionDeclaration::default()
    }
}

fn get(url: &str) -> Ask {
    Ask {
        method: "GET".to_string(),
        url: url.to_string(),
        headers: Vec::new(),
        body: None,
        secret: None,
        timeout: Duration::from_secs(5),
        max_bytes: ANSWER_BYTES,
    }
}

/// The verbs of `project` with `http` as their port.
fn verbs(project: &TestProject, http: &Arc<FakeHttp>) -> Verbs {
    let ports = Ports {
        http: Arc::clone(http) as _,
        ..ports()
    };
    Verbs::open(
        &project.project_root(),
        project.root(),
        &ports,
        &MemoryConsent::approving_everything(),
    )
}

/// Every byte the project's effects journal holds, as text.
fn journal(project: &TestProject) -> String {
    let mut text = String::new();
    let mut dirs = vec![project.root().join(".specforge")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                text.push_str(&String::from_utf8_lossy(
                    &std::fs::read(&path).unwrap_or_default(),
                ));
            }
        }
    }
    text
}

#[test]
fn a_secret_is_named_by_its_extension_and_rides_its_https_host_alone() {
    let attached = secret(&[acme()], "@acme/tools:token", "https://api.acme.dev/v1/me").unwrap();
    assert_eq!(
        attached,
        AttachedSecret {
            header: "Authorization".to_string(),
            env: "ACME_TOKEN".to_string(),
            scheme: Some("Bearer".to_string()),
            hosts: vec!["api.acme.dev".to_string()],
        }
    );
    for (named, url, says) in [
        ("token", "https://api.acme.dev/", "names no secret"),
        (
            "@acme/tools:token",
            "http://api.acme.dev/",
            "only an https request",
        ),
        (
            "@other/x:token",
            "https://api.acme.dev/",
            "loads no extension '@other/x'",
        ),
        (
            "@acme/tools:nope",
            "https://api.acme.dev/",
            "it declares token",
        ),
    ] {
        let error = secret(&[acme()], named, url).unwrap_err();
        assert!(
            error.message.contains(says),
            "{named} {url}: {}",
            error.message
        );
    }
}

#[test]
fn the_secret_is_attached_as_the_request_leaves_and_no_answer_or_record_holds_it() {
    let project = TestProject::new();
    let url = "https://api.acme.dev/v1/me";
    let http = Arc::new(FakeHttp::new().route(url, 200, r#"{"login":"me"}"#));
    let ask = Ask {
        secret: Some(secret(&[acme()], "@acme/tools:token", url).unwrap()),
        ..get(url)
    };
    let answer = request(&verbs(&project, &http), &ask, EffectsMode::Online).unwrap();
    // The port gets the variable's name to read as the request leaves; the answer, only the header's name.
    let sent = http.requests();
    assert_eq!(
        sent[0].secret.as_ref().map(|s| s.env.as_str()),
        Some("ACME_TOKEN")
    );
    assert_eq!(
        answer.answer.secret_header.as_deref(),
        Some("Authorization")
    );
    assert_eq!(answer.answer.json, Some(serde_json::json!({"login": "me"})));
    let shown = serde_json::to_string(&answer).unwrap();
    assert!(
        !shown.contains("ACME_TOKEN") && !shown.contains("Bearer"),
        "{shown}"
    );
    let kept = journal(&project);
    assert!(kept.contains("api.acme.dev"), "the request is recorded");
    assert!(
        !kept.contains("ACME_TOKEN") && !kept.contains("Bearer"),
        "{kept}"
    );
}

#[test]
fn a_typed_credential_header_is_refused_before_anything_is_sent() {
    let project = TestProject::new();
    let http = Arc::new(FakeHttp::new());
    for header in [
        "Authorization: Bearer s3cr3t",
        "cookie: id=1",
        "Proxy-Authorization: x",
    ] {
        let ask = Ask {
            headers: vec![header.to_string()],
            ..get("https://api.acme.dev/")
        };
        let error = request(&verbs(&project, &http), &ask, EffectsMode::Online).unwrap_err();
        assert!(
            error.message.contains("credential"),
            "{header}: {}",
            error.message
        );
        assert!(!error.message.contains("s3cr3t"), "{}", error.message);
    }
    assert!(http.requests().is_empty());
}

#[test]
fn a_redirect_the_ceiling_leaves_no_host_for_is_refused_with_the_ceilings_words() {
    let project = TestProject::new().config(|config| {
        config["effects"] = serde_json::json!({"http": {"hosts": ["a.example"]}});
    });
    let http = Arc::new(
        FakeHttp::new()
            .redirect("https://a.example/x", "https://b.example/y")
            .route("https://b.example/y", 200, "elsewhere"),
    );
    let verbs = verbs(&project, &http);
    let error = request(&verbs, &get("https://a.example/x"), EffectsMode::Online).unwrap_err();
    assert_eq!(error.code, "E090", "{}", error.message);
    assert!(error.message.contains("b.example"), "{}", error.message);
    // The first request is judged too.
    let error = request(&verbs, &get("https://b.example/y"), EffectsMode::Online).unwrap_err();
    assert_eq!(error.code, "E090", "{}", error.message);
    assert_eq!(http.requests().len(), 1, "b.example is never asked");
}

#[test]
fn cached_answers_a_get_again_and_sends_a_post_each_time() {
    let project = TestProject::new();
    let url = "https://api.acme.dev/v1/items";
    let http = Arc::new(FakeHttp::new().route(url, 201, r#"{"id":1}"#));
    let verbs = verbs(&project, &http);
    let first = request(&verbs, &get(url), EffectsMode::Cached).unwrap();
    let again = request(&verbs, &get(url), EffectsMode::Cached).unwrap();
    assert!(!first.effect.from_journal && again.effect.from_journal);
    assert_eq!(again.answer.status, 201);
    assert_eq!(http.requests().len(), 1);
    let post = Ask {
        method: "POST".to_string(),
        body: Some(Body::Json {
            object: serde_json::json!({"title": "x"})
                .as_object()
                .unwrap()
                .clone(),
            files: Vec::new(),
        }),
        ..get(url)
    };
    for _ in 0..2 {
        assert!(
            !request(&verbs, &post, EffectsMode::Cached)
                .unwrap()
                .effect
                .from_journal
        );
    }
    assert_eq!(http.requests().len(), 3);
    let sent = &http.requests()[1];
    assert_eq!(sent.body.as_deref(), Some(br#"{"title":"x"}"#.as_slice()));
    assert!(
        sent.headers
            .iter()
            .any(|h| h.name == "Content-Type" && h.value == "application/json")
    );
    // Replay answers both from the journal, and refuses what was never sent.
    assert!(
        request(&verbs, &post, EffectsMode::Replay)
            .unwrap()
            .effect
            .from_journal
    );
    let never = request(
        &verbs,
        &get("https://api.acme.dev/other"),
        EffectsMode::Replay,
    )
    .unwrap_err();
    assert_eq!(never.code, NOT_IN_JOURNAL);
}

#[test]
fn a_body_past_the_cap_is_cut_and_read_as_no_json() {
    let project = TestProject::new();
    let url = "https://api.acme.dev/big";
    let http = Arc::new(FakeHttp::new().route(url, 200, r#"{"a":"0123456789"}"#));
    let ask = Ask {
        max_bytes: 5,
        ..get(url)
    };
    let answer = request(&verbs(&project, &http), &ask, EffectsMode::Online).unwrap();
    assert!(answer.answer.truncated);
    assert_eq!(answer.answer.body, r#"{"a":"#);
    assert_eq!(answer.answer.json, None);
}

#[test]
fn an_unset_secret_is_refused_by_its_variables_name() {
    let attached = secret(&[acme()], "@acme/tools:token", "https://api.acme.dev/").unwrap();
    for value in [None, Some(String::new())] {
        let refused = secret_is_set(&attached, |_| value.clone()).unwrap_err();
        assert_eq!(refused.code, "secret_unset");
        assert!(
            refused.message.contains("ACME_TOKEN"),
            "{}",
            refused.message
        );
    }
    assert!(secret_is_set(&attached, |_| Some("t".into())).is_ok());
}
