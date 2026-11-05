//! The `Http` port's adapter against local servers (ADR 0335): the address class, redirects and their headers, the
//! body cap, the timeouts, and a TLS rejection as one class.
#![allow(
    clippy::unwrap_used,
    reason = "a test fails on the first unexpected value"
)]

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use specforge_effects::http::{Any, get};
use specforge_effects::{
    AttachedSecret, EffectsCeiling, Grant, Header, Http, HttpCeiling, HttpRequest, Narrow,
    OutcomeClass, Private, Redirected, ResolveRequest, TlsRequest, Unreachable,
};
use specforge_http::Client;
use specforge_protocol_types::{HttpMethod, Operation};

/// A server on loopback that answers each connection with `answer(n, head)` (the `n`th request, its head) and sends
/// back each request head. Its address is `http://127.0.0.1:<port>`.
fn serve(answer: impl Fn(usize, &str) -> String + Send + 'static) -> (String, Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (heads, seen) = channel();
    std::thread::spawn(move || {
        for (n, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else { continue };
            let head = head_of(&mut stream);
            let _ = heads.send(head.clone());
            let _ = stream.write_all(answer(n, &head).as_bytes());
        }
    });
    (base, seen)
}

fn head_of(stream: &mut TcpStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
        head.push(byte[0]);
    }
    String::from_utf8_lossy(&head).to_ascii_lowercase()
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
        ..get(url)
    }
}

/// A person's `GET` of `url`: loopback is theirs to reach at the typed origin only.
fn typed(url: &str) -> HttpRequest {
    HttpRequest {
        allow_private: Private::TypedOrigin,
        ..get(url)
    }
}

/// A typed private address may redirect within its own origin; a redirect to another origin at a loopback address
/// (as one to `169.254.169.254` would be) is refused and never asked.
#[test]
fn only_the_typed_origin_may_be_private_on_a_typed_request() {
    let (other, other_heads) = serve(|_, _| ok("other"));
    let (base, _) = serve(move |n, _| {
        if n == 0 {
            moved("/here")
        } else if n == 1 {
            ok("here")
        } else {
            moved(&format!("{other}/"))
        }
    });
    let same = Client::new().request(&typed(&format!("{base}/")), &Any);
    assert_eq!(same.class, OutcomeClass::Ok, "{same:?}");
    assert_eq!(same.body, b"here");
    let elsewhere = Client::new().request(&typed(&format!("{base}/")), &Any);
    assert_eq!(
        elsewhere.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress),
        "{elsewhere:?}"
    );
    assert!(asked(&other_heads).is_empty(), "the hop was requested");
}

#[specforge_test_macros::test(port = "Http", verify = "Http contract is satisfied")]
fn the_client_keeps_the_http_contract() {
    specforge_effects::testing::http_contract(&Client::new());
}

#[test]
fn loopback_is_refused_unless_the_request_allows_private_addresses() {
    let (base, _) = serve(|_, _| ok("hi"));
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
    let (base, _) = serve(|_, _| moved("/again"));
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
    let (other, seen_there) = serve(|_, _| ok("there"));
    let to = format!("{other}/landed");
    let (base, seen_here) = serve(move |_, _| moved(&to));
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
            .recv()
            .unwrap()
            .contains("authorization: bearer t")
    );
    let there = seen_there.recv().unwrap();
    assert!(!there.contains("authorization"), "{there}");
    assert!(there.contains("x-plain: p"), "{there}");
}

#[test]
fn a_body_past_its_cap_is_cut_there_and_capped() {
    let (base, _) = serve(|n, _| {
        if n == 0 {
            ok("0123456789abcdef")
        } else {
            // No Content-Length: the cap is found while reading.
            "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n0123456789abcdef".into()
        }
    });
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
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        head_of(&mut stream);
        // Headers, then a body that never ends.
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nslow");
        std::thread::sleep(Duration::from_secs(10));
    });
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
    assert!(started.elapsed() < Duration::from_secs(5));
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
    assert!(started.elapsed() < Duration::from_secs(7));
}

#[test]
fn tls_rejection_is_one_class() {
    let cert =
        rustls::pki_types::CertificateDer::from(include_bytes!("self-signed.cert.der").to_vec());
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
        include_bytes!("self-signed.key.der").to_vec().into(),
    );
    let config = Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let connection = rustls::ServerConnection::new(config.clone()).unwrap();
            let mut tls = rustls::StreamOwned::new(connection, stream);
            // The client rejects the certificate: the handshake fails here.
            let _ = tls.write_all(&ok("secret").into_bytes());
        }
    });
    for url in [
        format!("https://127.0.0.1:{port}/"),
        format!("https://localhost:{port}/"),
    ] {
        let outcome = Client::new().request(&private(&url), &Any);
        assert_eq!(
            outcome.class,
            OutcomeClass::Unreachable(Unreachable::Tls),
            "{outcome:?}"
        );
        assert!(outcome.body.is_empty());
    }
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
    let (foreign, foreign_heads) = serve(|_, _| ok("foreign"));
    let foreign = foreign.replace("127.0.0.1", "localhost");
    let (base, _) = serve(move |_, _| moved(&format!("{foreign}/")));
    let outcome = only_loopback_address().request(&private(&format!("{base}/")), &Any);
    assert_eq!(outcome.class, OutcomeClass::Denied, "{outcome:?}");
    assert!(outcome.detail.unwrap().contains("redirects to"));
    assert!(foreign_heads.try_recv().is_err(), "the hop was requested");
}

#[test]
fn a_redirect_the_grant_permits_is_followed() {
    let (next, _) = serve(|_, _| ok("there"));
    let (base, _) = serve(move |_, _| moved(&format!("{next}/")));
    let outcome = only_loopback_address().request(&private(&format!("{base}/")), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert_eq!(outcome.body, b"there");
}

/// What `seen` received: nothing, for a request that was never sent.
fn asked(seen: &Receiver<String>) -> Vec<String> {
    seen.try_iter().collect()
}

#[test]
fn a_private_address_is_refused_before_a_proxy_is_asked_for_it() {
    let (proxy, heads) = serve(|_, _| ok("from behind the proxy"));
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
        assert_eq!(asked(&heads), Vec::<String>::new(), "{url}");
    }
}

#[test]
fn a_redirect_through_a_proxy_to_a_private_name_is_refused() {
    let (proxy, heads) = serve(|_, _| moved("http://localhost:9/admin"));
    let outcome =
        Client::with_proxy(&proxy).request(&get("http://specforge-proxied.invalid/"), &Any);
    assert_eq!(
        outcome.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress),
        "{outcome:?}"
    );
    assert_eq!(asked(&heads).len(), 1, "the first hop only");
}

/// What the client cannot close (ADR 0335): a name it cannot resolve itself goes to the proxy, which resolves it
/// where this machine cannot look. Refusing it would break the networks whose only DNS is the proxy's.
#[test]
fn a_name_only_the_proxy_can_resolve_is_the_proxys_to_judge() {
    let (proxy, heads) = serve(|_, _| ok("from behind the proxy"));
    let outcome =
        Client::with_proxy(&proxy).request(&get("http://specforge-proxied.invalid/"), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert!(
        heads
            .recv()
            .unwrap()
            .starts_with("get http://specforge-proxied.invalid/ ")
    );
}

#[test]
fn a_request_that_allows_private_addresses_still_goes_through_the_proxy() {
    let (proxy, heads) = serve(|_, _| ok("from behind the proxy"));
    let outcome = Client::with_proxy(&proxy).request(&private("http://localhost:9/"), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    assert!(
        heads
            .recv()
            .unwrap()
            .starts_with("get http://localhost:9/ ")
    );
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
        let (next, next_heads) = serve(|_, _| ok("there"));
        let (base, _) = serve(move |_, _| redirect(status, &format!("{next}/")));

        let refused =
            only_methods(&[HttpMethod::Post]).request(&posting(&format!("{base}/")), &Any);
        assert_eq!(refused.class, OutcomeClass::Denied, "{status}: {refused:?}");
        assert!(
            next_heads.try_recv().is_err(),
            "{status}: a POST-only scope sent a GET"
        );
    }
}

#[test]
fn a_hop_the_scope_allows_as_a_get_is_sent_as_a_get_with_no_body() {
    let (next, next_heads) = serve(|_, _| ok("there"));
    let (base, _) = serve(move |_, _| redirect(303, &format!("{next}/")));
    let outcome = only_methods(&[HttpMethod::Post, HttpMethod::Get])
        .request(&posting(&format!("{base}/")), &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    let head = next_heads.recv().unwrap();
    assert!(head.starts_with("get / "), "{head}");
    assert!(!head.contains("content-length"), "{head}");
}

#[test]
fn a_hop_that_keeps_its_method_is_judged_with_it() {
    for status in [307, 308] {
        let (next, next_heads) = serve(|_, _| ok("there"));
        let (base, _) = serve(move |_, _| redirect(status, &format!("{next}/")));
        let outcome =
            only_methods(&[HttpMethod::Post]).request(&posting(&format!("{base}/")), &Any);
        assert_eq!(outcome.class, OutcomeClass::Ok, "{status}: {outcome:?}");
        let head = next_heads.recv().unwrap();
        assert!(head.starts_with("post / "), "{status}: {head}");
    }
}

/// A TLS server on loopback answering each connection as [`serve`] does, under the certificate for `localhost` and
/// `127.0.0.1` that [`trusting`] trusts. Its port.
fn serve_tls(answer: impl Fn(&str) -> String + Send + 'static) -> (u16, Receiver<String>) {
    let cert = rustls::pki_types::CertificateDer::from(
        include_bytes!("self-signed-leaf.cert.der").to_vec(),
    );
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
        include_bytes!("self-signed.key.der").to_vec().into(),
    );
    let config = Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (heads, seen) = channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let connection = rustls::ServerConnection::new(config.clone()).unwrap();
            let mut tls = rustls::StreamOwned::new(connection, stream);
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && tls.read(&mut byte).is_ok_and(|n| n == 1) {
                head.push(byte[0]);
            }
            let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
            let _ = heads.send(head.clone());
            let _ = tls.write_all(answer(&head).as_bytes());
            tls.conn.send_close_notify();
            let _ = tls.flush();
        }
    });
    (port, seen)
}

/// A client that trusts [`serve_tls`]'s certificate.
fn trusting() -> Client {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            include_bytes!("self-signed-leaf.cert.der").to_vec(),
        ))
        .unwrap();
    Client::with_tls(
        rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth(),
    )
}

#[test]
fn a_secret_goes_to_the_hosts_it_lists_and_is_dropped_on_a_redirect_elsewhere() {
    let (foreign, foreign_heads) = serve_tls(|_| ok("foreign"));
    let (listed, listed_heads) =
        serve_tls(move |_| moved(&format!("https://localhost:{foreign}/")));
    let request = HttpRequest {
        secret: Some(AttachedSecret {
            header: "X-Token".to_string(),
            // A variable set in every test run: setting one here would race the other tests.
            env: "PATH".to_string(),
            scheme: None,
            hosts: vec!["127.0.0.1".to_string()],
        }),
        ..private(&format!("https://127.0.0.1:{listed}/"))
    };
    let outcome = trusting().request(&request, &Any);
    assert_eq!(outcome.class, OutcomeClass::Ok, "{outcome:?}");
    let head = listed_heads.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(head.contains("x-token:"), "the listed host gets it: {head}");
    let head = foreign_heads.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(
        !head.contains("x-token"),
        "the hop elsewhere does not: {head}"
    );
}

#[test]
fn resolving_a_dot_invalid_host_is_unreachable_dns() {
    let outcome = Client::new().resolve(&ResolveRequest {
        host: "no-such-host.invalid".to_string(),
        allow_private: false,
        timeout_ms: 10_000,
    });
    assert_eq!(outcome.class, OutcomeClass::Unreachable(Unreachable::Dns));
    assert!(outcome.addresses.is_empty());
}

#[test]
fn a_host_that_resolves_only_to_loopback_is_refused_address() {
    let asked = |allow_private| {
        Client::new().resolve(&ResolveRequest {
            host: "127.0.0.1".to_string(),
            allow_private,
            timeout_ms: 10_000,
        })
    };
    let refused = asked(false);
    assert_eq!(
        refused.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress)
    );
    assert!(refused.addresses.is_empty(), "no private address gets out");
    assert_eq!(asked(true).addresses, ["127.0.0.1"]);
}

#[test]
fn a_handshake_reads_its_certificate_whether_or_not_the_chain_verifies() {
    let (port, _) = serve_tls(|_| ok("hi"));
    let request = TlsRequest {
        host: "localhost".to_string(),
        tcp_port: port,
        allow_private: true,
        timeout_ms: 10_000,
    };
    let trusted = trusting().tls(&request);
    assert_eq!(trusted.class, OutcomeClass::Ok, "{trusted:?}");
    assert!(trusted.chain_valid);
    assert_eq!(trusted.issuer.as_deref(), Some("localhost"));
    assert!(trusted.not_after.is_some());

    let rejected = Client::new().tls(&request);
    assert_eq!(rejected.class, OutcomeClass::Unreachable(Unreachable::Tls));
    assert!(!rejected.chain_valid);
    assert_eq!(rejected.not_after, trusted.not_after, "the facts it read");
    assert_eq!(rejected.issuer, trusted.issuer);

    let refused = Client::new().tls(&TlsRequest {
        allow_private: false,
        ..request
    });
    assert_eq!(
        refused.class,
        OutcomeClass::Unreachable(Unreachable::RefusedAddress),
        "a handshake reaches no address the request does not allow"
    );
    assert_eq!(refused.not_after, None);
}

#[test]
fn a_host_that_drips_bytes_does_not_outlive_the_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        // The head of a long handshake record, then its body a byte at a time: each read gets one.
        let _ = stream.write_all(&[0x16, 0x03, 0x03, 0x40, 0x00]);
        let began = Instant::now();
        while began.elapsed() < Duration::from_secs(6) && stream.write_all(&[0]).is_ok() {
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    let began = Instant::now();
    let dripped = Client::new().tls(&TlsRequest {
        host: "localhost".to_string(),
        tcp_port: port,
        timeout_ms: 300,
        allow_private: true,
    });
    assert_eq!(
        dripped.class,
        OutcomeClass::Unreachable(Unreachable::Timeout)
    );
    assert!(
        began.elapsed() < Duration::from_secs(3),
        "{:?}",
        began.elapsed()
    );
}
