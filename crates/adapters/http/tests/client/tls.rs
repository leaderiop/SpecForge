//! The client over TLS against a loopback server under a test certificate: a rejection as one class, a secret
//! and its hosts, a handshake's facts and deadline; and the resolver's address classes.

use super::*;

#[test]
fn tls_rejection_is_one_class() {
    let cert =
        rustls::pki_types::CertificateDer::from(include_bytes!("../self-signed.cert.der").to_vec());
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
        include_bytes!("../self-signed.key.der").to_vec().into(),
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

/// A TLS server on loopback answering each connection as [`serve`] does, under the certificate for `localhost` and
/// `127.0.0.1` that [`trusting`] trusts. Its port.
fn serve_tls(answer: impl Fn(&str) -> String + Send + 'static) -> (u16, Receiver<String>) {
    let cert = rustls::pki_types::CertificateDer::from(
        include_bytes!("../self-signed-leaf.cert.der").to_vec(),
    );
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
        include_bytes!("../self-signed.key.der").to_vec().into(),
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
            include_bytes!("../self-signed-leaf.cert.der").to_vec(),
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
    let head = listed_heads.recv_timeout(WAIT).unwrap();
    assert!(head.contains("x-token:"), "the listed host gets it: {head}");
    let head = foreign_heads.recv_timeout(WAIT).unwrap();
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
        timeout_ms: WAIT_MS,
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
            timeout_ms: WAIT_MS,
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
        timeout_ms: WAIT_MS,
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
        // The head of a long handshake record, then its body a byte at a time until the client hangs up: each read
        // gets one, so only the handshake's whole deadline ends it.
        let _ = stream.write_all(&[0x16, 0x03, 0x03, 0x40, 0x00]);
        while stream.write_all(&[0]).is_ok() {
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    let dripped = short().tls(&TlsRequest {
        host: "localhost".to_string(),
        tcp_port: port,
        timeout_ms: WAIT_MS,
        allow_private: true,
    });
    assert_eq!(
        dripped.class,
        OutcomeClass::Unreachable(Unreachable::Timeout)
    );
}

/// The TLS configuration loads the platform's trust store, seconds on a loaded machine: it is built once per client,
/// within the deadline of the first handshake that needs it, and shared by every later handshake and request.
#[test]
fn a_handshake_builds_the_tls_configuration_once_within_its_deadline() {
    let (port, _) = serve_tls(|_| ok("hi"));
    let (release, released) = channel::<()>();
    let released = Mutex::new(released);
    let (configs, past_hold) = (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
    let (counted, passed) = (Arc::clone(&configs), Arc::clone(&past_hold));
    let client = Client::new().with_before_build(move |build| {
        if build == Build::Tls {
            counted.fetch_add(1, Ordering::SeqCst);
            // Held until the test lets it go; at most WAIT, so a failing test still ends.
            let _ = released.lock().unwrap().recv_timeout(WAIT);
            passed.store(true, Ordering::SeqCst);
        }
    });
    let request = TlsRequest {
        host: "localhost".to_string(),
        tcp_port: port,
        allow_private: true,
        timeout_ms: WAIT_MS,
    };

    let first = client.tls(&TlsRequest {
        timeout_ms: 300,
        ..request.clone()
    });

    assert_eq!(
        first.class,
        OutcomeClass::Unreachable(Unreachable::Timeout),
        "{first:?}"
    );
    assert!(
        !past_hold.load(Ordering::SeqCst),
        "the handshake waited for the build"
    );
    release.send(()).unwrap();
    // The platform's verifier refuses the test's certificate: each handshake reaches it, on the one configuration.
    for _ in 0..3 {
        let shaken = client.tls(&request);
        assert_eq!(
            shaken.class,
            OutcomeClass::Unreachable(Unreachable::Tls),
            "{shaken:?}"
        );
    }
    let requested = client.request(&private(&format!("https://localhost:{port}/")), &Any);
    assert_eq!(
        requested.class,
        OutcomeClass::Unreachable(Unreachable::Tls),
        "{requested:?}"
    );
    assert_eq!(
        configs.load(Ordering::SeqCst),
        1,
        "one configuration, shared by the handshakes and the request's client"
    );
}
