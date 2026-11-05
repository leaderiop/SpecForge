//! What a lent call's effects mode does (ADR 0335 D4, ADR 0337 D4): a record is its asker's, a fresh one answers only a
//! `GET`, and `cached` and `replay` hold no port.

use std::sync::Arc;

use serde_json::Map;
use specforge_effects::testing::{self, FakeHttp, MemoryConsent};
use specforge_effects::{
    Answer, EffectJournal, EffectRequest, Effects, EffectsCeiling, EffectsMode, Grant, HttpRequest,
    Outcome, OutcomeClass, Ports, Provenance, Roots, UnixMillis,
};
use specforge_extension_sdk::host;
use specforge_extension_sdk::prelude::*;
use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::{Surface, execute};
use specforge_ops::extension::command::{CommandFormat, ExtensionCommands};
use specforge_ops::view::ProjectView;
use specforge_ops::{Caller, Containment};
use specforge_project::testing::TestProject;
use specforge_protocol_types::{HostError, Operation};
use specforge_wasm::testing::InProcessRuntime;
use specforge_wasm::{Limits, PortCall, PortSession};
use tempfile::TempDir;

const URL: &str = "https://example.test/a";

fn declaration(name: &str) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    c.capability(
        Capability::http("api")
            .hosts(&["example.test"])
            .schemes(&["https"])
            .why("call the API"),
    );
    c
}

fn request(method: &str) -> HttpRequest {
    HttpRequest {
        method: method.to_string(),
        url: URL.to_string(),
        ..HttpRequest::default()
    }
}

/// A project on disk, whose journal the runs share, and the fake its live runs reach.
struct Project {
    dir: TempDir,
    http: Arc<FakeHttp>,
    ports: Ports,
}

impl Project {
    fn new() -> Self {
        let http = Arc::new(FakeHttp::new().route(URL, 200, "answer"));
        Project {
            dir: TempDir::new().unwrap(),
            ports: Ports {
                http: http.clone(),
                ..testing::ports()
            },
            http,
        }
    }

    fn journal(&self) -> EffectJournal {
        EffectJournal::at(self.dir.path())
    }

    /// `extension`'s session, its effects `effects`.
    fn session(&self, extension: &str, effects: Effects) -> PortSession {
        let roots = Roots::new(self.dir.path(), self.dir.path());
        let grant = Grant::decide(
            &declaration(extension).declaration(),
            &Provenance::third_party(b"x"),
            &EffectsCeiling::default(),
            &roots,
            &MemoryConsent::approving_everything(),
        );
        let call = PortCall::new(effects, &self.ports, roots, grant, Vec::new());
        PortSession::effectful(extension, Operation::Command, Some(call), Limits::CEILING)
    }

    fn online(&self, extension: &str) -> PortSession {
        self.session(
            extension,
            Effects::online(self.ports.clone(), self.journal()),
        )
    }

    fn auto(&self, extension: &str) -> PortSession {
        self.session(
            extension,
            Effects::auto(self.ports.clone(), Vec::new(), self.journal()),
        )
    }

    fn cached(&self, extension: &str) -> PortSession {
        self.session(extension, Effects::cached(self.journal()))
    }

    fn replay(&self, extension: &str) -> PortSession {
        self.session(extension, Effects::replay(self.journal()))
    }
}

fn failed_class<T: std::fmt::Debug>(answer: Result<T, HostError>) -> String {
    match answer {
        Err(HostError::Failed(failure)) => failure.class,
        other => panic!("not a failure: {other:?}"),
    }
}

#[test]
fn another_extension_never_reads_a_lent_record_in_cached() {
    let project = Project::new();
    let live = project.online("@t/a").http(request("GET")).unwrap();
    assert_eq!(live.body, b"answer");

    let theirs = project.cached("@t/b").http(request("GET"));
    assert_eq!(failed_class(theirs), "not_performed");
    let own = project.cached("@t/a").http(request("GET")).unwrap();
    assert_eq!(own.body, b"answer");
    assert_eq!(
        project.http.requests().len(),
        1,
        "only the live run reached it"
    );
}

/// A replayed answer is the live one: its status, kept headers, final URL and body, on a redirect that ended in a 404.
#[test]
fn a_replay_answers_what_the_live_call_did() {
    let target = "https://example.test/moved";
    let http = Arc::new(FakeHttp::new().redirect(URL, target).route_with(
        target,
        404,
        vec![("etag", "abc")],
        "not here",
    ));
    let mut project = Project::new();
    project.ports.http = http;
    let asked = || HttpRequest {
        keep_headers: vec!["etag".to_string()],
        max_redirects: 2,
        ..request("GET")
    };

    let live = project.online("@t/a").http(asked()).unwrap();
    assert_eq!(live.status, 404);
    assert_eq!(live.final_url, target);
    assert_eq!(live.headers, [("etag".to_string(), "abc".to_string())]);

    assert_eq!(project.replay("@t/a").http(asked()).unwrap(), live);
    assert_eq!(project.cached("@t/a").http(asked()).unwrap(), live);
}

/// A response header that can carry a credential reaches the guest live, and is never written to the journal.
#[test]
fn the_journal_keeps_no_sensitive_response_header() {
    let http = Arc::new(FakeHttp::new().route_with(
        URL,
        200,
        vec![("etag", "abc"), ("set-cookie", "session=SECRET-VALUE")],
        "answer",
    ));
    let mut project = Project::new();
    project.ports.http = http;
    let asked = || HttpRequest {
        keep_headers: vec!["etag".to_string(), "set-cookie".to_string()],
        ..request("GET")
    };

    let live = project.online("@t/a").http(asked()).unwrap();
    assert_eq!(live.headers.len(), 2, "{live:?}");

    let replayed = project.replay("@t/a").http(asked()).unwrap();
    assert_eq!(
        replayed.headers,
        [("etag".to_string(), "abc".to_string())],
        "{replayed:?}"
    );
    for entry in walkdir(project.dir.path()) {
        let kept = String::from_utf8_lossy(&std::fs::read(&entry).unwrap()).into_owned();
        assert!(
            !kept.contains("SECRET-VALUE"),
            "{}: {kept}",
            entry.display()
        );
    }
}

/// A payload that is no answer the host wrote answers nothing, never an empty success.
#[test]
fn a_record_without_its_answer_is_no_answer() {
    let project = Project::new();
    project.online("@t/a").http(request("GET")).unwrap();
    for entry in walkdir(project.dir.path()) {
        if entry.extension().is_some_and(|e| e == "payload") {
            std::fs::write(&entry, b"").unwrap();
        }
    }
    let answer = project.replay("@t/a").http(request("GET"));
    assert_eq!(failed_class(answer), "not_in_journal");
}

/// Every file under `dir`.
fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(walkdir(&path));
        } else {
            files.push(path);
        }
    }
    files
}

#[test]
fn replay_serves_an_extension_its_own_record_only() {
    let project = Project::new();
    project.online("@t/a").http(request("GET")).unwrap();

    let own = project.replay("@t/a").http(request("GET")).unwrap();
    assert_eq!(own.body, b"answer");
    let theirs = project.replay("@t/b").http(request("GET"));
    assert_eq!(failed_class(theirs), "not_in_journal");
    assert_eq!(project.http.requests().len(), 1);
}

#[test]
fn no_guest_reads_a_host_run_checks_record() {
    let project = Project::new();
    let effects = Effects::online(project.ports.clone(), project.journal());
    let asked = EffectRequest::Http(HttpRequest {
        body_cap: 1024,
        timeout_ms: 60_000,
        ..request("GET")
    });
    effects.serve("links", &asked, UnixMillis::now_or_end(), |_| Answer {
        outcome: Outcome {
            class: OutcomeClass::Ok,
            digest: "sha256:0".to_string(),
        },
        payload: None,
    });

    let answer = project.cached("@t/a").http(request("GET"));
    assert_eq!(failed_class(answer), "not_performed");
    let answer = project.replay("@t/a").http(request("GET"));
    assert_eq!(failed_class(answer), "not_in_journal");
}

#[test]
fn cached_holds_no_port_and_the_guest_hears_it_was_not_performed() {
    let project = Project::new();
    assert!(Effects::cached(project.journal()).ports().is_none());
    assert!(Effects::replay(project.journal()).ports().is_none());

    let answer = project.cached("@t/a").http(request("GET"));
    assert_eq!(failed_class(answer), "not_performed");
    assert!(project.http.requests().is_empty(), "nothing went out");
}

/// A dry run promises nothing happens: a POST its command asks for reaches no port, whatever mode the run asked for.
#[test]
fn a_dry_runs_post_reaches_nothing() {
    const EXT: &str = "@t/poster";
    fn poster() -> ContributionsBuilder {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.capability(
            Capability::http("api")
                .hosts(&["example.test"])
                .schemes(&["https"])
                .why("call the API"),
        );
        c.command("post", |cmd| {
            cmd.title("Post").handler(|_| {
                let answered = host::http::send(&request("POST"));
                CommandOutput::ok(format!("{:?}\n", answered.map(|r| r.status)))
            });
        });
        c
    }
    let runtime = InProcessRuntime::new()
        .with(poster)
        .importing(EXT, &["specforge:host/http@0.1.0"]);
    let project = TestProject::new().serving(runtime).enabling(&[EXT]);
    let compiled = project.compile();
    let view = ProjectView::of(&compiled);
    let commands = ExtensionCommands::build(view.registries());
    let post = commands.all().iter().find(|c| c.id() == "post").unwrap();

    let http = Arc::new(FakeHttp::new().route(URL, 200, "posted"));
    let ports = Ports {
        http: http.clone(),
        ..testing::ports()
    };
    let consent = MemoryConsent::approving_everything();
    let containment = Containment::unbounded();
    let run = |dry_run: bool, mode: EffectsMode| {
        let caller = Caller::new(&containment, Asking::Nobody).lending(&ports, &consent, mode);
        execute(
            &view,
            post,
            &Map::new(),
            CommandFormat::Json,
            None,
            dry_run,
            caller,
            Surface::Cli { interactive: false },
        )
        .unwrap()
        .output
        .stdout
    };

    for mode in [EffectsMode::Online, EffectsMode::Auto] {
        let answered = run(true, mode);
        assert!(answered.contains("not_performed"), "{mode:?}: {answered}");
    }
    assert!(http.requests().is_empty(), "nothing went out");

    // The same command, not a dry run, does reach the port.
    assert_eq!(run(false, EffectsMode::Online), "Ok(200)\n");
    assert_eq!(http.requests().len(), 1);
}

/// A POST asked for something to happen: yesterday's answer cannot say it did.
#[test]
fn a_request_that_is_no_get_is_never_answered_from_a_fresh_record() {
    let project = Project::new();
    project.online("@t/a").http(request("POST")).unwrap();
    assert_eq!(project.http.requests().len(), 1);

    let cached = project.cached("@t/a").http(request("POST"));
    assert_eq!(failed_class(cached), "not_performed");
    assert_eq!(project.http.requests().len(), 1, "cached sends nothing");

    project.auto("@t/a").http(request("POST")).unwrap();
    assert_eq!(project.http.requests().len(), 2, "auto performs it again");

    // A GET is answered again, by both.
    project.online("@t/a").http(request("GET")).unwrap();
    project.auto("@t/a").http(request("GET")).unwrap();
    project.cached("@t/a").http(request("GET")).unwrap();
    assert_eq!(project.http.requests().len(), 3);

    // Replay performs nothing and asked for the past.
    let replayed = project.replay("@t/a").http(request("POST")).unwrap();
    assert_eq!(replayed.body, b"answer");
    assert_eq!(project.http.requests().len(), 3);
}
