//! The `Http` port: one request, its redirects followed by the adapter under the caller's policy (ADR 0335 D1). Its
//! adapter is `specforge-http`.

use std::sync::Arc;
use std::time::Duration;

use specforge_package::MAX_PACKAGE_BYTES;
use specforge_protocol_types::{
    AttachedSecret, HttpRequest, OutcomeClass, ResolveRequest, TlsRequest, Unreachable,
};
use url::Url;

/// Connecting gives up after this.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// A request's whole time when it names none.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
/// No request takes longer, whatever it asks.
pub const TIMEOUT_CEILING: Duration = Duration::from_secs(60);
/// The most redirects a request may follow.
pub const MAX_REDIRECTS: u32 = 5;
/// The most body bytes any request reads.
pub const BODY_CEILING: u64 = MAX_PACKAGE_BYTES;

/// The body cap of an answer copied into a guest whose memory is `max_memory_mb`: a quarter of it, at most the
/// ceiling, so a body can never fill the guest. A guest's `http` import is its caller.
pub fn guest_body_cap(max_memory_mb: u64) -> u64 {
    BODY_CEILING.min(max_memory_mb.saturating_mul(1024 * 1024) / 4)
}

/// Performs one request. Every adapter refuses a redirect from `https` to `http` (`redirect:downgrade`), one the
/// policy refuses (`redirect:refused`) and one past `max_redirects` (`redirect:limit`); reads at most `body_cap`
/// bytes (`capped` past it); and answers a transport failure as `unreachable:*`, never a panic.
pub trait Http: Send + Sync {
    fn request(&self, request: &HttpRequest, redirects: &dyn Redirects) -> HttpOutcome;
    /// A name's addresses. No answer is `unreachable:dns`; a name left with no address the request allows is
    /// `unreachable:refused_address`, and no address it does not allow is ever answered.
    fn resolve(&self, request: &ResolveRequest) -> ResolveOutcome;
    /// A TLS handshake under the port's verifier, and what the leaf certificate says. A rejection is `unreachable:tls`
    /// with `chain_valid: false` and the facts it read.
    fn tls(&self, request: &TlsRequest) -> TlsOutcome;
}

/// An HTTP port as the composition root shares it.
pub type SharedHttp = Arc<dyn Http>;

/// The method a redirect answered `status` to a `method` request is sent with: a `GET` after a 303, or after a 301 or
/// 302 from a `POST`, as browsers do, and its own otherwise. A hop sent as a `GET` carries no body. Every adapter sends
/// each hop with it and every grant judges the hop by it, so what is judged is what is sent.
pub fn hop_method(status: u16, method: &str) -> &str {
    if status == 303 || (matches!(status, 301 | 302) && method.eq_ignore_ascii_case("POST")) {
        "GET"
    } else {
        method
    }
}

/// Which redirects a request may follow, asked of each hop with the method it will be sent with ([`hop_method`]).
pub trait Redirects: Send + Sync {
    fn allow(&self, from: &str, to: &str, method: &str) -> bool;
}

/// Any target.
pub struct Any;

impl Redirects for Any {
    fn allow(&self, _from: &str, _to: &str, _method: &str) -> bool {
        true
    }
}

/// A target of the same scheme.
pub struct SameScheme;

impl Redirects for SameScheme {
    fn allow(&self, from: &str, to: &str, _method: &str) -> bool {
        scheme(from) == scheme(to)
    }
}

/// The header a secret adds to a hop. Its value never shows in debug output.
#[derive(Clone, PartialEq, Eq)]
pub struct SentHeader {
    pub name: String,
    pub value: String,
}

impl std::fmt::Debug for SentHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentHeader")
            .field("name", &self.name)
            .field("value", &"<redacted>")
            .finish()
    }
}

/// The header `secret` adds to a hop to `url` (ADR 0335 D6): `<scheme> <value>` or the bare value, the value as
/// `read` finds the secret's environment variable now. None unless the hop is `https` to a host the capability
/// lists, and none while the variable is unset (the response decides). Every adapter asks it of each hop, so a
/// redirect elsewhere carries nothing.
pub fn secret_header(
    secret: &AttachedSecret,
    url: &Url,
    read: impl Fn(&str) -> Option<String>,
) -> Option<SentHeader> {
    let host = url.host_str()?;
    let listed = secret
        .hosts
        .iter()
        .any(|pattern| crate::ceiling::host_covers(pattern, host));
    if url.scheme() != "https" || !listed {
        return None;
    }
    let value = read(&secret.env).filter(|value| !value.is_empty())?;
    Some(SentHeader {
        name: secret.header.clone(),
        value: match &secret.scheme {
            Some(scheme) => format!("{scheme} {value}"),
            None => value,
        },
    })
}

/// The host of `url` as a request would reach it (WHATWG); none when it names none.
pub fn host_of(url: &str) -> Option<String> {
    Url::parse(url).ok()?.host_str().map(str::to_string)
}

/// The scheme of `url`, lowercased; empty when it has none.
pub fn scheme(url: &str) -> String {
    url.split_once("://")
        .map(|(scheme, _)| scheme.to_ascii_lowercase())
        .unwrap_or_default()
}

/// What a name resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveOutcome {
    pub class: OutcomeClass,
    pub addresses: Vec<String>,
}

impl ResolveOutcome {
    pub fn of(class: OutcomeClass) -> Self {
        ResolveOutcome {
            class,
            addresses: Vec::new(),
        }
    }
}

/// What a handshake read of a host's certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsOutcome {
    pub class: OutcomeClass,
    /// When the leaf certificate expires, in UTC (`2030-01-31T23:59:59Z`).
    pub not_after: Option<String>,
    /// The leaf's issuer: its common name, when it names one.
    pub issuer: Option<String>,
    pub chain_valid: bool,
}

impl TlsOutcome {
    pub fn of(class: OutcomeClass) -> Self {
        TlsOutcome {
            class,
            not_after: None,
            issuer: None,
            chain_valid: false,
        }
    }
}

/// What a request answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpOutcome {
    /// `ok` for a 2xx, `status:<code>` for another answer, else why there was none.
    pub class: OutcomeClass,
    /// The final answer's status, when one came.
    pub status: Option<u16>,
    /// The URL that answered, after the redirects followed.
    pub final_url: String,
    /// The response headers the request's `keep_headers` names, lowercased, in the order they came.
    pub headers: Vec<(String, String)>,
    /// At most the request's `body_cap` bytes.
    pub body: Vec<u8>,
    /// For a person: what the transport said, when the class is a failure.
    pub detail: Option<String>,
}

impl HttpOutcome {
    /// No answer from `url`, for `why`.
    pub fn unreachable(url: &str, why: Unreachable, detail: impl Into<String>) -> Self {
        HttpOutcome::failed(url, OutcomeClass::Unreachable(why), detail)
    }

    /// No answer from `url`, of `class`.
    pub fn failed(url: &str, class: OutcomeClass, detail: impl Into<String>) -> Self {
        HttpOutcome {
            class,
            status: None,
            final_url: url.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
            detail: Some(detail.into()),
        }
    }

    /// The kept header `name` (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// The class of an answer with `status`.
pub fn status_class(status: u16) -> OutcomeClass {
    if (200..300).contains(&status) {
        OutcomeClass::Ok
    } else {
        OutcomeClass::Status(status)
    }
}

/// A `GET` of `url` with the default timeout, redirect count and the full body ceiling.
pub fn get(url: impl Into<String>) -> HttpRequest {
    HttpRequest {
        method: "GET".into(),
        url: url.into(),
        body_cap: BODY_CEILING,
        timeout_ms: DEFAULT_TIMEOUT.as_millis() as u64,
        max_redirects: MAX_REDIRECTS,
        ..HttpRequest::default()
    }
}
