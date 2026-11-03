//! The `Process` port: run a program with a deadline, an environment, stdin and an output cap (ADR 0335 D1). Its
//! adapter is `specforge-process`, the one runner every spawn goes through.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use specforge_protocol_types::{OutcomeClass, ProcessRequest};

/// No run a request or an extension asks for outlives this, whatever its request asks.
pub const DEADLINE_CEILING: Duration = Duration::from_secs(600);
/// The ceiling of the host's own `extension build`: a cold release build of a wasm component can pass 600 s.
pub const BUILD_CEILING: Duration = Duration::from_secs(30 * 60);
/// The most bytes kept of each of stdout and stderr.
pub const OUTPUT_CAP: u64 = 64 * 1024 * 1024;

/// Runs one program. Every adapter holds the run to the request's deadline (at most its [`Ceiling`]) and its output
/// cap, ends the program's whole process group when either passes, and never panics: a failure is a class.
pub trait Process: Send + Sync {
    /// `request`, held to [`Ceiling::Request`]: what any process a request or an extension asks for is held to.
    fn run(&self, request: &ProcessRequest, dir: &Path, output: Output<'_>) -> ProcessOutcome {
        self.run_within(request, dir, output, Ceiling::Request)
    }

    /// `request`, held to `ceiling`. Only the host's own work names a ceiling past the request's.
    fn run_within(
        &self,
        request: &ProcessRequest,
        dir: &Path,
        output: Output<'_>,
        ceiling: Ceiling,
    ) -> ProcessOutcome;
}

/// The most a run's deadline may be: a host-owned choice, never part of the wire request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ceiling {
    /// [`DEADLINE_CEILING`]: any process a request or an extension asks for.
    Request,
    /// [`BUILD_CEILING`]: the host's own `extension build`.
    HostBuild,
}

impl Ceiling {
    pub fn limit(self) -> Duration {
        match self {
            Ceiling::Request => DEADLINE_CEILING,
            Ceiling::HostBuild => BUILD_CEILING,
        }
    }
}

/// A process port as the composition root shares it.
pub type SharedProcess = Arc<dyn Process>;

/// Where the program's output goes: a host-side choice, not part of the wire request.
pub enum Output<'a> {
    /// stdout and stderr are kept and returned.
    Buffer,
    /// stdout is written to `capture` as it arrives and shown where `shown` says; stderr goes to the terminal unless
    /// `shown` discards it. Nothing is returned.
    Tee {
        capture: &'a mut dyn Write,
        shown: Shown,
    },
    /// Both streams go to the terminal.
    Inherit,
}

/// Where a teed stdout is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    Terminal,
    Stderr,
    Discard,
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessClass {
    Exited(i32),
    /// Ended by a signal it did not ask for.
    Signalled,
    /// Stopped at its deadline.
    DeadlineExceeded,
    /// Stopped when its output passed the cap.
    OutputCapped,
    /// No such program.
    NotFound,
    /// The directory it was to run in does not exist.
    NoDirectory,
    /// Found and not started (permissions, an empty argv, …), with why.
    CannotStart(String),
    /// Its output could not be written to the caller's capture: the run was stopped, the capture is short.
    Uncaptured(String),
    /// Started, and its end could not be observed (waiting failed, or the capture could not be written), with why.
    Lost(String),
}

impl ProcessClass {
    /// The class an outcome digest and a verdict compare.
    pub fn outcome(&self) -> OutcomeClass {
        match self {
            ProcessClass::Exited(0) => OutcomeClass::Ok,
            ProcessClass::Exited(code) => OutcomeClass::Exit(*code),
            ProcessClass::Signalled => OutcomeClass::Signal,
            ProcessClass::DeadlineExceeded => OutcomeClass::Deadline,
            ProcessClass::OutputCapped => OutcomeClass::Capped,
            ProcessClass::NotFound => OutcomeClass::NotFound,
            ProcessClass::NoDirectory => OutcomeClass::Missing,
            ProcessClass::CannotStart(_) | ProcessClass::Lost(_) | ProcessClass::Uncaptured(_) => {
                OutcomeClass::CannotStart
            }
        }
    }
}

/// How a run ended, in words that follow the program's name ("git was not found", "z3 did not finish before its
/// deadline"): the one message every caller shows.
impl std::fmt::Display for ProcessClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessClass::Exited(code) => write!(f, "exited with status {code}"),
            ProcessClass::Signalled => f.write_str("was ended by a signal"),
            ProcessClass::DeadlineExceeded => f.write_str("did not finish before its deadline"),
            ProcessClass::OutputCapped => f.write_str("was stopped when its output passed the cap"),
            ProcessClass::NotFound => f.write_str("was not found"),
            ProcessClass::NoDirectory => {
                f.write_str("cannot run: the directory it runs in does not exist")
            }
            ProcessClass::CannotStart(why) => write!(f, "cannot start: {why}"),
            ProcessClass::Uncaptured(why) => {
                write!(f, "had output that could not be captured: {why}")
            }
            ProcessClass::Lost(why) => write!(f, "ended without an answer: {why}"),
        }
    }
}

/// What a run gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutcome {
    pub class: ProcessClass,
    /// Empty unless the output was [`Output::Buffer`].
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// When the run started, less a grace for the coarse clocks file times come from: a file the program wrote is
    /// never older than this (ADR 0125 D3).
    pub started: SystemTime,
}

impl ProcessOutcome {
    /// An outcome of `class` with no output, started now.
    pub fn of(class: ProcessClass) -> Self {
        ProcessOutcome {
            class,
            stdout: Vec::new(),
            stderr: Vec::new(),
            started: started_now(),
        }
    }

    /// The program exited 0.
    pub fn success(&self) -> bool {
        self.class == ProcessClass::Exited(0)
    }

    /// stdout as [`text`].
    pub fn text(&self) -> String {
        text(&self.stdout)
    }

    /// stderr as [`text`].
    pub fn stderr_text(&self) -> String {
        text(&self.stderr)
    }
}

/// `bytes` as UTF-8, an invalid sequence read as U+FFFD, and `\r\n` read as `\n`.
pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

/// Now, less 100 ms: file times come from the kernel's coarse clock, which can lag `now` by a few ticks, so a file
/// written at once would look older than the start.
pub fn started_now() -> SystemTime {
    SystemTime::now()
        .checked_sub(Duration::from_millis(100))
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// A request for `argv` with `deadline` (held to the ceiling) and the full output cap, no stdin, no env change.
pub fn request<S: Into<String>>(
    argv: impl IntoIterator<Item = S>,
    deadline: Duration,
) -> ProcessRequest {
    ProcessRequest {
        argv: argv.into_iter().map(Into::into).collect(),
        deadline_ms: deadline.min(DEADLINE_CEILING).as_millis() as u64,
        output_cap: OUTPUT_CAP,
        ..ProcessRequest::default()
    }
}
