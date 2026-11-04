//! What collect asks the machine it runs on (ADR 0125): run a collector's command. Whether it may run is the
//! grant's and the `Consent` port's (ADR 0335 D2). Two adapters:
//! [`ProcessMachine`](crate::analysis::collect::ProcessMachine) (a child process) and
//! `specforge_ops::testing::ScriptedMachine` (tests). A base module, as the solver's
//! port is: the adapters and the test support name it without reaching into the analysis group.

use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

/// Environment variable the machine sets to the report path when it runs a collector's command, so a
/// runner integration can write there directly.
pub const REPORT_ENV: &str = "SPECFORGE_REPORT";

/// Where the runner's own output goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerOutput {
    /// The user's terminal.
    Inherit,
    /// Stderr, keeping stdout clean for machine-readable output.
    Stderr,
    /// Discarded (the MCP server owns stdio).
    Discard,
}

/// One collector command to run to its end.
#[derive(Debug, Clone, Copy)]
pub struct CollectorCommand<'a> {
    /// The command line, `{report}` expanded. Never empty: collect refuses an empty command first.
    pub argv: &'a [String],
    /// The directory it runs in: the project root.
    pub dir: &'a Path,
    /// What `SPECFORGE_REPORT` is set to: the report location as the project names it.
    pub report: &'a Path,
    /// Where its standard output and error go.
    pub output: RunnerOutput,
}

/// What running a command produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ran {
    /// Its exit code; `None` when a signal ended it. Non-zero is normal when tests fail.
    pub exit_code: Option<i32>,
    /// No later than the modification time of any file the command wrote: collect reads a report
    /// directory's files modified at or after it, so an old report can never pass for a new run.
    pub started: SystemTime,
}

/// Why a command produced no [`Ran`].
#[derive(Debug)]
pub enum RunFailure {
    /// It could not be started, or not waited for.
    NotRun(std::io::Error),
    /// Its standard output could not be written to the capture.
    Capture(std::io::Error),
    /// It was stopped, with its whole process group: past the collector deadline or its output cap (ADR 0335).
    Stopped { why: String },
}

/// The machine collect runs on.
pub trait CollectorMachine: Send + Sync {
    /// Run `command` to its end. Its standard output goes where `command.output` says and, when `capture`
    /// is given, is also written to it as it arrives. A non-zero exit is `Ok`.
    fn run(
        &self,
        command: &CollectorCommand<'_>,
        capture: Option<&mut dyn Write>,
    ) -> Result<Ran, RunFailure>;
}
