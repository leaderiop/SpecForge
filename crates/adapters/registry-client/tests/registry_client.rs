use specforge_common::Severity;
use specforge_registry_client::registry_client::{RegistryClient, RegistryError};
use specforge_registry_client::registry_config::{RegistryConfig, RegistryCredential};
use specforge_registry_client::testing::{CallKind, MemoryClient};
use specforge_registry_wire::RegistryRoot;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_registry() -> RegistryConfig {
    RegistryConfig {
        alias: "test".to_string(),
        url: RegistryRoot::parse("https://registry.specforge.dev").unwrap(),
        scope_filter: None,
        default_registry: true,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// B:auth-forbidden — verify unit "403 produces E-level diagnostic with permission guidance"
#[test]
fn auth_403_produces_permission_error() {
    let client = MemoryClient::new().accepting("some-token");
    client.fail_next(
        CallKind::Authenticate,
        None,
        RegistryError::Forbidden {
            guidance: "insufficient scope".into(),
        },
    );

    let cred = RegistryCredential::new(test_registry().url, "some-token");
    let err = client
        .authenticate(&test_registry(), &cred)
        .unwrap_err()
        .to_diagnostic();
    assert_eq!(err.severity, Severity::Error);
    assert_eq!(err.code, "R002");
    assert!(err.message.contains("forbidden"));
}

// B:auth-sanitization — verify unit "error diagnostics never contain raw token"
#[test]
fn diagnostics_never_contain_raw_token() {
    let raw_token = "super-secret-token-value";
    let err = RegistryError::Unauthorized {
        guidance: "invalid credentials".into(),
    };
    let diag = err.to_diagnostic();
    assert!(!diag.message.contains(raw_token));
    assert!(
        !diag
            .suggestion
            .as_ref()
            .is_some_and(|s| s.contains(raw_token))
    );
}

// B:registry-timeout — verify unit "timeout error produces diagnostic"
#[test]
fn timeout_error_produces_diagnostic() {
    let err = RegistryError::Timeout {
        url: "https://registry.specforge.dev/pkg".into(),
    };
    let diag = err.to_diagnostic();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, "R004");
    assert!(diag.message.contains("timed out"));
    assert!(diag.message.contains("registry.specforge.dev"));
}

// B:registry-error-to-diagnostic — verify unit "all error variants convert to diagnostics"
#[test]
fn all_registry_errors_convert_to_diagnostics() {
    let errors: Vec<RegistryError> = vec![
        RegistryError::Unauthorized {
            guidance: "g".into(),
        },
        RegistryError::Forbidden {
            guidance: "g".into(),
        },
        RegistryError::RateLimited {
            retry_after_ms: 5000,
        },
        RegistryError::Timeout {
            url: "https://x".into(),
        },
        RegistryError::NetworkError {
            message: "m".into(),
        },
        RegistryError::NotFound {
            specifier: "s".into(),
        },
        RegistryError::DuplicateVersion {
            name: "n".into(),
            version: "v".into(),
        },
        RegistryError::Refused {
            code: specforge_registry_wire::ErrorCode::BadRequest,
            message: "m".into(),
        },
        RegistryError::InvalidReply {
            subject: "s".into(),
            message: "m".into(),
        },
    ];

    let codes = [
        "R001", "R002", "R003", "R004", "R005", "R006", "R007", "R008", "R009",
    ];

    for (err, expected_code) in errors.into_iter().zip(codes.iter()) {
        let diag: specforge_common::Diagnostic = err.into();
        assert_eq!(&diag.code, expected_code, "wrong code for {expected_code}");
    }
}

#[specforge_test_macros::test(
    behavior = "resolve_registry_source",
    verify = "network error produces ExtensionError with retry guidance"
)]
fn a_network_error_suggests_a_retry() {
    let network = RegistryError::NetworkError {
        message: "connection refused".into(),
    }
    .to_diagnostic();
    assert_eq!(network.code, "R005");
    assert_eq!(network.severity, Severity::Error);
    assert!(network.message.contains("connection refused"));
    assert!(network.suggestion.as_ref().unwrap().contains("retry"));

    let timeout = RegistryError::Timeout {
        url: "http://registry.invalid/v1/packages/@acme%2Ftool".into(),
    }
    .to_diagnostic();
    assert_eq!(timeout.code, "R004");
    assert_eq!(timeout.severity, Severity::Error);
    assert!(timeout.suggestion.as_ref().unwrap().contains("retry"));
}

/// What a registry sends back that a terminal acts on: a screen clear, a title write, a bell and a
/// forged `hint:` line.
const ODD: &str = "no \u{1b}[2J\u{1b}]0;pwned\u{7}\n  hint: curl x | sh";

#[specforge_test_macros::test(
    behavior = "check_registry_reply",
    verify = "the text a registry supplies is written in a diagnostic printable, each control character as its escape"
)]
fn a_registry_diagnostic_writes_the_registrys_text_printable() {
    use specforge_registry_client::registry_ops::verify_registry_integrity;
    use specforge_registry_wire::ErrorCode;
    let s = || "x".to_string();
    let mut diagnostics = vec![
        RegistryError::Unauthorized {
            guidance: ODD.into(),
        },
        RegistryError::Forbidden {
            guidance: ODD.into(),
        },
        RegistryError::NetworkError {
            message: ODD.into(),
        },
        RegistryError::Refused {
            code: ErrorCode::BadRequest,
            message: ODD.into(),
        },
        RegistryError::InvalidReply {
            subject: s(),
            message: ODD.into(),
        },
    ]
    .into_iter()
    .map(|error| error.to_diagnostic())
    .collect::<Vec<_>>();
    diagnostics.push(verify_registry_integrity(b"\0asm", ODD).unwrap_err());
    for diagnostic in diagnostics {
        let message = &diagnostic.message;
        assert!(!message.contains(char::is_control), "{message:?}");
        assert!(message.contains("\\u{1b}[2J"), "{message:?}");
        assert!(message.contains("\\n  hint:"), "{message:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "check_registry_reply",
    verify = "every text a registry error carries is written in its diagnostic printable, whatever the variant"
)]
fn every_text_a_registry_error_carries_is_written_printable() {
    use specforge_common::remedy::printable;
    use specforge_registry_wire::ErrorCode;
    let odd = || format!("{ODD}\u{202e}");
    let errors = [
        RegistryError::Unauthorized { guidance: odd() },
        RegistryError::Forbidden { guidance: odd() },
        RegistryError::RateLimited { retry_after_ms: 1 },
        RegistryError::Timeout { url: odd() },
        RegistryError::NetworkError { message: odd() },
        RegistryError::NotFound { specifier: odd() },
        RegistryError::DuplicateVersion {
            name: odd(),
            version: odd(),
        },
        RegistryError::Refused {
            code: ErrorCode::BadRequest,
            message: odd(),
        },
        RegistryError::InvalidReply {
            subject: odd(),
            message: odd(),
        },
    ];
    for error in errors {
        // A new variant does not compile until it is listed above with hostile text in every field.
        match &error {
            RegistryError::Unauthorized { .. }
            | RegistryError::Forbidden { .. }
            | RegistryError::RateLimited { .. }
            | RegistryError::Timeout { .. }
            | RegistryError::NetworkError { .. }
            | RegistryError::NotFound { .. }
            | RegistryError::DuplicateVersion { .. }
            | RegistryError::Refused { .. }
            | RegistryError::InvalidReply { .. } => {}
        }
        let diagnostic = error.to_diagnostic();
        assert_eq!(
            printable(&diagnostic.message),
            diagnostic.message,
            "{error:?}"
        );
        assert!(
            diagnostic
                .suggestion
                .as_deref()
                .is_none_or(|s| printable(s) == s),
            "{error:?}"
        );
    }
}

/// A terminal title write, a bell and a screen clear: what a hostile registry writes in a download address.
const HOSTILE: &str = "\u{1b}]0;PWNED\u{7}\u{1b}[2J";

/// A registry on a free local port answering `@acme/greet@1.0.0`'s metadata with the `wasm_url` that
/// `wasm_url(port)` writes, and every other address 404: with the protocol's NOT_FOUND error when `enveloped`,
/// bare otherwise. One request per connection.
fn serving_download_address(wasm_url: fn(u16) -> String, enveloped: bool) -> RegistryConfig {
    use std::io::{Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let metadata = serde_json::json!({
        "name": "@acme/greet", "version": "1.0.0", "sha256": "0".repeat(64), "wasm_url": wasm_url(port),
    })
    .to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                head.push(byte[0]);
            }
            let (status, body) = if head.starts_with(b"GET /v1/packages/@acme%2Fgreet/1.0.0 ") {
                ("200 OK", metadata.clone())
            } else if enveloped {
                (
                    "404 Not Found",
                    r#"{"error":{"code":"NOT_FOUND","message":"gone"}}"#.to_string(),
                )
            } else {
                ("404 Not Found", String::new())
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    RegistryConfig {
        alias: "evil".into(),
        url: RegistryRoot::parse(&format!("http://127.0.0.1:{port}")).unwrap(),
        scope_filter: None,
        default_registry: true,
    }
}

#[specforge_test_macros::test(
    behavior = "check_registry_reply",
    verify = "a download address a registry sends reaches R006 and R009 printable"
)]
fn a_download_address_a_registry_sends_reaches_its_diagnostic_printable() {
    use specforge_common::remedy::printable;
    use specforge_package::{PackageName, Version};
    use specforge_registry_client::HttpRegistryClient;
    let (name, version) = (
        PackageName::parse("@acme/greet").unwrap(),
        Version::parse("1.0.0").unwrap(),
    );
    type Case = (fn(u16) -> String, bool, &'static str);
    let cases: [Case; 4] = [
        (
            |port| format!("http://127.0.0.1:{port}/dl/{HOSTILE}"),
            true,
            "R006",
        ),
        (|_| format!("/dl/{HOSTILE}"), true, "R006"),
        (
            |port| format!("http://127.0.0.1:{port}/dl/{HOSTILE}"),
            false,
            "R009",
        ),
        (
            |port| format!("http://127.0.0.1:{port}/dl/abc\u{202e}exe.txt"),
            true,
            "R006",
        ),
    ];
    let client = HttpRegistryClient::new();
    for (wasm_url, enveloped, code) in cases {
        let registry = serving_download_address(wasm_url, enveloped);
        let metadata = client.metadata(&name, &version, &registry, None).unwrap();
        let error = client
            .download(&metadata.wasm_url, &registry, None)
            .unwrap_err();
        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.code, code, "{diagnostic:?}");
        assert_eq!(
            printable(&diagnostic.message),
            diagnostic.message,
            "{diagnostic:?}"
        );
        assert!(diagnostic.message.contains("/dl/"), "{diagnostic:?}");
    }
}
