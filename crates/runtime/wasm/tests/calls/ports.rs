//! An effectful call reaches a port only through what it was lent (ADR 0335 D4). The guest is the in-process twin of
//! the sandbox probe's `ports` command, so what it is answered is what a component is answered; a request is what a
//! guest's Rust value can hold, which is more than the WIT lets through: every guard here is the session's.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_effects::testing::{FakeFs, FakeGit, FakeHttp, FakeProcess, MemoryConsent};
use specforge_effects::{
    AttachedSecret, EffectJournal, Effects, EffectsCeiling, EnvVar, Grant, HttpRequest, Ports,
    ProcessClass, ProcessRequest, Provenance, Roots,
};
use specforge_extension_sdk::host::{self, HostError};
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::Operation;
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{PortCall, PortSession, WasmCallResult, WasmRuntime};

pub(crate) const EXT: &str = "@ports/x";
const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// What a guest reports of a port's answer.
fn report<T>(answer: Result<T, HostError>, ok: impl FnOnce(T) -> Value) -> Value {
    match answer {
        Ok(value) => ok(value),
        Err(HostError::Denied(host::Denial::CallKind(why))) => {
            json!({"denied": "call_kind", "why": why})
        }
        Err(HostError::Denied(host::Denial::Scope(why))) => json!({"denied": "scope", "why": why}),
        Err(HostError::Failed(failure)) => json!({"failed": failure.class}),
    }
}

fn http_request(url: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        ..HttpRequest::default()
    }
}

/// The twin of the probe's `ports` command.
fn guest(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    let asked: Value = serde_json::from_slice(input).ok()?;
    let target = asked["target"].as_str().unwrap_or("").to_string();
    let answer = match export {
        "cmd__ports" => match asked["port"].as_str()? {
            "http" => report(host::http::send(&http_request(&target)), |r| {
                json!({
                    "ok": r.status,
                    "bytes": r.body.len(),
                    "truncated": r.truncated,
                    "headers": r.headers,
                })
            }),
            "fs" => report(
                host::fs::stat("project", &target),
                |e| json!({"ok": e.is_some()}),
            ),
            "read" => report(
                host::fs::read("project", &target, 0),
                |c| json!({"ok": String::from_utf8_lossy(&c.bytes), "truncated": c.truncated}),
            ),
            "process" => report(
                host::process::run(&ProcessRequest {
                    argv: vec![target],
                    ..ProcessRequest::default()
                }),
                |o| json!({"ok": o.exit_code, "truncated": o.truncated}),
            ),
            "git" => report(host::git::commit(&target), |sha| json!({"ok": sha})),
            "blob" => report(
                host::git::blobs(SHA, std::slice::from_ref(&target)),
                |b| json!({"ok": b.blobs.len()}),
            ),
            _ => return None,
        },
        _ => return None,
    };
    Some(Ok(answer.to_string().into_bytes()))
}

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.secret("api-token", "ACME_API_VALUE", "the API's token");
    c.capability(
        Capability::http("api")
            .hosts(&["example.test"])
            .schemes(&["https"])
            .secret_header("Authorization", "api-token", Some("Bearer"))
            .why("call the API"),
    );
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Stat, FsAccess::Read, FsAccess::Walk])
            .why("read the project"),
    );
    c.capability(Capability::git("history").why("read the history"));
    c.capability(
        Capability::process("tests")
            .argv(&["cargo"])
            .env(&["MODE", "ACME_API_VALUE"])
            .why("run the tests"),
    );
    c
}

pub(crate) fn runtime() -> InProcessRuntime {
    InProcessRuntime::new().with_handler(extension, guest)
}

/// Fakes the test holds, and the grant over them, every approval given.
pub(crate) struct Lent {
    http: Arc<FakeHttp>,
    process: Arc<FakeProcess>,
    pub(crate) call: PortCall,
}

fn lent_over(http: FakeHttp, process: FakeProcess, fs: FakeFs, git: FakeGit) -> Lent {
    let (http, process) = (Arc::new(http), Arc::new(process));
    let ports = Ports {
        process: process.clone(),
        http: http.clone(),
        fs: Arc::new(fs),
        git: Arc::new(git),
    };
    let roots = Roots::new("/project", "/project/spec");
    let grant = Grant::decide(
        &extension().declaration(),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    let secrets = vec!["ACME_API_VALUE".to_string()];
    let call = PortCall::online(&ports, EffectJournal::detached(), roots, grant, secrets);
    Lent {
        http,
        process,
        call,
    }
}

pub(crate) fn lent() -> Lent {
    lent_over(
        FakeHttp::new().route("https://example.test/a", 200, "answer"),
        FakeProcess::new().answering("cargo", ProcessClass::Exited(0), "built"),
        FakeFs::new().with_file("/project/a.md", "a"),
        FakeGit::none(),
    )
}

/// What the guest reports of a command calling `port` on `target`.
fn command(runtime: &dyn WasmRuntime, port: &str, target: &str, lent: Option<PortCall>) -> Value {
    let input = json!({"port": port, "target": target}).to_string();
    answered(runtime.call_with_ports(
        EXT,
        "cmd__ports",
        input.as_bytes(),
        Operation::Command,
        lent,
    ))
}

fn answered(result: WasmCallResult) -> Value {
    match result {
        WasmCallResult::Ok(bytes) => {
            serde_json::from_slice(&bytes).expect("the guest reports JSON")
        }
        WasmCallResult::Trap(trap) => panic!("the guest trapped: {trap:?}"),
    }
}

#[specforge_test_macros::test(
    behavior = "grant_a_host_port_call",
    verify = "a command's port call inside its grant reaches the port"
)]
fn a_commands_port_call_inside_its_grant_reaches_the_port() {
    let fakes = lent();
    let answer = command(
        &runtime(),
        "http",
        "https://example.test/a",
        Some(fakes.call.clone()),
    );
    assert_eq!(answer["ok"], 200, "{answer}");
    assert_eq!(answer["bytes"], 6, "{answer}");
    assert_eq!(fakes.http.requests().len(), 1);
    assert!(fakes.call.take_denials().is_empty());
}

#[specforge_test_macros::test(
    behavior = "grant_a_host_port_call",
    verify = "a port call outside the grant is answered denied(scope) and reported as E090"
)]
fn a_port_call_outside_the_grant_is_denied_and_reported() {
    let fakes = lent();
    let answer = command(
        &runtime(),
        "http",
        "https://elsewhere.test/a",
        Some(fakes.call.clone()),
    );
    assert_eq!(answer["denied"], "scope", "{answer}");
    assert!(fakes.http.requests().is_empty(), "the fake saw nothing");
    let denials = fakes.call.take_denials();
    assert_eq!(denials.len(), 1, "{denials:?}");
    assert_eq!(denials[0].code, "E090");
    assert!(denials[0].message.contains(EXT), "{}", denials[0].message);
}

/// A guest in a loop is told each time, and costs the host a bounded list: the first denials, each cut to a fixed
/// length, then one line saying how many more.
#[test]
fn a_loop_of_huge_denied_requests_keeps_a_bounded_list() {
    let fakes = lent();
    let mut session = PortSession::effectful(
        EXT,
        Operation::Command,
        Some(fakes.call.clone()),
        specforge_wasm::Limits::CEILING,
    );
    // Multi-byte, so the cut has a boundary to respect.
    let huge = format!("https://elsewhere.test/{}", "é".repeat(512 * 1024));
    for _ in 0..1000 {
        let answer = session.http(http_request(&huge));
        assert!(matches!(answer, Err(HostError::Denied(_))), "{answer:?}");
    }
    let denials = fakes.call.take_denials();
    assert_eq!(denials.len(), 33, "32 kept, then one line");
    assert!(
        denials.iter().all(|denial| denial.code == "E090"),
        "{denials:?}"
    );
    let kept: usize = denials.iter().map(|denial| denial.message.len()).sum();
    assert!(kept < 33 * 1024, "the requests' text is cut: {kept} bytes");
    assert!(
        denials[32].message.contains("968 more"),
        "{}",
        denials[32].message
    );
    assert!(fakes.call.take_denials().is_empty(), "taken once");
}

#[specforge_test_macros::test(
    behavior = "link_host_ports",
    verify = "a pure call that calls a port is answered denied(call-kind) and no port runs"
)]
fn a_pure_call_that_calls_a_port_is_denied_its_call_kind() {
    let fakes = lent();
    let runtime = runtime();
    let input = json!({"port": "http", "target": "https://example.test/a"}).to_string();
    for answer in [
        runtime.call_export(EXT, "cmd__ports", input.as_bytes()),
        runtime.call_with_ports(
            EXT,
            "cmd__ports",
            input.as_bytes(),
            Operation::Pass,
            Some(fakes.call.clone()),
        ),
    ] {
        let answer = answered(answer);
        assert_eq!(answer["denied"], "call_kind", "{answer}");
    }
    assert!(fakes.http.requests().is_empty(), "no port ran");
    assert!(
        fakes.call.take_denials().is_empty(),
        "a kind that may use no port never reached a grant"
    );
}

#[test]
fn a_call_lent_nothing_is_denied_scope() {
    let answer = command(&runtime(), "http", "https://example.test/a", None);
    assert_eq!(answer["denied"], "scope", "{answer}");
}

#[test]
fn a_command_is_kept_from_the_process_port_and_a_collector_reaches_it() {
    let fakes = lent();
    let runtime = runtime();
    let answer = command(&runtime, "process", "cargo", Some(fakes.call.clone()));
    assert_eq!(answer["denied"], "call_kind", "{answer}");
    assert!(fakes.process.requests().is_empty());
    assert_eq!(fakes.call.take_denials().len(), 1, "E090: it was asked");

    let input = json!({"port": "process", "target": "cargo"}).to_string();
    let answer = answered(runtime.call_with_ports(
        EXT,
        "cmd__ports",
        input.as_bytes(),
        Operation::Collect,
        Some(fakes.call.clone()),
    ));
    assert_eq!(answer["ok"], 0, "{answer}");
    assert_eq!(fakes.process.requests().len(), 1);
}

#[specforge_test_macros::test(
    behavior = "grant_a_host_port_call",
    verify = "a port's answer is held to a quarter of the guest's memory and says it was truncated"
)]
fn a_ports_answer_is_held_to_a_quarter_of_the_guests_memory() {
    let fakes = lent_over(
        FakeHttp::new().route("https://example.test/big", 200, vec![0u8; 8 << 20]),
        FakeProcess::new(),
        FakeFs::new(),
        FakeGit::none(),
    );
    let runtime = runtime();
    runtime.apply_limits(
        EXT,
        specforge_wasm::Limits {
            execution_ms: 1_000,
            memory_mb: 16,
        },
    );
    let answer = command(
        &runtime,
        "http",
        "https://example.test/big",
        Some(fakes.call),
    );
    assert_eq!(answer["ok"], 200, "{answer}");
    assert_eq!(answer["bytes"], 4 << 20, "{answer}");
    assert_eq!(answer["truncated"], true, "{answer}");
    assert_eq!(fakes.http.requests()[0].body_cap, 4 << 20);
}

// ── What only the host may set (ADR 0288) ───────────────────────────────────

/// A session over `fakes`, as a guest's Rust value reaches it in process.
fn session(fakes: &Lent, kind: Operation) -> specforge_wasm::PortSession {
    specforge_wasm::PortSession::effectful(
        EXT,
        kind,
        Some(fakes.call.clone()),
        specforge_wasm::Limits::CEILING,
    )
}

#[specforge_test_macros::test(
    behavior = "grant_a_host_port_call",
    verify = "a secret the host attaches never shows in what a guest is answered"
)]
fn a_secret_the_host_attaches_never_shows_in_what_a_guest_is_answered() {
    let fakes = lent_over(
        FakeHttp::new().route_with(
            "https://example.test/a",
            200,
            vec![("x-kept", "kept")],
            "answer",
        ),
        FakeProcess::new(),
        FakeFs::new(),
        FakeGit::none(),
    );
    // A guest that names a secret of its own, and asks to keep the header the host attaches.
    let asked = HttpRequest {
        secret: Some(AttachedSecret {
            env: "OTHER_TOKEN".to_string(),
            ..AttachedSecret::default()
        }),
        keep_headers: vec!["authorization".to_string(), "x-kept".to_string()],
        ..http_request("https://example.test/a")
    };
    let answer = session(&fakes, Operation::Command)
        .http(asked)
        .expect("inside its grant");

    let sent = &fakes.http.requests()[0];
    let attached = sent.secret.as_ref().expect("the grant attaches its own");
    assert_eq!(attached.env, "ACME_API_VALUE", "never the guest's");
    assert_eq!(attached.header, "Authorization");
    // Everything the guest is answered, spelled out: no field of it names the secret.
    let shown = format!("{answer:?}");
    assert!(!shown.contains("ACME_API_VALUE"), "{shown}");
    assert!(!shown.contains("Bearer"), "{shown}");
    assert_eq!(answer.headers, [("x-kept".to_string(), "kept".to_string())]);
}

#[test]
fn a_redirect_to_a_host_outside_the_grant_never_carries_the_guest_there() {
    let fakes = lent_over(
        FakeHttp::new()
            .redirect("https://example.test/a", "https://elsewhere.test/b")
            .route("https://elsewhere.test/b", 200, "theirs"),
        FakeProcess::new(),
        FakeFs::new(),
        FakeGit::none(),
    );
    let refused = session(&fakes, Operation::Command)
        .http(HttpRequest {
            max_redirects: 3,
            ..http_request("https://example.test/a")
        })
        .expect_err("a hop the grant does not permit stops the request");
    assert!(
        matches!(&refused, HostError::Denied(host::Denial::Scope(_))),
        "{refused:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "grant_a_host_port_call",
    verify = "a program a guest runs inherits no declared secret and cannot set its PATH or loader"
)]
fn a_program_a_guest_runs_inherits_no_declared_secret_and_cannot_set_its_path_or_loader() {
    let fakes = lent();
    let run = |env: Vec<EnvVar>| {
        session(&fakes, Operation::Collect).process_run(ProcessRequest {
            argv: vec!["cargo".to_string()],
            env,
            ..ProcessRequest::default()
        })
    };
    // Removed last, so a guest's own value for it does not stand either.
    run(vec![
        EnvVar::set("ACME_API_VALUE", "guest"),
        EnvVar::set("MODE", "x"),
    ])
    .expect("granted");
    let env = &fakes.process.requests()[0].env;
    assert_eq!(env.first(), Some(&EnvVar::set("ACME_API_VALUE", "guest")));
    assert_eq!(env.last(), Some(&EnvVar::removed("ACME_API_VALUE")));
    assert_eq!(
        fakes.process.requests()[0].inherit.as_deref(),
        Some(["MODE".to_string(), "ACME_API_VALUE".to_string()].as_slice()),
        "the host says what it inherits"
    );

    for name in [
        "PATH",
        "Path",
        "PATHEXT",
        "LD_PRELOAD",
        "DYLD_INSERT_LIBRARIES",
        "BASH_ENV",
    ] {
        let refused = run(vec![EnvVar::set(name, "/tmp/evil")]);
        assert!(
            matches!(refused, Err(HostError::Denied(host::Denial::Scope(_)))),
            "{name}: {refused:?}"
        );
    }
    assert_eq!(fakes.process.requests().len(), 1, "no other program ran");
}

#[specforge_test_macros::test(
    behavior = "grant_a_host_port_call",
    verify = "a guest cannot open the private-address opt-in or read a withheld file at a commit"
)]
fn a_guest_cannot_open_the_private_address_opt_in_or_read_a_withheld_file_at_a_commit() {
    let dir = std::env::temp_dir();
    let fakes = lent_over(
        FakeHttp::new().route("https://example.test/a", 200, "answer"),
        FakeProcess::new(),
        FakeFs::new(),
        FakeGit::repo(&dir, SHA, Vec::new()),
    );
    session(&fakes, Operation::Command)
        .http(HttpRequest {
            allow_private: true,
            ..http_request("https://example.test/a")
        })
        .expect("inside its grant");
    assert!(!fakes.http.requests()[0].allow_private);

    for withheld in [".env", "config/.env.production", ".git/config"] {
        let refused = session(&fakes, Operation::Command).git_blobs(SHA, &[withheld.to_string()]);
        assert!(
            matches!(refused, Err(HostError::Denied(host::Denial::Scope(_)))),
            "{withheld}: {refused:?}"
        );
    }
}

#[test]
fn a_guest_cannot_name_another_root_inside_a_path() {
    let fakes = lent();
    let refused = session(&fakes, Operation::Command).fs_stat("project", "<home>/.ssh/id_rsa");
    assert!(
        matches!(refused, Err(HostError::Denied(host::Denial::Scope(_)))),
        "{refused:?}"
    );
}

#[test]
fn what_a_guest_asks_of_git_cannot_be_an_option_or_climb_out() {
    let dir = std::env::temp_dir();
    let fakes = lent_over(
        FakeHttp::new(),
        FakeProcess::new(),
        FakeFs::new(),
        FakeGit::repo(&dir, SHA, Vec::new()),
    );
    let mut port = session(&fakes, Operation::Command);
    assert_eq!(
        port.git_commit("HEAD").expect("a name"),
        Some(SHA.to_string())
    );
    for revision in ["", "--upload-pack=x", "a b"] {
        assert!(port.git_commit(revision).is_err(), "{revision:?}");
    }
    assert!(port.git_merge_base("--all", SHA).is_err());
    for dir in ["../up", "/etc"] {
        assert!(port.git_tree(SHA, dir).is_err(), "{dir}");
        assert!(port.git_sizes(SHA, &[dir.to_string()]).is_err(), "{dir}");
    }
}

#[test]
fn a_replay_with_no_record_answers_not_in_journal() {
    let mut fakes = lent();
    fakes.call.effects = Arc::new(Effects::replay(EffectJournal::detached()));
    let refused = session(&fakes, Operation::Command)
        .http(http_request("https://example.test/a"))
        .expect_err("nothing is recorded");
    assert!(
        matches!(&refused, HostError::Failed(f) if f.class == "not_in_journal"),
        "{refused:?}"
    );
    assert!(fakes.http.requests().is_empty(), "replay never sends");
}

#[test]
fn a_git_question_needs_a_git_capability() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.capability(Capability::fs("files").why("read the project"));
    let roots = Roots::new("/project", "/project/spec");
    let grant = Grant::decide(
        &c.declaration(),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    assert!(grant.permits_git(Operation::Command, "branch").is_err());
    assert!(grant.permits_git(Operation::Pass, "branch").is_err());
}
