use super::*;
use crate::ceiling::{HttpCeiling, Narrow};
use crate::testing::MemoryConsent;
use serde_json::json;
use specforge_protocol_types::{FsRequest, HttpMethod, HttpRequest, ProcessRequest};

fn declaration(capabilities: serde_json::Value) -> ExtensionDeclaration {
    let mut declaration = ExtensionDeclaration::default();
    declaration.handshake.name = "@acme/x".to_string();
    declaration.capabilities = serde_json::from_value(capabilities).unwrap();
    declaration
}

fn caps() -> ExtensionDeclaration {
    declaration(json!([
        {"name": "tests", "why": "run the tests", "call_kinds": ["collect"], "port": "process",
         "argv": ["cargo", "test", "--out={report}"]},
        {"name": "links", "why": "check links", "port": "http", "methods": ["HEAD", "GET"],
         "hosts": {"list": ["*.example.com", "crates.io"]}},
        {"name": "docs", "why": "read docs", "port": "fs", "ops": ["stat", "read"]},
        {"name": "history", "why": "read history", "port": "git", "ops": ["read"]},
    ]))
}

fn roots() -> Roots {
    Roots::new("/p", "/p/spec")
}

fn third_party() -> Provenance {
    Provenance::third_party(b"blob")
}

fn http(method: &str, url: &str) -> EffectRequest {
    EffectRequest::Http(HttpRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers: Vec::new(),
        body: None,
        body_cap: 1,
        timeout_ms: 1,
        max_redirects: 0,
        allow_private: false,
        keep_headers: Vec::new(),
        secret: None,
    })
}

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

/// `hosts`' http capability, approved: only its hosts decide.
fn hosts(hosts: &[&str]) -> Grant {
    Grant::decide(
        &declaration(json!([
            {"name": "links", "why": "check links", "port": "http", "hosts": {"list": hosts}},
        ])),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &MemoryConsent::approving_everything(),
    )
}

fn permits_url(grant: &Grant, url: &str) -> bool {
    grant.permits(Operation::Collect, &http("GET", url)).is_ok()
}

#[test]
fn the_host_judged_is_the_host_the_url_connects_to() {
    let good = hosts(&["good.com"]);
    // A backslash ends the authority (WHATWG): this is evil.com, its path `/@good.com/`.
    assert!(!permits_url(&good, "https://evil.com\\@good.com/"));
    // Userinfo is not the host, either way round.
    assert!(!permits_url(&good, "https://good.com@evil.com/"));
    assert!(!permits_url(&good, "https://good.com:pw@evil.com/"));
    assert!(permits_url(&good, "https://user:pw@good.com/"));
    // A port is not part of the host, and the host compares without case.
    assert!(permits_url(&good, "https://good.com:8443/a?b#c"));
    assert!(permits_url(&good, "https://GOOD.com/"));
    // A name under or beside the host is another host; a trailing dot is one this scope does not list.
    assert!(!permits_url(&good, "https://good.com.evil.com/"));
    assert!(!permits_url(&good, "https://notgood.com/"));
    assert!(!permits_url(&good, "https://good.com./"));
    // An IDN is judged as it goes on the wire: a look-alike is not the host, and punycode lists the real one.
    assert!(!permits_url(&good, "https://g\u{043e}od.com/"));
    assert!(permits_url(
        &hosts(&["xn--bcher-kva.example"]),
        "https://b\u{fc}cher.example/"
    ));
    // An IPv6 literal is no listed name, however it is written; what cannot be parsed is nothing.
    assert!(!permits_url(&good, "https://[::1]/"));
    assert!(!permits_url(&good, "https://[::1]@good.com@evil.com/"));
    let from_values = Grant::decide(
        &declaration(json!([
            {"name": "links", "why": "check links", "port": "http", "hosts": "from_values"},
        ])),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &MemoryConsent::approving_everything(),
    );
    assert!(permits_url(&from_values, "https://[::1]:8080/"));
    assert!(!permits_url(&good, "not a url"));
    assert!(!permits_url(&good, "ftp://good.com/"));
}

fn run(argv: &[&str]) -> EffectRequest {
    EffectRequest::Process(ProcessRequest {
        argv: argv.iter().map(|a| a.to_string()).collect(),
        env: Vec::new(),
        stdin: None,
        deadline_ms: 1,
        output_cap: 1,
        text: false,
    })
}

fn read(root: &str, path: &str) -> EffectRequest {
    EffectRequest::Fs(FsRequest {
        root: root.to_string(),
        path: path.to_string(),
        op: FsOp::Read { max_bytes: 1 },
    })
}

fn uses(pairs: &[(Operation, &str)]) -> Uses {
    pairs
        .iter()
        .map(|(op, name)| (*op, name.to_string()))
        .collect()
}

fn approve_all(grant: &Grant, consent: &MemoryConsent) {
    for granted in grant.capabilities() {
        if let Some(key) = grant.key(granted) {
            consent.approve(Approval::Capability(key)).unwrap();
        }
    }
}

#[test]
fn the_gate_holds_each_call_kind_to_its_ports() {
    use Operation::*;
    let rows: [(Operation, [bool; 6]); 13] = [
        // fs read, fs write, git read, git write, http, process
        (Pass, [true, false, true, false, false, false]),
        (Validate, [true, false, true, false, false, false]),
        (Hook, [true, false, true, false, false, false]),
        (Prompt, [true, false, false, false, false, false]),
        (Render, [true, false, false, false, false, false]),
        (Scan, [true, false, false, false, false, false]),
        (Migrate, [true, false, false, false, false, false]),
        (Collect, [true, true, true, true, true, true]),
        (Command, [true, true, true, true, true, false]),
        (McpTool, [true, false, true, false, true, false]),
        (McpResource, [true, false, true, false, true, false]),
        (Handshake, [false; 6]),
        (Describe, [false; 6]),
    ];
    for (op, expected) in rows {
        let found = [
            gate(op, Port::Fs, false),
            gate(op, Port::Fs, true),
            gate(op, Port::Git, false),
            gate(op, Port::Git, true),
            gate(op, Port::Http, false),
            gate(op, Port::Process, false),
        ];
        assert_eq!(found, expected, "{op}");
    }
}

#[test]
fn a_grant_permits_what_is_declared_gated_and_approved() {
    let consent = MemoryConsent::default();
    let grant = Grant::decide(
        &caps(),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    );
    // Reading the spec needs no approval; http needs one.
    assert_eq!(
        grant.permits(Operation::Pass, &read("spec", "a.md")),
        Ok(())
    );
    let head = http("HEAD", "https://docs.example.com/a");
    assert_eq!(
        grant.permits(Operation::Collect, &head).unwrap_err().why,
        Denied::NotApproved
    );
    approve_all(&grant, &consent);
    let grant = Grant::decide(
        &caps(),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    );
    assert_eq!(grant.permits(Operation::Collect, &head), Ok(()));
    assert_eq!(
        grant.permits(
            Operation::Collect,
            &run(&["cargo", "test", "--out=/r.json"])
        ),
        Ok(())
    );
    // A pass never reaches the network, whatever is declared and approved.
    assert_eq!(
        grant.permits(Operation::Pass, &head).unwrap_err().why,
        Denied::CallKind
    );
    // Nothing declared covers another host, method or command.
    for (op, request) in [
        (Operation::Collect, http("HEAD", "https://example.org/")),
        (
            Operation::Collect,
            http("POST", "https://docs.example.com/"),
        ),
        (Operation::Collect, run(&["cargo", "test", "--all"])),
        (Operation::Pass, read("vendor", "a.md")),
    ] {
        assert_eq!(
            grant.permits(op, &request).unwrap_err().why,
            Denied::Undeclared,
            "{request:?}"
        );
    }
    // A command never runs a process: the gate refuses it before any capability is read.
    let command_run = run(&["cargo", "test", "--out=/r"]);
    assert_eq!(
        grant
            .permits(Operation::Command, &command_run)
            .unwrap_err()
            .why,
        Denied::CallKind
    );
    // A capability declared for some call kinds covers no other.
    let collect_only = declaration(
        json!([{"name": "api", "why": "ask", "call_kinds": ["collect"],
        "port": "http", "hosts": {"list": ["crates.io"]}}]),
    );
    let grant = Grant::decide(
        &collect_only,
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    );
    let get = http("GET", "https://crates.io/");
    assert_eq!(
        grant.permits(Operation::McpTool, &get).unwrap_err().why,
        Denied::Undeclared
    );
}

#[test]
fn a_ceiling_narrows_a_capability_and_what_it_narrows_out_is_denied() {
    let consent = MemoryConsent::approving_everything();
    let ceiling = EffectsCeiling {
        http: Narrow::Only(HttpCeiling {
            hosts: Some(vec!["crates.io".to_string()]),
            ..HttpCeiling::default()
        }),
        process: Narrow::Off,
        ..EffectsCeiling::default()
    };
    let grant = Grant::decide(&caps(), &third_party(), &ceiling, &roots(), &consent);
    let links = &grant.capabilities()[1];
    assert!(
        matches!(&links.scope, Some(PortScope::Http(h)) if h.hosts == Hosts::List(vec!["crates.io".into()]))
    );
    assert_eq!(links.removed[0].key, "effects.http.hosts");
    assert_eq!(
        grant.capabilities()[0].scope,
        None,
        "the process port is off"
    );
    assert_eq!(
        grant.permits(Operation::Collect, &http("GET", "https://crates.io/x")),
        Ok(())
    );
    let denied = grant
        .permits(
            Operation::Collect,
            &http("GET", "https://docs.example.com/x"),
        )
        .unwrap_err();
    assert_eq!(
        (denied.why, denied.scope.as_deref()),
        (Denied::Ceiling, Some("links"))
    );
}

#[test]
fn an_approval_covers_a_narrower_grant_and_a_wider_one_or_a_new_argv_asks_again() {
    let consent = MemoryConsent::default();
    let grant = Grant::decide(
        &caps(),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    );
    approve_all(&grant, &consent);
    let narrower = EffectsCeiling {
        http: Narrow::Only(HttpCeiling {
            methods: Some(vec![HttpMethod::Head]),
            ..HttpCeiling::default()
        }),
        ..EffectsCeiling::default()
    };
    let grant = Grant::decide(&caps(), &third_party(), &narrower, &roots(), &consent);
    assert!(
        grant
            .capabilities()
            .iter()
            .all(|g| !g.needs_approval || g.approved)
    );

    let wider = declaration(json!([
        {"name": "links", "why": "check links", "port": "http", "hosts": {"list": ["*.example.com", "*.io"]}},
        {"name": "tests", "why": "run", "call_kinds": ["collect"], "port": "process", "argv": ["cargo", "test", "--all"]},
    ]));
    let grant = Grant::decide(
        &wider,
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    );
    assert!(
        grant.capabilities().iter().all(|g| !g.approved),
        "a wider host list and a new argv ask again"
    );
}

#[test]
fn a_builtin_approval_never_meets_a_third_party_load_and_a_new_digest_asks_again() {
    let consent = MemoryConsent::default();
    let builtin = Provenance::builtin(b"blob");
    let grant = Grant::decide(
        &caps(),
        &builtin,
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    );
    approve_all(&grant, &consent);
    let approved = |provenance: &Provenance| {
        Grant::decide(
            &caps(),
            provenance,
            &EffectsCeiling::default(),
            &roots(),
            &consent,
        )
        .capabilities()[1]
            .approved
    };
    assert!(
        approved(&Provenance::builtin(b"another build")),
        "a builtin's approval survives an upgrade"
    );
    assert!(
        !approved(&Provenance::third_party(b"blob")),
        "the same name, loaded as a third party"
    );
    let grant = Grant::decide(
        &caps(),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    );
    approve_all(&grant, &consent);
    assert!(approved(&third_party()));
    assert!(
        !approved(&Provenance::third_party(b"blob v2")),
        "another blob asks again"
    );
}

#[test]
fn a_run_needs_only_the_approvals_of_what_it_uses_and_w170_lists_them() {
    let grant = Grant::decide(
        &caps(),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &MemoryConsent::default(),
    );
    let needs = grant.needs(&uses(&[
        (Operation::Collect, "tests"),
        (Operation::Pass, "docs"),
    ]));
    let scopes: Vec<String> = needs
        .iter()
        .map(|n| match n.approval() {
            Approval::Capability(a) => a.scope,
            Approval::Root(r) => r.root,
        })
        .collect();
    assert_eq!(
        scopes,
        ["tests"],
        "links is declared but unused; docs needs no approval"
    );
    assert!(
        grant.needs(&uses(&[(Operation::Pass, "links")])).is_empty(),
        "the gate refuses it: no ask"
    );

    let w170 = grant
        .unapproved_summary(&uses(&[(Operation::Collect, "tests")]))
        .unwrap();
    assert_eq!(w170.code, "W170");
    assert!(
        w170.message
            .contains("'tests' (process cargo test --out={report})"),
        "{}",
        w170.message
    );
    assert!(!w170.message.contains("links"));
    assert!(
        w170.suggestion
            .as_deref()
            .unwrap_or("")
            .contains("specforge grants approve @acme/x tests"),
        "{w170:?}"
    );
    assert!(
        grant
            .unapproved_summary(&uses(&[(Operation::Pass, "docs")]))
            .is_none()
    );
}

#[test]
fn e090_names_the_extension_the_request_and_the_scope() {
    let grant = Grant::decide(
        &caps(),
        &third_party(),
        &EffectsCeiling {
            http: Narrow::Off,
            ..EffectsCeiling::default()
        },
        &roots(),
        &MemoryConsent::approving_everything(),
    );
    let denial = grant
        .permits(Operation::Collect, &http("GET", "https://crates.io/a"))
        .unwrap_err();
    let diagnostic = denial.diagnostic();
    assert_eq!(diagnostic.code, "E090");
    for part in [
        "'@acme/x'",
        "GET https://crates.io/a",
        "capability 'links'",
        "effects ceiling",
    ] {
        assert!(
            diagnostic.message.contains(part),
            "{part}: {}",
            diagnostic.message
        );
    }
}

#[test]
fn the_memory_consent_keeps_the_consent_contract() {
    crate::testing::consent_contract(&MemoryConsent::default());
}

#[test]
fn an_unusable_store_approves_nothing_and_the_grant_says_why() {
    let grant = Grant::decide(
        &caps(),
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &MemoryConsent::unusable("not JSON"),
    );
    assert!(grant.capabilities().iter().all(|g| !g.approved));
    assert_eq!(
        grant.store_problem().map(|e| e.why.as_str()),
        Some("not JSON")
    );
}

#[test]
fn a_secret_rides_only_a_listed_https_host() {
    use crate::http::{Any, secret_header};
    use specforge_protocol_types::{SecretDescriptor, SettingsItem};
    let mut declaration = declaration(json!([
        {"name": "feed", "why": "read the release feed", "port": "http", "schemes": ["https"],
         "hosts": {"list": ["api.example.com"]},
         "secret_header": {"header": "Authorization", "secret": "token", "scheme": "Bearer"}},
    ]));
    declaration.settings = vec![SettingsItem::Secret(SecretDescriptor {
        name: "token".to_string(),
        env: "ACME_TOKEN".to_string(),
        description: None,
    })];
    let consent = MemoryConsent::approving_everything();
    let grant = std::sync::Arc::new(Grant::decide(
        &declaration,
        &third_party(),
        &EffectsCeiling::default(),
        &roots(),
        &consent,
    ));
    let fake = std::sync::Arc::new(crate::testing::FakeHttp::new());
    let granted = grant.http(Operation::Collect, fake.clone());
    let EffectRequest::Http(asked) = http("GET", "https://api.example.com/v1/releases") else {
        unreachable!()
    };
    granted.request(&asked, &Any);

    // The grant puts the secret's variable name on the request it permits; the value is read at send time.
    let sent = fake.requests();
    let secret = sent[0]
        .secret
        .clone()
        .expect("the grant attaches its secret");
    assert_eq!(secret.env, "ACME_TOKEN");
    assert_eq!(secret.hosts, ["api.example.com"]);
    let read = |env: &str| (env == "ACME_TOKEN").then(|| "t0k".to_string());
    assert_eq!(
        secret_header(&secret, &url("https://api.example.com/v1"), read),
        Some(("Authorization".to_string(), "Bearer t0k".to_string()))
    );
    // Plain http, another host (a redirect elsewhere) and an unset variable carry nothing.
    assert_eq!(
        secret_header(&secret, &url("http://api.example.com/v1"), read),
        None
    );
    assert_eq!(
        secret_header(&secret, &url("https://evil.example.org/"), read),
        None
    );
    assert_eq!(
        secret_header(&secret, &url("https://api.example.com.evil.org/"), read),
        None
    );
    assert_eq!(
        secret_header(&secret, &url("https://api.example.com/"), |_| None),
        None
    );
    // Neither the request's wire form nor its digest holds the secret.
    let wire = serde_json::to_string(&sent[0]).unwrap();
    assert!(
        !wire.contains("ACME_TOKEN") && !wire.contains("secret"),
        "{wire}"
    );
}
