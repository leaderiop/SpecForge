//! The adapter behind the effect plane's `Http` port (ADR 0335): one blocking reqwest client, built on first use, on
//! rustls with ring and the platform verifier. It follows redirects itself under the caller's policy, refuses a
//! private address unless the request allows one (judged here, for through a proxy the proxy resolves the name),
//! holds the whole request to one timeout and reads at most the body cap. A transport failure is an outcome class,
//! never a panic.

use std::error::Error as StdError;
use std::io::Read as _;
use std::net::IpAddr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use reqwest::blocking::{Client as Reqwest, Response};
use reqwest::header::{HeaderValue, LOCATION};
use reqwest::{Method, Url};
use specforge_effects::http::{
    BODY_CEILING, CONNECT_TIMEOUT, TIMEOUT_CEILING, hop_method, status_class,
};
use specforge_effects::{
    Http, HttpOutcome, HttpRequest, OutcomeClass, Redirected, Redirects, ResolveOutcome,
    ResolveRequest, TlsOutcome, TlsRequest, Unreachable,
};

mod address;
mod handshake;

pub use address::public;
use address::{Failed, PublicOnly, destination};

/// The `Http` port over the network. Its two reqwest clients (one that refuses private addresses, one that allows
/// them) are built on first use, so a command that never reaches the network loads no trust store.
#[derive(Default)]
pub struct Client {
    public: OnceLock<Result<Reqwest, String>>,
    any: OnceLock<Result<Reqwest, String>>,
    /// The TLS configuration to use in place of the platform's verifier.
    tls: Option<rustls::ClientConfig>,
    /// The proxy every request goes through, in place of the environment's and the system's.
    proxy: Option<String>,
}

impl Client {
    pub fn new() -> Self {
        Client::default()
    }

    /// A client that verifies with `tls`, not the platform's verifier: for a test's own certificate.
    pub fn with_tls(tls: rustls::ClientConfig) -> Self {
        Client {
            tls: Some(tls),
            ..Client::default()
        }
    }

    /// A client that sends every request through the proxy at `url`: for a test's own proxy, whose environment no
    /// test may change.
    pub fn with_proxy(url: &str) -> Self {
        Client {
            proxy: Some(url.to_string()),
            ..Client::default()
        }
    }

    fn client(&self, allow_private: bool) -> Result<&Reqwest, &str> {
        let cell = if allow_private {
            &self.any
        } else {
            &self.public
        };
        cell.get_or_init(|| build(allow_private, self.tls.clone(), self.proxy.as_deref()))
            .as_ref()
            .map_err(String::as_str)
    }
}

/// A reqwest client that follows no redirect, connects within [`CONNECT_TIMEOUT`] and, unless `allow_private`,
/// resolves names to public addresses only.
fn build(
    allow_private: bool,
    config: Option<rustls::ClientConfig>,
    proxy: Option<&str>,
) -> Result<Reqwest, String> {
    let config = match config {
        Some(config) => config,
        None => tls()?,
    };
    let mut builder = Reqwest::builder()
        .tls_backend_preconfigured(config)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TIMEOUT_CEILING);
    if !allow_private {
        builder = builder.dns_resolver(Arc::new(PublicOnly::system()));
    }
    if let Some(proxy) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|e| chain(&e))?);
    }
    builder.build().map_err(|e| chain(&e))
}

/// rustls on ring with the platform's verifier; on Linux, Mozilla's roots are added to the system's, so a host with
/// none still verifies (one verifier, never a second attempt after a rejection).
fn tls() -> Result<rustls::ClientConfig, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    #[cfg(target_os = "linux")]
    let verifier = rustls_platform_verifier::Verifier::new_with_extra_roots(
        webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().cloned(),
        provider.clone(),
    );
    #[cfg(not(target_os = "linux"))]
    let verifier = rustls_platform_verifier::Verifier::new(provider.clone());
    let verifier = verifier.map_err(|e| e.to_string())?;
    Ok(rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth())
}

impl Http for Client {
    fn request(&self, request: &HttpRequest, redirects: &dyn Redirects) -> HttpOutcome {
        let deadline =
            Instant::now() + Duration::from_millis(request.timeout_ms).min(TIMEOUT_CEILING);
        let client = match self.client(request.allow_private) {
            Ok(client) => client,
            Err(why) => return HttpOutcome::unreachable(&request.url, Unreachable::Tls, why),
        };
        let mut url = request.url.clone();
        let mut method = request.method.clone();
        let mut body = request.body.clone();
        let mut headers = request.headers.clone();
        let mut hops = 0;
        loop {
            let parsed = match Url::parse(&url) {
                Ok(parsed) => parsed,
                Err(e) => {
                    return HttpOutcome::unreachable(
                        &url,
                        Unreachable::Connect,
                        format!("not a URL: {e}"),
                    );
                }
            };
            if !request.allow_private && literal(&parsed).is_some_and(|ip| !public(ip)) {
                return HttpOutcome::unreachable(
                    &url,
                    Unreachable::RefusedAddress,
                    "a loopback, private, link-local or unspecified address",
                );
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return HttpOutcome::unreachable(
                    &url,
                    Unreachable::Timeout,
                    "the request's time ran out",
                );
            }
            if !request.allow_private
                && let Some(url::Host::Domain(name)) = parsed.host()
                && let Err(refused) = destination(name, left.min(CONNECT_TIMEOUT))
            {
                return HttpOutcome::unreachable(
                    &url,
                    Unreachable::RefusedAddress,
                    refused.to_string(),
                );
            }
            let left = deadline.saturating_duration_since(Instant::now());
            let Ok(verb) = Method::from_bytes(method.as_bytes()) else {
                return HttpOutcome::unreachable(
                    &url,
                    Unreachable::Connect,
                    format!("not a method: {method}"),
                );
            };
            let mut asked = client.request(verb, parsed.clone()).timeout(left);
            let secret = secret_of(request, &parsed);
            for header in &headers {
                if secret
                    .as_ref()
                    .is_some_and(|(name, _)| name.eq_ignore_ascii_case(&header.name))
                {
                    continue;
                }
                asked = asked.header(&header.name, &header.value);
            }
            if let Some((name, value)) = secret {
                asked = asked.header(name, value);
            }
            if let Some(body) = &body {
                asked = asked.body(body.clone());
            }
            let response = match asked.send() {
                Ok(response) => response,
                Err(e) => return HttpOutcome::unreachable(&url, class_of(&e), chain(&e)),
            };
            let status = response.status().as_u16();
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|l| l.to_str().ok());
            let (true, Some(location)) = ((300..400).contains(&status), location) else {
                return answer(response, request, url, deadline);
            };
            let next = hop_method(status, &method).to_string();
            let to = match hop(
                &parsed,
                location,
                hops,
                request.max_redirects,
                &next,
                redirects,
            ) {
                Ok(to) => to,
                Err((why, detail)) => {
                    return HttpOutcome {
                        status: Some(status),
                        ..HttpOutcome::failed(&url, OutcomeClass::Redirect(why), detail)
                    };
                }
            };
            if parsed.origin() != to.origin() {
                headers.retain(|header| !header.sensitive);
            }
            if next == "GET" {
                body = None;
            }
            method = next;
            hops += 1;
            url = to.into();
        }
    }

    fn resolve(&self, request: &ResolveRequest) -> ResolveOutcome {
        handshake::resolve(request)
    }

    fn tls(&self, request: &TlsRequest) -> TlsOutcome {
        let config = self.tls.clone().map_or_else(tls, Ok);
        match config {
            Ok(config) => handshake::tls(config, request),
            Err(_) => TlsOutcome::of(OutcomeClass::Unreachable(Unreachable::Tls)),
        }
    }
}

/// The header the grant's secret adds to the hop to `url`, its value read from the environment now (ADR 0335 D6):
/// asked of every hop, so a redirect to a host the capability does not list, or off `https`, carries none. The
/// value is marked sensitive, so no debug output of the request shows it.
fn secret_of(request: &HttpRequest, url: &Url) -> Option<(String, HeaderValue)> {
    let secret = request.secret.as_ref()?;
    let header =
        specforge_effects::http::secret_header(secret, url, |env| std::env::var(env).ok())?;
    let mut value = HeaderValue::from_str(&header.value).ok()?;
    value.set_sensitive(true);
    Some((header.name, value))
}

/// The IP address `url` names literally, if it does.
fn literal(url: &Url) -> Option<IpAddr> {
    match url.host()? {
        url::Host::Ipv4(ip) => Some(IpAddr::V4(ip)),
        url::Host::Ipv6(ip) => Some(IpAddr::V6(ip)),
        url::Host::Domain(_) => None,
    }
}

/// Where a redirect from `from` to `location` leads, the `hops`th of at most `max`: refused past the limit, from
/// `https` to anything else, or when `redirects` says no to it sent with `method`.
fn hop(
    from: &Url,
    location: &str,
    hops: u32,
    max: u32,
    method: &str,
    redirects: &dyn Redirects,
) -> Result<Url, (Redirected, String)> {
    let to = from
        .join(location)
        .map_err(|e| (Redirected::Refused, format!("Location {location}: {e}")))?;
    if hops >= max {
        return Err((Redirected::Limit, format!("more than {max} redirects")));
    }
    if from.scheme() == "https" && to.scheme() != "https" {
        return Err((Redirected::Downgrade, format!("{from} redirects to {to}")));
    }
    if !redirects.allow(from.as_str(), to.as_str(), method) {
        return Err((Redirected::Refused, format!("{from} redirects to {to}")));
    }
    Ok(to)
}

/// The final answer from `url`: its kept headers and at most the request's body cap, read before `deadline`.
fn answer(
    response: Response,
    request: &HttpRequest,
    url: String,
    deadline: Instant,
) -> HttpOutcome {
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter(|(name, _)| {
            request
                .keep_headers
                .iter()
                .any(|kept| kept.eq_ignore_ascii_case(name.as_str()))
        })
        .filter_map(|(name, value)| {
            Some((name.as_str().to_string(), value.to_str().ok()?.to_string()))
        })
        .collect();
    let mut outcome = HttpOutcome {
        class: status_class(status),
        status: Some(status),
        final_url: url,
        headers,
        body: Vec::new(),
        detail: None,
    };
    let cap = request.body_cap.min(BODY_CEILING);
    if response.content_length().is_some_and(|n| n > cap) {
        outcome.class = OutcomeClass::Capped;
        outcome.detail = Some(format!("the body is longer than {cap} bytes"));
        return outcome;
    }
    match body_of(response, cap, deadline, &mut outcome.body) {
        Ok(true) => {}
        Ok(false) => {
            outcome.class = OutcomeClass::Capped;
            outcome.detail = Some(format!("the body is longer than {cap} bytes"));
        }
        Err(why) => {
            outcome.class = OutcomeClass::Unreachable(why.0);
            outcome.detail = Some(why.1);
        }
    }
    outcome
}

/// Read at most `cap` bytes of `response` into `body`, all of it before `deadline` (ADR 0250): `Ok(true)` when the
/// whole body fit, `Ok(false)` when it is longer (`body` holds its first `cap` bytes). No more than `cap + 1` bytes
/// are ever held; the deadline is checked between reads, and one read waits at most the request's timeout.
fn body_of(
    response: Response,
    cap: u64,
    deadline: Instant,
    body: &mut Vec<u8>,
) -> Result<bool, (Unreachable, String)> {
    let mut reader = response.take(cap + 1);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let read = match reader.read(&mut chunk) {
            Ok(read) => read,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut || Instant::now() >= deadline => {
                return Err((
                    Unreachable::Timeout,
                    format!("the body was still arriving: {e}"),
                ));
            }
            Err(e) => return Err((Unreachable::Reset, format!("failed to read the body: {e}"))),
        };
        if read == 0 {
            return Ok(true);
        }
        body.extend_from_slice(&chunk[..read]);
        if body.len() as u64 > cap {
            body.truncate(cap as usize);
            return Ok(false);
        }
        if Instant::now() >= deadline {
            return Err((Unreachable::Timeout, "the body was still arriving".into()));
        }
    }
}

/// Why a request got no answer, the same on every OS (O10): any rustls or verifier rejection is `tls`.
fn class_of(error: &reqwest::Error) -> Unreachable {
    let mut source: Option<&(dyn StdError + 'static)> = Some(error);
    while let Some(e) = source {
        match e.downcast_ref::<Failed>() {
            Some(Failed::Dns(_)) => return Unreachable::Dns,
            Some(Failed::Refused(_)) => return Unreachable::RefusedAddress,
            None => {}
        }
        if e.is::<rustls::Error>() {
            return Unreachable::Tls;
        }
        // An `io::Error`'s `source` skips the error it wraps: step into it.
        source = match e
            .downcast_ref::<std::io::Error>()
            .and_then(|io| io.get_ref())
        {
            Some(inner) => Some(inner as &(dyn StdError + 'static)),
            None => e.source(),
        };
    }
    if error.is_timeout() {
        Unreachable::Timeout
    } else if error.is_connect() {
        Unreachable::Connect
    } else {
        Unreachable::Reset
    }
}

/// `error` and its causes, for a person.
fn chain(error: &dyn StdError) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(e) = source {
        let cause = e.to_string();
        if !text.contains(&cause) {
            text.push_str(": ");
            text.push_str(&cause);
        }
        source = e.source();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_effects::http::{Any, SameScheme};

    fn from(url: &str) -> Url {
        Url::parse(url).unwrap()
    }

    #[test]
    fn a_hop_from_https_to_http_is_a_downgrade() {
        let to = hop(
            &from("https://a.example/x"),
            "http://a.example/y",
            0,
            5,
            "GET",
            &Any,
        );
        assert_eq!(to.unwrap_err().0, Redirected::Downgrade);
        let to = hop(&from("https://a.example/x"), "/y", 0, 5, "GET", &SameScheme);
        assert_eq!(to.unwrap().as_str(), "https://a.example/y");
    }

    #[test]
    fn a_hop_past_the_limit_or_the_policy_is_refused() {
        assert_eq!(
            hop(&from("http://a.example/"), "/b", 2, 2, "GET", &Any)
                .unwrap_err()
                .0,
            Redirected::Limit
        );
        assert_eq!(
            hop(
                &from("http://a.example/"),
                "https://b.example/",
                0,
                2,
                "GET",
                &SameScheme
            )
            .unwrap_err()
            .0,
            Redirected::Refused
        );
    }
}
