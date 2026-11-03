//! The `Process` port: run a program with a deadline, an environment, stdin and an output cap (ADR 0335 D1). Its
//! adapter is `specforge-process`, the one runner every spawn goes through.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use specforge_protocol_types::{OutcomeClass, ProcessRequest};

/// No run outlives this, whatever its request asks.
pub const DEADLINE_CEILING: Duration = Duration::from_secs(600);
/// The most bytes kept of each of stdout and stderr.
pub const OUTPUT_CAP: u64 = 64 * 1024 * 1024;

/// Runs one program. Every adapter holds the run to the request's deadline (at most [`DEADLINE_CEILING`]) and its
/// output cap, ends the program's whole process group when either passes, and never panics: a failure is a class.
pub trait Process: Send + Sync {
    fn run(&self, request: &ProcessRequest, dir: &Path, output: Output<'_>) -> ProcessOutcome;
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
    /// Found and not started (permissions, an empty argv, …), with why.
    CannotStart(String),
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
            ProcessClass::CannotStart(_) | ProcessClass::Lost(_) => OutcomeClass::CannotStart,
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
