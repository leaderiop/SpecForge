//! The effect plane (ADR 0335): every effect the host performs, for an extension, a check, a CLI verb or itself,
//! goes through one of the four ports the caller passes, [`Process`], [`Http`], [`Fs`] and [`Git`]. Each is an
//! object-safe trait shared as an `Arc`; the adapters are pure-Rust crates under `crates/adapters`, the fakes are [`testing`]'s.
//! [`digest`] gives each request and outcome its canonical digest, and [`contained`] decides where a written path
//! may lead.
//!
//! It also holds the policy that decides each effect: [`grant`] (declared, gated, under the [`ceiling`] and approved),
//! [`consent`] (the machine's approvals) and [`scope`] (what each kind of scope means).

pub mod ceiling;
pub mod consent;
pub mod contained;
mod digest;
pub mod fs;
pub mod gate;
pub mod git;
pub mod grant;
mod granted;
pub mod http;
pub mod journal;
pub mod mode;
pub mod process;
mod provenance;
pub mod scope;
#[cfg(feature = "testing")]
pub mod testing;
mod time;

pub use ceiling::{EffectsCeiling, FsCeiling, GitCeiling, HttpCeiling, Narrow, Narrowed, Removed};
pub use consent::{
    Approval, Approvals, ApprovedAt, CapabilityApproval, Consent, ConsentError, KeyedBy,
    ProjectKey, Revoke, RootApproval, SharedConsent,
};
pub use contained::{Contained, Lookup, RealFs, Refused, Resolved, RootName, Roots};
pub use digest::{canonical, digest};
pub use fs::{Fs, FsOutcome, SharedFs};
pub use gate::{Port, gate};
pub use git::{Git, SharedGit};
pub use grant::{Denial, Denied, Grant, Granted, Need, OutsideRoot, Uses};
pub use http::{Http, HttpOutcome, Redirects, SharedHttp};
pub use journal::{Answer, EffectJournal, EffectRecord, RecordDigest};
pub use mode::{Effects, EffectsMode, Served};
pub use process::{Ceiling, Output, Process, ProcessClass, ProcessOutcome, SharedProcess, Shown};
pub use provenance::Provenance;
pub use specforge_protocol_types::{
    AttachedSecret, EffectRequest, EnvVar, FsOp, FsRequest, Header, HttpRequest, Outcome,
    OutcomeClass, ProcessRequest, Redirected, Unreachable,
};
pub use time::{ClockError, UnixMillis};

/// The ports a surface builds once, at its composition root, and passes down.
#[derive(Clone)]
pub struct Ports {
    pub process: SharedProcess,
    pub http: SharedHttp,
    pub fs: SharedFs,
    pub git: SharedGit,
}
