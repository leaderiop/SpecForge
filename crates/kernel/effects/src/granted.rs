//! The ports as a grant holds them (ADR 0335 D2): each wraps a port and answers a request its [`Grant`] refuses as
//! `denied`, without performing it, so a caller handed granted ports needs no check of its own. Plan 04 binds them
//! per instance for a guest's imports.

use std::path::Path;
use std::sync::Arc;

use specforge_protocol_types::{
    EffectRequest, FsRequest, HttpRequest, Operation, OutcomeClass, ProcessRequest,
};

use crate::fs::{Fs, FsOutcome};
use crate::grant::{Denial, Grant};
use crate::http::{Http, HttpOutcome, Redirects};
use crate::process::{Ceiling, Output, Process, ProcessClass, ProcessOutcome};
use crate::{Roots, SharedFs, SharedHttp, SharedProcess};

/// A port under a grant, for calls of one kind.
pub struct Granted<P: ?Sized> {
    pub grant: Arc<Grant>,
    pub op: Operation,
    pub port: Arc<P>,
}

impl<P: ?Sized> Granted<P> {
    fn check(&self, request: EffectRequest) -> Result<(), Denial> {
        self.grant.permits(self.op, &request)
    }
}

impl Http for Granted<dyn Http> {
    /// The request as its grant permits it, carrying the secret the permitting capability attaches and no other
    /// (ADR 0335 D6).
    fn request(&self, request: &HttpRequest, redirects: &dyn Redirects) -> HttpOutcome {
        let asked = EffectRequest::Http(request.clone());
        match self.grant.permitted(self.op, &asked) {
            Ok(granted) => {
                let request = HttpRequest {
                    secret: granted.secret.clone(),
                    ..request.clone()
                };
                self.port.request(&request, redirects)
            }
            Err(denial) => HttpOutcome::failed(&request.url, OutcomeClass::Denied, denial.detail),
        }
    }
}

impl Fs for Granted<dyn Fs> {
    fn run(&self, roots: &Roots, request: &FsRequest) -> FsOutcome {
        match self.check(EffectRequest::Fs(request.clone())) {
            Ok(()) => self.port.run(roots, request),
            Err(_) => FsOutcome::of(OutcomeClass::Denied),
        }
    }
}

impl Process for Granted<dyn Process> {
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
        Arc::new(Granted {
            grant: Arc::clone(self),
            op,
            port: http,
        })
    }

    /// `fs` under this grant, for calls of kind `op`.
    pub fn fs(self: &Arc<Self>, op: Operation, fs: SharedFs) -> SharedFs {
        Arc::new(Granted {
            grant: Arc::clone(self),
            op,
            port: fs,
        })
    }

    /// `process` under this grant, for calls of kind `op`.
    pub fn process(self: &Arc<Self>, op: Operation, process: SharedProcess) -> SharedProcess {
        Arc::new(Granted {
            grant: Arc::clone(self),
            op,
            port: process,
        })
    }
}
