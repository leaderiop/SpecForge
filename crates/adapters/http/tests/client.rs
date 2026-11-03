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
use specforge_effects::{Header, Http, HttpRequest, OutcomeClass, Redirected, Unreachable};
use specforge_http::Client;

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

/// A `GET` of `url` that may reach loopback.
fn private(url: &str) -> HttpRequest {
    HttpRequest {
        allow_private: true,
        ..get(url)
    }
}

#[test]
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
