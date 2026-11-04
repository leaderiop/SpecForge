//! The ports as a grant holds them (ADR 0335 D2): each wraps a port and answers a request its [`Grant`] refuses as
//! `denied`, without performing it, so a caller handed granted ports needs no check of its own. The wasm runtime binds
//! them per instance for a guest's imports.

use std::path::Path;
use std::sync::{Arc, Mutex};

use specforge_protocol_types::{
    EffectRequest, FsRequest, HttpRequest, Operation, OutcomeClass, ProcessRequest, ResolveRequest,
    TlsRequest,
};

use crate::fs::{Fs, FsOutcome};
use crate::grant::{Denial, Grant};
use crate::http::{Http, HttpOutcome, Redirects, ResolveOutcome, TlsOutcome};
use crate::process::{Ceiling, Output, Process, ProcessClass, ProcessOutcome};
use crate::{Roots, SharedFs, SharedHttp, SharedProcess};

/// A port under a grant, for calls of one kind.
pub struct GrantedPort<P: ?Sized> {
    pub grant: Arc<Grant>,
    pub op: Operation,
    pub port: Arc<P>,
}

impl<P: ?Sized> GrantedPort<P> {
    fn check(&self, request: EffectRequest) -> Result<(), Denial> {
        self.grant.permits(self.op, &request)
    }
}

impl Http for GrantedPort<dyn Http> {
    /// The request as its grant permits it, carrying the secret the permitting capability attaches and no other, and
    /// each redirect asked of the grant again: a hop it does not permit stops the request as `denied` (ADR 0335 D6).
    fn request(&self, request: &HttpRequest, redirects: &dyn Redirects) -> HttpOutcome {
        let asked = EffectRequest::Http(request.clone());
        let granted = match self.grant.permitted(self.op, &asked) {
            Ok(granted) => granted,
            Err(denial) => {
                return HttpOutcome::failed(&request.url, OutcomeClass::Denied, denial.detail);
            }
        };
        let request = HttpRequest {
            secret: granted.secret.clone(),
            ..request.clone()
        };
        let hops = Regranted {
            grant: &self.grant,
            op: self.op,
            request: &request,
            then: redirects,
            denied: Mutex::new(None),
        };
        let outcome = self.port.request(&request, &hops);
        match hops.denied.into_inner().unwrap_or_else(|e| e.into_inner()) {
            Some(detail) => HttpOutcome {
                class: OutcomeClass::Denied,
                detail: Some(detail),
                ..outcome
            },
            None => outcome,
        }
    }

    fn resolve(&self, request: &ResolveRequest) -> ResolveOutcome {
        match self.check(EffectRequest::Resolve(request.clone())) {
            Ok(()) => self.port.resolve(request),
            Err(_) => ResolveOutcome::of(OutcomeClass::Denied),
        }
    }

    fn tls(&self, request: &TlsRequest) -> TlsOutcome {
        match self.check(EffectRequest::Tls(request.clone())) {
            Ok(()) => self.port.tls(request),
            Err(_) => TlsOutcome::of(OutcomeClass::Denied),
        }
    }
}

/// A granted request's redirects: each hop is permitted by the grant as the request it will be, its URL and the
/// method it is sent with, then by the caller's policy.
struct Regranted<'a> {
    grant: &'a Grant,
    op: Operation,
    request: &'a HttpRequest,
    then: &'a dyn Redirects,
    /// Why the grant refused a hop: the port reports a refused redirect; the answer is `denied`.
    denied: Mutex<Option<String>>,
}

impl Redirects for Regranted<'_> {
    fn allow(&self, from: &str, to: &str, method: &str) -> bool {
        let hop = EffectRequest::Http(HttpRequest {
            url: to.to_string(),
            method: method.to_string(),
            ..self.request.clone()
        });
        match self.grant.permits(self.op, &hop) {
            Ok(()) => self.then.allow(from, to, method),
            Err(denial) => {
                *self.denied.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("{from} redirects to {to}: {}", denial.detail));
                false
            }
        }
    }
}

impl Fs for GrantedPort<dyn Fs> {
    fn run(&self, roots: &Roots, request: &FsRequest) -> FsOutcome {
        match self.check(EffectRequest::Fs(request.clone())) {
            Ok(()) => self.port.run(roots, request),
            Err(_) => FsOutcome::of(OutcomeClass::Denied),
        }
    }
}

impl Process for GrantedPort<dyn Process> {
    fn run_within(
        &self,
        request: &ProcessRequest,
        dir: &Path,
        output: Output<'_>,
        ceiling: Ceiling,
    ) -> ProcessOutcome {
        match self.check(EffectRequest::Process(request.clone())) {
            Ok(()) => self.port.run_within(request, dir, output, ceiling),
            Err(denial) => ProcessOutcome::of(ProcessClass::CannotStart(format!(
                "not granted: {}",
                denial.detail
            ))),
        }
    }
}

impl Grant {
    /// `http` under this grant, for calls of kind `op`.
    pub fn http(self: &Arc<Self>, op: Operation, http: SharedHttp) -> SharedHttp {
        Arc::new(GrantedPort {
            grant: Arc::clone(self),
            op,
            port: http,
        })
    }

    /// `fs` under this grant, for calls of kind `op`.
    pub fn fs(self: &Arc<Self>, op: Operation, fs: SharedFs) -> SharedFs {
        Arc::new(GrantedPort {
            grant: Arc::clone(self),
            op,
            port: fs,
        })
    }

    /// `process` under this grant, for calls of kind `op`.
    pub fn process(self: &Arc<Self>, op: Operation, process: SharedProcess) -> SharedProcess {
        Arc::new(GrantedPort {
            grant: Arc::clone(self),
            op,
            port: process,
        })
    }
}
