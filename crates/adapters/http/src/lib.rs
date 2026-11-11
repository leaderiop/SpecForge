//! The adapter behind the effect plane's `Http` port (ADR 0335): one blocking reqwest client, built on first use
//! within the first request's deadline, on rustls with ring and the platform verifier, whose configuration is built once
//! per client, within the deadline of the first request or handshake that needs it (a failed build is not kept: the
//! next caller builds again). It follows redirects itself under the caller's policy, refuses a
//! private address unless the request allows one (judged here, for through a proxy the proxy resolves the name),
//! holds the whole request to one timeout and reads at most the body cap. A transport failure is an outcome class,
//! never a panic.

use std::error::Error as StdError;
use std::io::Read as _;
use std::net::IpAddr;
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use hyper_util::client::proxy::matcher::Matcher;
use reqwest::blocking::{Client as Reqwest, Response};
use reqwest::header::{HeaderValue, LOCATION};
use reqwest::{Method, Url};
use specforge_effects::http::{
    BODY_CEILING, CONNECT_TIMEOUT, TIMEOUT_CEILING, hop_method, status_class,
};
use specforge_effects::{
    Http, HttpOutcome, HttpRequest, OutcomeClass, Private, Redirected, Redirects, ResolveOutcome,
    ResolveRequest, TlsOutcome, TlsRequest, Unreachable,
};

mod address;
mod handshake;

pub use address::public;
use address::{Failed, PublicOnly, destination};

/// The `Http` port over the network. Its two reqwest clients (one that refuses private addresses, one that allows
/// them) and the TLS configuration both they and a handshake use are built on first use, so a command that never
/// reaches the network loads no trust store; a request or handshake waits for a build no longer than its own deadline
/// ([`Lazy`]).
#[derive(Default)]
pub struct Client {
    public: Arc<Lazy<Reqwest>>,
    any: Arc<Lazy<Reqwest>>,
    /// The TLS configuration: the platform's verifier, or a test's set by [`Client::with_tls`].
    config: Arc<Lazy<rustls::ClientConfig>>,
    /// The proxy every request goes through, in place of the environment's and the system's.
    proxy: Option<String>,
    /// The ceiling on a request's time in place of [`TIMEOUT_CEILING`] (only lower, and only a test's).
    ceiling: Option<Duration>,
    /// Run on a build's thread before it builds, given what it builds; an error fails that build: a test's.
    before_build: Option<Arc<Hook>>,
}

/// A test's hook run before a build ([`Client::with_before_build`]).
type Hook = dyn Fn(Build) -> Result<(), String> + Send + Sync;

/// What a [`Client`] builds on first use: what a test's [`Client::with_before_build`] hook is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    /// The TLS configuration, with the platform's verifier.
    Tls,
    /// A reqwest client, and whether it allows private addresses.
    Client { allow_private: bool },
}

/// One thing built on a thread of its own the first time a request needs it: a reqwest client, or the TLS
/// configuration. A build reads the trust store and the system's proxies, which takes seconds on a loaded machine: the
/// request that started it waits at most its own time left, and a later one takes what the build left. A failed build
/// is not kept: the callers waiting on it get its error, and the next caller starts another (a long-lived server
/// recovers once the trust store is readable).
struct Lazy<T> {
    built: OnceLock<T>,
    /// The builds started and how the last failed one failed; the lock [`Self::done`] is signalled under.
    state: Mutex<Builds>,
    done: Condvar,
}

#[derive(Default)]
struct Builds {
    /// Whether a build is under way.
    building: bool,
    /// The builds started so far: a build's number is the count when it started.
    started: u64,
    /// The last failed build's number and error.
    failed: Option<(u64, String)>,
}

impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Lazy {
            built: OnceLock::new(),
            state: Mutex::new(Builds::default()),
            done: Condvar::new(),
        }
    }
}

impl<T: Send + Sync + 'static> Lazy<T> {
    /// What `build` (started here, on a thread named `name`, if no build is under way) has made within `within`: its
    /// error when the build this call waited on failed, `None` when it has not ended.
    fn within(
        self: &Arc<Self>,
        name: &str,
        within: Duration,
        build: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Option<Result<&T, String>> {
        if let Some(built) = self.built.get() {
            return Some(Ok(built));
        }
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if self.built.get().is_none() && !state.building {
            state.building = true;
            state.started += 1;
            let (this, number) = (Arc::clone(self), state.started);
            let spawned = std::thread::Builder::new()
                .name(name.into())
                .spawn(move || {
                    let built = build();
                    let mut state = this.state.lock().unwrap_or_else(PoisonError::into_inner);
                    match built {
                        Ok(built) => {
                            let _ = this.built.set(built);
                        }
                        Err(why) => state.failed = Some((number, why)),
                    }
                    state.building = false;
                    this.done.notify_all();
                });
            if let Err(e) = spawned {
                state.building = false;
                state.failed = Some((number, format!("cannot start the {name} thread: {e}")));
            }
        }
        // The build under way is the latest started: this call takes its end (or a later failure's, should one fail
        // before this call wakes).
        let awaited = state.started;
        let failed = |state: &Builds| {
            state
                .failed
                .as_ref()
                .filter(|(number, _)| *number >= awaited)
                .map(|(_, why)| why.clone())
        };
        let (state, _) = self
            .done
            .wait_timeout_while(state, within, |state| {
                self.built.get().is_none() && failed(state).is_none()
            })
            .unwrap_or_else(PoisonError::into_inner);
        match self.built.get() {
            Some(built) => Some(Ok(built)),
            None => failed(&state).map(Err),
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Client::default()
    }

    /// A client that verifies with `tls`, not the platform's verifier: for a test's own certificate.
    pub fn with_tls(tls: rustls::ClientConfig) -> Self {
        let client = Client::default();
        let _ = client.config.built.set(tls);
        client
    }

    /// A client that sends every request through the proxy at `url`: for a test's own proxy, whose environment no
    /// test may change.
    pub fn with_proxy(url: &str) -> Self {
        Client {
            proxy: Some(url.to_string()),
            ..Client::default()
        }
    }

    /// This client with every request, resolve and handshake held to `ceiling` (when lower than
    /// [`TIMEOUT_CEILING`]): a test's request that must time out does so well within its runner's kill.
    #[cfg(any(test, feature = "testing"))]
    pub fn with_timeout_ceiling(mut self, ceiling: Duration) -> Self {
        self.ceiling = Some(ceiling);
        self
    }

    /// This client with `hook` run on a build's thread before each build, given what it builds: a test holds a build
    /// back to see a request wait for it no longer than its deadline, counts the builds, or fails one with the error
    /// it returns.
    #[cfg(any(test, feature = "testing"))]
    pub fn with_before_build(
        mut self,
        hook: impl Fn(Build) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        self.before_build = Some(Arc::new(hook));
        self
    }

    /// The ceiling on any request's time.
    fn ceiling(&self) -> Duration {
        self.ceiling
            .map_or(TIMEOUT_CEILING, |c| c.min(TIMEOUT_CEILING))
    }

    /// The host of the proxy a request to `url` goes through: this client's own, else the system's or the
    /// environment's, read as reqwest reads them; none when it goes direct.
    fn proxy_host(&self, url: &Url) -> Option<String> {
        if let Some(proxy) = &self.proxy {
            return Some(
                Url::parse(proxy)
                    .ok()
                    .and_then(|proxy| proxy.host_str().map(str::to_string))
                    .unwrap_or_else(|| proxy.clone()),
            );
        }
        let uri = url.as_str().parse::<http::Uri>().ok()?;
        let intercept = system_proxies().intercept(&uri)?;
        // A proxy URI with no host is still a proxy: judged by an empty name, which no grant reaches.
        Some(intercept.uri().host().unwrap_or_default().to_string())
    }

    /// The reqwest client of `allow_private`, built within `within` ([`Lazy`]): `None` when its build outlasts it.
    fn client(&self, allow_private: bool, within: Duration) -> Option<Result<&Reqwest, String>> {
        let (lazy, name) = if allow_private {
            (&self.any, "http-any-client-build")
        } else {
            (&self.public, "http-public-client-build")
        };
        let (config, proxy, hook) = (
            Arc::clone(&self.config),
            self.proxy.clone(),
            self.before_build.clone(),
        );
        lazy.within(name, within, move || {
            if let Some(hook) = &hook {
                hook(Build::Client { allow_private })?;
            }
            // The system's proxies are read here too, so a request's own reading of them never waits.
            if proxy.is_none() {
                system_proxies();
            }
            // This build's thread has no deadline of its own: the request waiting on it holds to its own.
            let config = config
                .within(TLS_BUILD, Duration::MAX, move || {
                    built_config(hook.as_deref())
                })
                .ok_or("the TLS configuration was not built")??
                .clone();
            build(allow_private, config, proxy.as_deref())
        })
    }

    /// The TLS configuration, built within `within` ([`Lazy`]): `None` when its build outlasts it.
    fn config(&self, within: Duration) -> Option<Result<&rustls::ClientConfig, String>> {
        let hook = self.before_build.clone();
        self.config
            .within(TLS_BUILD, within, move || built_config(hook.as_deref()))
    }
}

/// The name of the TLS configuration's build thread.
const TLS_BUILD: &str = "http-tls-build";

/// The platform's TLS configuration ([`tls`]), `hook` run first.
fn built_config(hook: Option<&Hook>) -> Result<rustls::ClientConfig, String> {
    if let Some(hook) = hook {
        hook(Build::Tls)?;
    }
    tls()
}

/// The system's and the environment's proxies, read once, as reqwest reads them.
fn system_proxies() -> &'static Matcher {
    static SYSTEM: OnceLock<Matcher> = OnceLock::new();
    SYSTEM.get_or_init(Matcher::from_system)
}

/// A reqwest client on `config` that follows no redirect, connects within [`CONNECT_TIMEOUT`] and, unless
/// `allow_private`, resolves names to public addresses only.
fn build(
    allow_private: bool,
    config: rustls::ClientConfig,
    proxy: Option<&str>,
) -> Result<Reqwest, String> {
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
        // Started before anything else: the deadline covers building the client too.
        let deadline = Instant::now() + handshake::time_of(request.timeout_ms, self.ceiling());
        let typed = Url::parse(&request.url).map(|url| url.origin()).ok();
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
            // A hop to another origin than the typed one is the server's choice, held to the public rule.
            let private = match request.allow_private {
                Private::Never => false,
                Private::TypedOrigin => typed.as_ref() == Some(&parsed.origin()),
                Private::Any => true,
            };
            let client =
                match self.client(private, deadline.saturating_duration_since(Instant::now())) {
                    Some(Ok(client)) => client,
                    Some(Err(why)) => return HttpOutcome::unreachable(&url, Unreachable::Tls, why),
                    None => {
                        return HttpOutcome::unreachable(
                            &url,
                            Unreachable::Timeout,
                            "the request's time ran out while its client was being built",
                        );
                    }
                };
            if !private && literal(&parsed).is_some_and(|ip| !public(ip)) {
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
            if !private
                && let Some(url::Host::Domain(name)) = parsed.host()
                && let Err(refused) = destination(name, left.min(CONNECT_TIMEOUT))
            {
                return HttpOutcome::unreachable(
                    &url,
                    Unreachable::RefusedAddress,
                    refused.to_string(),
                );
            }
            // The proxy sees what the hop carries, so it is the caller's to permit as the destination is.
            if let Some(proxy) = self.proxy_host(&parsed)
                && !redirects.through_proxy(&proxy, parsed.as_str())
            {
                return HttpOutcome::failed(
                    &url,
                    OutcomeClass::Denied,
                    format!("{url} goes through the proxy on {proxy}, which is not permitted"),
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
                Err(e) => {
                    // Through a proxy, no connection made is the proxy's: it was the one connected to.
                    let class = match class_of(&e) {
                        Unreachable::Connect if self.proxy_host(&parsed).is_some() => {
                            Unreachable::Proxy
                        }
                        class => class,
                    };
                    return HttpOutcome::unreachable(&url, class, chain(&e));
                }
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
                // The redirect not followed keeps its status and kept headers (its `location`): with
                // `max_redirects` 0, it is what the caller asked for.
                Err((why, detail)) => {
                    return HttpOutcome {
                        status: Some(status),
                        headers: kept_headers(&response, request),
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
        handshake::resolve(request, self.ceiling())
    }

    fn tls(&self, request: &TlsRequest) -> TlsOutcome {
        // Started before anything else: the deadline covers building the configuration too.
        let deadline = Instant::now() + handshake::time_of(request.timeout_ms, self.ceiling());
        match self.config(deadline.saturating_duration_since(Instant::now())) {
            Some(Ok(config)) => handshake::tls(
                config.clone(),
                request,
                deadline.saturating_duration_since(Instant::now()),
            ),
            Some(Err(_)) => TlsOutcome::of(OutcomeClass::Unreachable(Unreachable::Tls)),
            None => TlsOutcome::of(OutcomeClass::Unreachable(Unreachable::Timeout)),
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

/// The headers of `response` that `request` keeps, by name, in any case.
fn kept_headers(response: &Response, request: &HttpRequest) -> Vec<(String, String)> {
    response
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
        .collect()
}

/// The final answer from `url`: its kept headers and at most the request's body cap, read before `deadline`.
fn answer(
    response: Response,
    request: &HttpRequest,
    url: String,
    deadline: Instant,
) -> HttpOutcome {
    let status = response.status().as_u16();
    let headers = kept_headers(&response, request);
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

/// Why a request got no answer, the same on every OS: any rustls or verifier rejection is `tls`.
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
