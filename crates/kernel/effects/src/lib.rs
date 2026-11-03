//! The effect plane (ADR 0335): every effect the host performs, for an extension, a check, a CLI verb or itself,
//! goes through one of the ports the caller passes, [`Process`], [`Http`] and [`Fs`]. Each is an object-safe
//! trait shared as an `Arc`; the adapters are pure-Rust crates under `crates/adapters`, the fakes are [`testing`]'s.
//! [`digest`] gives each request and outcome its canonical digest, and [`contained`] decides where a written path
//! may lead.

pub mod contained;
mod digest;
pub mod fs;
pub mod http;
pub mod process;
#[cfg(feature = "testing")]
pub mod testing;

pub use contained::{Contained, Lookup, RealFs, Refused, Resolved, RootName, Roots};
pub use digest::{canonical, digest};
pub use fs::{Fs, FsOutcome, SharedFs};
pub use http::{Http, HttpOutcome, Redirects, SharedHttp};
pub use process::{Output, Process, ProcessClass, ProcessOutcome, SharedProcess, Shown};
// `EffectRecord` and `EffectRequest` have no caller yet: plan 03's effect journal writes and serves them.
pub use specforge_protocol_types::{
    EffectRecord, EffectRequest, EnvVar, FsOp, FsRequest, Header, HttpRequest, Outcome,
    OutcomeClass, ProcessRequest, Redirected, Unreachable,
};

/// The ports a surface builds once, at its composition root, and passes down.
#[derive(Clone)]
pub struct Ports {
    pub process: SharedProcess,
    pub http: SharedHttp,
    pub fs: SharedFs,
}
