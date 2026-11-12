//! The `Http` port's adapter against local servers (ADR 0335): the address class, redirects and their headers, the
//! body cap, the timeouts, and a TLS rejection as one class.
#![allow(
    clippy::unwrap_used,
    reason = "a test fails on the first unexpected value"
)]

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use specforge_effects::http::{Any, get};
use specforge_effects::testing::server::Server;
use specforge_effects::{
    AttachedSecret, EffectsCeiling, Grant, Header, Http, HttpCeiling, HttpRequest, Narrow,
    OutcomeClass, Private, Redirected, ResolveRequest, TlsRequest, Unreachable,
};
use specforge_http::{Build, Client};
use specforge_protocol_types::{HttpMethod, Operation};

mod tls;

/// How long a test waits for what must come: a wait ends as soon as it does, so only a failing test spends it, and it
/// stays under the test runner's kill (60 s), so a failing test prints its assertion.
const WAIT: Duration = Duration::from_secs(20);
/// A request to a server that answers is given [`WAIT`], so a loaded machine never times it out.
const WAIT_MS: u64 = WAIT.as_millis() as u64;
/// The ceiling of a client whose request must time out: whatever holds the request, its deadline ends it by then.
const SHORT: Duration = Duration::from_millis(500);

/// A client whose every request, resolve and handshake gives up by [`SHORT`].
fn short() -> Client {
    Client::new().with_timeout_ceiling(SHORT)
}

fn ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn moved(to: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

/// A `GET` of `url` that may reach loopback on every hop.
fn private(url: &str) -> HttpRequest {
    HttpRequest {
        allow_private: Private::Any,
        timeout_ms: WAIT_MS,
        ..get(url)
    }
}

/// A person's `GET` of `url`: loopback is theirs to reach at the typed origin only.
fn typed(url: &str) -> HttpRequest {
    HttpRequest {
        allow_private: Private::TypedOrigin,
        timeout_ms: WAIT_MS,
        ..get(url)
    }
}

/// A typed private address may redirect within its own origin; a redirect to another origin at a loopback address
/// (as one to `169.254.169.254` would be) is refused and never asked.
#[test]
fn only_the_typed_origin_may_be_private_on_a_typed_request() {
    let other_heads = Server::reply(|_, _| ok("other"));
    let other = other_heads.base.clone();
    let base = Server::reply(move |n, _| {
        if n == 0 {
            moved("/here")
        } else if n == 1 {
            ok("here")
        } else {
            moved(&format!("{other}/"))
        }
    })
    .base;
    let same = Client::new().request(&typed(&format!("{base}/")), &Any);
    assert_eq!(same.class, OutcomeClass::Ok, "{same:?}");
    assert_eq!(same.body, b"here");
    let elsewhere = Client::new().request(&typed(&format!("{base}/")), &Any);
    assert_eq!(
        elsewhere.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress),
        "{elsewhere:?}"
    );
    assert!(other_heads.requests().is_empty(), "the hop was requested");
}

#[specforge_test_macros::test(port = "Http", verify = "Http contract is satisfied")]
fn the_client_keeps_the_http_contract() {
    specforge_effects::testing::http_contract(&Client::new());
}

#[test]
fn loopback_is_refused_unless_the_request_allows_private_addresses() {
    let base = Server::reply(|_, _| ok("hi")).base;
    let client = Client::new();
    let refused = client.request(&get(format!("{base}/")), &Any);
    assert_eq!(
        refused.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress),
        "{refused:?}"
    );
    let port = base.rsplit(':').next().unwrap();
    let named = client.request(&get(format!("http://localhost:{port}/")), &Any);
    assert_eq!(
        named.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress),
        "{named:?}"
    );
    let allowed = client.request(&private(&format!("{base}/")), &Any);
    assert_eq!(allowed.class, OutcomeClass::Ok, "{allowed:?}");
    assert_eq!(allowed.body, b"hi");
}

#[test]
fn redirects_past_the_limit_are_refused() {
    let base = Server::reply(|_, _| moved("/again")).base;
    let outcome = Client::new().request(
        &HttpRequest {
            max_redirects: 2,
            ..private(&format!("{base}/"))
        },
        &Any,
    );
    assert_eq!(
        outcome.class,
        OutcomeClass::Redirect(Redirected::Limit),
        "{outcome:?}"
    );
    assert_eq!(outcome.status, Some(302));
    // The redirect not followed keeps its kept headers: with no redirect to follow, it is the answer.
    let unfollowed = Client::new().request(
        &HttpRequest {
            max_redirects: 0,
            keep_headers: vec!["location".to_string()],
            ..private(&format!("{base}/"))
        },
        &Any,
    );
    assert_eq!(
        (unfollowed.class, unfollowed.header("location")),
        (OutcomeClass::Redirect(Redirected::Limit), Some("/again")),
        "{unfollowed:?}"
    );
}

#[test]
fn a_sensitive_header_is_dropped_on_a_redirect_to_another_origin() {
    let seen_there = Server::reply(|_, _| ok("there"));
    let other = seen_there.base.clone();
    let to = format!("{other}/landed");
    let seen_here = Server::reply(move |_, _| moved(&to));
    let base = seen_here.base.clone();
    let outcome = Client::new().request(
        &HttpRequest {
            headers: vec![
                Header::sensitive("Authorization", "Bearer t"),
                Header::plain("X-Plain", "p"),
            ],
            ..private(&format!("{base}/"))
        },
        &Any,
    );
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert_eq!(outcome.final_url, format!("{other}/landed"));
    assert!(
        seen_here
            .next(WAIT)
            .unwrap()
            .head
            .to_ascii_lowercase()
            .contains("authorization: bearer t")
    );
    let there = seen_there.next(WAIT).unwrap().head.to_ascii_lowercase();
    assert!(!there.contains("authorization"), "{there}");
    assert!(there.contains("x-plain: p"), "{there}");
}

#[test]
fn a_body_past_its_cap_is_cut_there_and_capped() {
    let base = Server::reply(|n, _| {
        if n == 0 {
            ok("0123456789abcdef")
        } else {
            // No Content-Length: the cap is found while reading.
            "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n0123456789abcdef".into()
        }
    })
    .base;
    let client = Client::new();
    for _ in 0..2 {
        let outcome = client.request(
            &HttpRequest {
                body_cap: 10,
                ..private(&format!("{base}/"))
            },
            &Any,
        );
        assert_eq!(outcome.class, OutcomeClass::Capped, "{outcome:?}");
        assert!(outcome.body.len() <= 10);
    }
}

#[test]
fn a_request_is_held_to_its_whole_timeout() {
    // Headers, then a body that never ends: the connection is held open for the whole run, so only the request's
    // own deadline can end it.
    let held = std::sync::Mutex::new(Vec::new());
    let base = Server::serve(move |_, _, stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nslow");
        held.lock().unwrap().push(stream.try_clone().unwrap());
    })
    .base;
    let outcome = short().request(&private(&format!("{base}/")), &Any);
    assert_eq!(
        outcome.class,
        OutcomeClass::Unreachable(Unreachable::Timeout),
        "{outcome:?}"
    );
}

#[test]
fn a_blackhole_address_gives_up_within_the_timeout() {
    // 10.255.255.1 is never routed: the connect waits until the request's time runs out, or fails at once offline.
    let outcome = short().request(&private("http://10.255.255.1:81/"), &Any);
    assert!(
        matches!(
            outcome.class,
            OutcomeClass::Unreachable(Unreachable::Timeout | Unreachable::Connect)
        ),
        "{outcome:?}"
    );
}

/// Building a client reads the trust store and the system's proxies, seconds on a loaded machine: a fresh client's
/// first request waits for that build no longer than its own deadline, and a later request takes what it built.
#[test]
fn a_request_waits_for_its_clients_build_no_longer_than_its_deadline() {
    let base = Server::reply(|_, _| ok("hi")).base;
    let (release, released) = channel::<()>();
    let released = Mutex::new(released);
    let (builds, past_hold) = (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
    let (counted, passed) = (Arc::clone(&builds), Arc::clone(&past_hold));
    let client = Client::new().with_before_build(move |build| {
        if let Build::Client { .. } = build {
            counted.fetch_add(1, Ordering::SeqCst);
            // Held until the test lets it go; at most WAIT, so a failing test still ends.
            let _ = released.lock().unwrap().recv_timeout(WAIT);
            passed.store(true, Ordering::SeqCst);
        }
        Ok(())
    });

    let first = client.request(
        &HttpRequest {
            timeout_ms: 300,
            ..private(&format!("{base}/"))
        },
        &Any,
    );

    assert_eq!(
        first.class,
        OutcomeClass::Unreachable(Unreachable::Timeout),
        "{first:?}"
    );
    assert!(
        !past_hold.load(Ordering::SeqCst),
        "the request waited for the build"
    );
    release.send(()).unwrap();
    // The build goes on once let go; the next request, given time, waits for it.
    let second = client.request(&private(&format!("{base}/")), &Any);
    assert_eq!(second.class, OutcomeClass::Ok, "{second:?}");
    assert_eq!(second.body, b"hi");
    assert_eq!(
        builds.load(Ordering::SeqCst),
        1,
        "one build, taken by the later request"
    );
}

/// The whole request under a grant that lists only `127.0.0.1`.
fn only_loopback_address() -> Arc<dyn Http> {
    let ceiling = EffectsCeiling {
        http: Narrow::Only(HttpCeiling {
            hosts: Some(vec!["127.0.0.1".to_string()]),
            ..HttpCeiling::default()
        }),
        ..EffectsCeiling::default()
    };
    Arc::new(Grant::user(&ceiling)).http(Operation::Collect, Arc::new(Client::new()))
}

#[test]
fn a_redirect_the_grant_does_not_permit_stops_as_denied_and_is_never_requested() {
    let foreign_heads = Server::reply(|_, _| ok("foreign"));
    let foreign = foreign_heads.base.clone();
    let foreign = foreign.replace("127.0.0.1", "localhost");
    let base = Server::reply(move |_, _| moved(&format!("{foreign}/"))).base;
    let outcome = only_loopback_address().request(&private(&format!("{base}/")), &Any);
    assert_eq!(outcome.class, OutcomeClass::Denied, "{outcome:?}");
    assert!(outcome.detail.unwrap().contains("redirects to"));
    assert!(foreign_heads.requests().is_empty(), "the hop was requested");
}

#[test]
fn a_redirect_the_grant_permits_is_followed() {
    let next = Server::reply(|_, _| ok("there")).base;
    let base = Server::reply(move |_, _| moved(&format!("{next}/"))).base;
    let outcome = only_loopback_address().request(&private(&format!("{base}/")), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert_eq!(outcome.body, b"there");
}

#[test]
fn a_private_address_is_refused_before_a_proxy_is_asked_for_it() {
    let heads = Server::reply(|_, _| ok("from behind the proxy"));
    let proxy = heads.base.clone();
    let client = Client::with_proxy(&proxy);
    for url in [
        "http://localhost:9/admin",
        "http://127.0.0.1:9/admin",
        "http://[::1]:9/admin",
        "http://10.0.0.1/admin",
        "http://169.254.169.254/latest/meta-data/",
    ] {
        let outcome = client.request(&get(url), &Any);
        assert_eq!(
            outcome.class,
            OutcomeClass::Unreachable(Unreachable::RefusedAddress),
            "{url}: {outcome:?}"
        );
        assert!(heads.requests().is_empty(), "{url}");
    }
}

#[test]
fn a_redirect_through_a_proxy_to_a_private_name_is_refused() {
    let heads = Server::reply(|_, _| moved("http://localhost:9/admin"));
    let proxy = heads.base.clone();
    let outcome =
        Client::with_proxy(&proxy).request(&get("http://specforge-proxied.invalid/"), &Any);
    assert_eq!(
        outcome.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress),
        "{outcome:?}"
    );
    assert_eq!(heads.requests().len(), 1, "the first hop only");
}

/// What the client cannot close (ADR 0335): a name it cannot resolve itself goes to the proxy, which resolves it
/// where this machine cannot look. Refusing it would break the networks whose only DNS is the proxy's.
#[test]
fn a_name_only_the_proxy_can_resolve_is_the_proxys_to_judge() {
    let heads = Server::reply(|_, _| ok("from behind the proxy"));
    let proxy = heads.base.clone();
    let outcome =
        Client::with_proxy(&proxy).request(&get("http://specforge-proxied.invalid/"), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert!(
        heads
            .next(WAIT)
            .unwrap()
            .head
            .to_ascii_lowercase()
            .starts_with("get http://specforge-proxied.invalid/ ")
    );
}

#[test]
fn a_request_that_allows_private_addresses_still_goes_through_the_proxy() {
    let heads = Server::reply(|_, _| ok("from behind the proxy"));
    let proxy = heads.base.clone();
    let outcome = Client::with_proxy(&proxy).request(&private("http://localhost:9/"), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert!(
        heads
            .next(WAIT)
            .unwrap()
            .head
            .to_ascii_lowercase()
            .starts_with("get http://localhost:9/ ")
    );
}

/// A request that fails at the proxy (its connection, its authentication, a refused `CONNECT`) is the proxy's
/// failure, not the destination's.
#[test]
fn a_request_the_proxy_fails_is_unreachable_proxy() {
    let refusing = Server::reply(
        |_, _| "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    )
    .base;
    let asking = Server::reply(|_, _| {
        "HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    })
    .base;
    let gone = format!("http://127.0.0.1:{}", Server::closed_port());
    for (proxy, url) in [
        (&refusing, "https://specforge-proxied.invalid/"),
        (&asking, "https://specforge-proxied.invalid/"),
        (&gone, "http://specforge-proxied.invalid/"),
        (&gone, "https://specforge-proxied.invalid/"),
    ] {
        let outcome = Client::with_proxy(proxy).request(&get(url), &Any);
        assert_eq!(
            outcome.class,
            OutcomeClass::Unreachable(Unreachable::Proxy),
            "{proxy} {url}: {outcome:?}"
        );
    }
}

/// A redirect answered `status`, to `to`.
fn redirect(status: u16, to: &str) -> String {
    format!(
        "HTTP/1.1 {status} Moved\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

/// The whole request under a grant that allows only `methods`, on any host.
fn only_methods(methods: &[HttpMethod]) -> Arc<dyn Http> {
    let ceiling = EffectsCeiling {
        http: Narrow::Only(HttpCeiling {
            methods: Some(methods.to_vec()),
            ..HttpCeiling::default()
        }),
        ..EffectsCeiling::default()
    };
    Arc::new(Grant::user(&ceiling)).http(Operation::Collect, Arc::new(Client::new()))
}

/// A `POST` with a body, of `url`, that may reach loopback.
fn posting(url: &str) -> HttpRequest {
    HttpRequest {
        method: "POST".to_string(),
        body: Some(b"secret form".to_vec()),
        ..private(url)
    }
}

#[test]
fn a_hop_that_will_be_sent_as_a_get_is_judged_as_a_get() {
    for status in [301, 302, 303] {
        let next_heads = Server::reply(|_, _| ok("there"));
        let next = next_heads.base.clone();
        let base = Server::reply(move |_, _| redirect(status, &format!("{next}/"))).base;

        let refused =
            only_methods(&[HttpMethod::Post]).request(&posting(&format!("{base}/")), &Any);
        assert_eq!(refused.class, OutcomeClass::Denied, "{status}: {refused:?}");
        assert!(
            next_heads.requests().is_empty(),
            "{status}: a POST-only scope sent a GET"
        );
    }
}

#[test]
fn a_hop_the_scope_allows_as_a_get_is_sent_as_a_get_with_no_body() {
    let next_heads = Server::reply(|_, _| ok("there"));
    let next = next_heads.base.clone();
    let base = Server::reply(move |_, _| redirect(303, &format!("{next}/"))).base;
    let outcome = only_methods(&[HttpMethod::Post, HttpMethod::Get])
        .request(&posting(&format!("{base}/")), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    let head = next_heads.next(WAIT).unwrap().head.to_ascii_lowercase();
    assert!(head.starts_with("get / "), "{head}");
    assert!(!head.contains("content-length"), "{head}");
}

#[test]
fn a_head_redirected_by_a_303_stays_a_head() {
    let next_heads = Server::reply(|_, _| ok("there"));
    let next = next_heads.base.clone();
    let base = Server::reply(move |_, _| redirect(303, &format!("{next}/"))).base;
    let heading = HttpRequest {
        method: "HEAD".to_string(),
        ..private(&format!("{base}/"))
    };
    let outcome = only_methods(&[HttpMethod::Head]).request(&heading, &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    let head = next_heads.next(WAIT).unwrap().head.to_ascii_lowercase();
    assert!(head.starts_with("head / "), "{head}");
}

#[test]
fn a_hop_that_keeps_its_method_is_judged_with_it() {
    for status in [307, 308] {
        let next_heads = Server::reply(|_, _| ok("there"));
        let next = next_heads.base.clone();
        let base = Server::reply(move |_, _| redirect(status, &format!("{next}/"))).base;
        let outcome =
            only_methods(&[HttpMethod::Post]).request(&posting(&format!("{base}/")), &Any);
        assert_eq!(outcome.class, OutcomeClass::Ok, "{status}: {outcome:?}");
        let head = next_heads.next(WAIT).unwrap().head.to_ascii_lowercase();
        assert!(head.starts_with("post / "), "{status}: {head}");
    }
}

#[test]
fn a_post_query_sends_its_json_body_and_is_never_retried() {
    let answering =
        Server::reply(|_, _| "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
    let base = answering.base.clone();
    let query = |url: String, timeout_ms| HttpRequest {
        method: "POST".into(),
        headers: vec![Header::plain("content-type", "application/json")],
        body: Some(br#"{"package":"serde"}"#.to_vec()),
        max_redirects: 0,
        timeout_ms,
        ..private(&url)
    };
    let client = Client::new();
    let outcome = client.request(&query(format!("{base}/v1/query"), WAIT_MS), &Any);
    assert_eq!(outcome.status, Some(200), "{outcome:?}");
    assert_eq!(outcome.body, b"{}");
    let asked = answering.next(WAIT).unwrap();
    assert!(asked.method.eq_ignore_ascii_case("POST"), "{asked:?}");
    assert_eq!(asked.path, "/v1/query");
    assert_eq!(asked.header("content-type"), Some("application/json"));
    assert_eq!(asked.body, br#"{"package":"serde"}"#);

    // A server that never answers: the query times out once, and is not sent again.
    let held = std::sync::Mutex::new(Vec::new());
    let silent = Server::serve(move |_, _, stream| {
        held.lock().unwrap().push(stream.try_clone().unwrap());
    });
    let outcome = client.request(&query(format!("{}/v1/query", silent.base), 300), &Any);
    assert_eq!(
        outcome.class,
        OutcomeClass::Unreachable(Unreachable::Timeout),
        "{outcome:?}"
    );
    // The server reads its connections in the order they came, and the client sent all it would before it returned:
    // what the server read before this request is the query's connection, at most once.
    let mut after =
        std::net::TcpStream::connect(silent.base.trim_start_matches("http://")).unwrap();
    after
        .write_all(b"GET /after HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    let before: Vec<_> = std::iter::from_fn(|| silent.next(WAIT))
        .take_while(|request| request.path != "/after")
        .collect();
    assert_eq!(before.len(), 1, "sent once: {before:?}");
}

/// A policy that permits every redirect and the proxies on `hosts` alone, and keeps each proxy it was asked about.
struct Proxies {
    hosts: Vec<&'static str>,
    asked: std::sync::Mutex<Vec<(String, String)>>,
}

impl specforge_effects::Redirects for Proxies {
    fn allow(&self, _from: &str, _to: &str, _method: &str) -> bool {
        true
    }

    fn through_proxy(&self, host: &str, url: &str) -> bool {
        self.asked
            .lock()
            .unwrap()
            .push((host.to_string(), url.to_string()));
        self.hosts.contains(&host)
    }
}

/// The proxy a request goes through is the caller's to permit, asked before anything is sent: a refused one sees no
/// request, and the answer is `denied`.
#[test]
fn a_proxy_the_policy_refuses_is_never_sent_to() {
    let proxy = Server::reply(|_, _| ok("through the proxy"));
    let url = "http://specforge-proxied.invalid/";
    let policy = |hosts| Proxies {
        hosts,
        asked: std::sync::Mutex::new(Vec::new()),
    };
    let refusing = policy(Vec::new());
    let outcome = Client::with_proxy(&proxy.base).request(&get(url), &refusing);
    assert_eq!(outcome.class, OutcomeClass::Denied, "{outcome:?}");
    assert!(proxy.requests().is_empty(), "the proxy saw a request");
    assert_eq!(
        *refusing.asked.lock().unwrap(),
        [("127.0.0.1".to_string(), url.to_string())]
    );
    let permitting = policy(vec!["127.0.0.1"]);
    let outcome = Client::with_proxy(&proxy.base).request(&get(url), &permitting);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert_eq!(outcome.body, b"through the proxy");
}
