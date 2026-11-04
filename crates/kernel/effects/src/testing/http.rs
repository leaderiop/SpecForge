//! The fake of the `Http` port.

use std::collections::BTreeMap;
use std::sync::Mutex;

use specforge_protocol_types::{
    EffectRequest, HttpRequest, OutcomeClass, Redirected, ResolveRequest, TlsRequest, Unreachable,
};

use crate::http::{ResolveOutcome, TlsOutcome, scheme, status_class};
use crate::{Http, HttpOutcome, Redirects};

/// An HTTP port that answers from a route table, follows redirects as every adapter must, and records every request.
#[derive(Default)]
pub struct FakeHttp {
    routes: BTreeMap<String, Route>,
    names: BTreeMap<String, Vec<String>>,
    certificates: BTreeMap<(String, u16), TlsOutcome>,
    requests: Mutex<Vec<HttpRequest>>,
    asked: Mutex<Vec<EffectRequest>>,
}

#[derive(Clone)]
struct Route {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl FakeHttp {
    /// Every URL is unreachable (`unreachable:dns`).
    pub fn new() -> Self {
        FakeHttp::default()
    }

    /// `url` answers `status` with `body`.
    pub fn route(self, url: &str, status: u16, body: impl Into<Vec<u8>>) -> Self {
        self.route_with(url, status, Vec::new(), body)
    }

    /// `url` answers `status` with `headers` and `body`.
    pub fn route_with(
        mut self,
        url: &str,
        status: u16,
        headers: Vec<(&str, &str)>,
        body: impl Into<Vec<u8>>,
    ) -> Self {
        let headers = headers
            .into_iter()
            .map(|(n, v)| (n.to_ascii_lowercase(), v.to_string()))
            .collect();
        self.routes.insert(
            url.to_string(),
            Route {
                status,
                headers,
                body: body.into(),
            },
        );
        self
    }

    /// `from` redirects (302) to `to`.
    pub fn redirect(self, from: &str, to: &str) -> Self {
        self.route_with(from, 302, vec![("location", to)], Vec::new())
    }

    /// `host` resolves to `addresses`.
    pub fn name(mut self, host: &str, addresses: &[&str]) -> Self {
        self.names.insert(
            host.to_string(),
            addresses.iter().map(ToString::to_string).collect(),
        );
        self
    }

    /// A handshake with `host:port` reads a certificate expiring at `not_after`, issued by `issuer`; `valid` when its
    /// chain verifies.
    pub fn certificate(
        mut self,
        host: &str,
        port: u16,
        not_after: &str,
        issuer: &str,
        valid: bool,
    ) -> Self {
        self.certificates.insert(
            (host.to_string(), port),
            TlsOutcome {
                class: if valid {
                    OutcomeClass::Ok
                } else {
                    OutcomeClass::Unreachable(Unreachable::Tls)
                },
                not_after: Some(not_after.to_string()),
                issuer: Some(issuer.to_string()),
                chain_valid: valid,
            },
        );
        self
    }

    /// The requests made so far, in order (one per request, not per hop).
    pub fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().expect("no request panics").clone()
    }

    /// The names resolved and the handshakes made so far, in order.
    pub fn asked(&self) -> Vec<EffectRequest> {
        self.asked.lock().expect("no request panics").clone()
    }
}

impl Http for FakeHttp {
    fn request(&self, request: &HttpRequest, redirects: &dyn Redirects) -> HttpOutcome {
        self.requests
            .lock()
            .expect("no request panics")
            .push(request.clone());
        let mut url = request.url.clone();
        let mut hops = 0;
        loop {
            let Some(route) = self.routes.get(&url) else {
                return HttpOutcome::unreachable(&url, Unreachable::Dns, "no route");
            };
            let location = route
                .headers
                .iter()
                .find(|(n, _)| n == "location")
                .map(|(_, v)| v.clone());
            if let (300..=399, Some(to)) = (route.status, location) {
                let refused = if hops == request.max_redirects {
                    Some(Redirected::Limit)
                } else if scheme(&url) == "https" && scheme(&to) == "http" {
                    Some(Redirected::Downgrade)
                } else if !redirects.allow(&url, &to) {
                    Some(Redirected::Refused)
                } else {
                    None
                };
                if let Some(why) = refused {
                    return HttpOutcome::failed(&url, OutcomeClass::Redirect(why), to);
                }
                hops += 1;
                url = to;
                continue;
            }
            let cap = usize::try_from(request.body_cap).unwrap_or(usize::MAX);
            let capped = route.body.len() > cap;
            return HttpOutcome {
                class: if capped {
                    OutcomeClass::Capped
                } else {
                    status_class(route.status)
                },
                status: Some(route.status),
                final_url: url,
                headers: route
                    .headers
                    .iter()
                    .filter(|(n, _)| {
                        request
                            .keep_headers
                            .iter()
                            .any(|k| k.eq_ignore_ascii_case(n))
                    })
                    .cloned()
                    .collect(),
                body: route.body[..route.body.len().min(cap)].to_vec(),
                detail: None,
            };
        }
    }

    fn resolve(&self, request: &ResolveRequest) -> ResolveOutcome {
        self.asked
            .lock()
            .expect("no request panics")
            .push(EffectRequest::Resolve(request.clone()));
        let Some(addresses) = self.names.get(&request.host) else {
            return ResolveOutcome::of(OutcomeClass::Unreachable(Unreachable::Dns));
        };
        ResolveOutcome {
            class: OutcomeClass::Ok,
            addresses: addresses.clone(),
        }
    }

    fn tls(&self, request: &TlsRequest) -> TlsOutcome {
        self.asked
            .lock()
            .expect("no request panics")
            .push(EffectRequest::Tls(request.clone()));
        self.certificates
            .get(&(request.host.clone(), request.tcp_port))
            .cloned()
            .unwrap_or_else(|| TlsOutcome::of(OutcomeClass::Unreachable(Unreachable::Connect)))
    }
}

#[cfg(test)]
mod fake_http_tests {
    use super::*;

    #[test]
    fn fake_http_answers_resolve_and_tls_from_its_routes() {
        let http = FakeHttp::new()
            .name("a.example", &["192.0.2.1"])
            .certificate("a.example", 443, "2030-01-31T23:59:59Z", "Test CA", true)
            .certificate("b.example", 443, "2030-01-31T23:59:59Z", "Test CA", false);
        let resolve = |host: &str| {
            http.resolve(&ResolveRequest {
                host: host.to_string(),
                ..ResolveRequest::default()
            })
        };
        let tls = |host: &str, tcp_port| {
            http.tls(&TlsRequest {
                host: host.to_string(),
                tcp_port,
                ..TlsRequest::default()
            })
        };
        assert_eq!(resolve("a.example").addresses, ["192.0.2.1"]);
        assert_eq!(
            resolve("b.example").class,
            OutcomeClass::Unreachable(Unreachable::Dns)
        );
        assert!(tls("a.example", 443).chain_valid);
        let rejected = tls("b.example", 443);
        assert_eq!(rejected.class, OutcomeClass::Unreachable(Unreachable::Tls));
        assert_eq!(rejected.issuer.as_deref(), Some("Test CA"));
        assert_eq!(
            tls("a.example", 8443).class,
            OutcomeClass::Unreachable(Unreachable::Connect)
        );
        assert_eq!(http.asked().len(), 5);
    }
}
