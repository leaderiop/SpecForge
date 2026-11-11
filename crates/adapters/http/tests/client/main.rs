//! The `Http` port's adapter against local servers (ADR 0335): the address class, redirects and their headers, the
//! body cap, the timeouts, and a TLS rejection as one class.
#![allow(
    clippy::unwrap_used,
    reason = "a test fails on the first unexpected value"
)]

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use specforge_effects::http::{Any, TIMEOUT_CEILING, get};
use specforge_effects::testing::server::Server;
use specforge_effects::{
    AttachedSecret, EffectsCeiling, Grant, Header, Http, HttpCeiling, HttpRequest, Narrow,
    OutcomeClass, Private, Redirected, ResolveRequest, TlsRequest, Unreachable,
};
use specforge_http::Client;
use specforge_protocol_types::{HttpMethod, Operation};

mod tls;

/// How long a test waits for what must come: a wait ends as soon as it does, so only a failing test spends it.
const WAIT: Duration = TIMEOUT_CEILING;
/// A request to a server that answers is given the adapter's whole ceiling, so a loaded machine never times it out.
const CEILING_MS: u64 = TIMEOUT_CEILING.as_millis() as u64;
/// A request held to a time of a second or less that gives up before this gave up at its own time, not the ceiling's.
const WELL_BEFORE_THE_CEILING: Duration = Duration::from_secs(TIMEOUT_CEILING.as_secs() / 2);

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
        timeout_ms: CEILING_MS,
        ..get(url)
    }
}

/// A person's `GET` of `url`: loopback is theirs to reach at the typed origin only.
fn typed(url: &str) -> HttpRequest {
    HttpRequest {
        allow_private: Private::TypedOrigin,
        timeout_ms: CEILING_MS,
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
    let started = Instant::now();
    let outcome = Client::new().request(
        &HttpRequest {
            timeout_ms: 500,
            ..private(&format!("{base}/"))
        },
        &Any,
    );
    assert_eq!(
        outcome.class,
        OutcomeClass::Unreachable(Unreachable::Timeout),
        "{outcome:?}"
    );
    assert!(
        started.elapsed() < WELL_BEFORE_THE_CEILING,
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_blackhole_address_gives_up_within_the_timeout() {
    let started = Instant::now();
    // 10.255.255.1 is never routed: the connect waits until the request's time runs out, or fails at once offline.
    let outcome = Client::new().request(
        &HttpRequest {
            timeout_ms: 1000,
            ..private("http://10.255.255.1:81/")
        },
        &Any,
    );
    assert!(
        matches!(
            outcome.class,
            OutcomeClass::Unreachable(Unreachable::Timeout | Unreachable::Connect)
        ),
        "{outcome:?}"
    );
    assert!(
        started.elapsed() < WELL_BEFORE_THE_CEILING,
        "{:?}",
        started.elapsed()
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
    let outcome = client.request(&query(format!("{base}/v1/query"), CEILING_MS), &Any);
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
    assert!(before.len() <= 1, "sent once: {before:?}");
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
