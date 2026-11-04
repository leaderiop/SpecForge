//! The collector machine on the user's machine (ADR 0125): a child process run through the effect plane's
//! Process port (ADR 0335), held to a deadline and an output cap. It is the only place collect reaches the process
//! and the wall clock; the approvals are the `Consent` port's (`crate::consent::UserConsent`).

use crate::machine::{
    CollectorCommand, CollectorMachine, REPORT_ENV, Ran, RunFailure, RunnerOutput,
};
use specforge_effects::process::{DEADLINE_CEILING, request};
use specforge_effects::{EnvVar, Output, ProcessClass, SharedProcess, Shown};
use std::io::Write;
use std::time::Duration;

/// A child process for each collector command.
#[derive(Clone)]
pub struct ProcessMachine {
    process: SharedProcess,
    /// When a collector's command is stopped: the runner's 600 s ceiling unless a test says otherwise.
    deadline: Duration,
}

impl ProcessMachine {
    /// The machine whose commands run through `process`.
    pub fn new(process: SharedProcess) -> Self {
        ProcessMachine {
            process,
            deadline: DEADLINE_CEILING,
        }
    }

    /// The same machine, its commands stopped past `deadline` (held to the ceiling): a test's short deadline.
    pub fn with_deadline(self, deadline: Duration) -> Self {
        ProcessMachine {
            deadline: deadline.min(DEADLINE_CEILING),
            ..self
        }
    }
}

impl CollectorMachine for ProcessMachine {
    fn run(
        &self,
        command: &CollectorCommand<'_>,
        capture: Option<&mut dyn Write>,
    ) -> Result<Ran, RunFailure> {
        if command.argv.is_empty() {
            return Err(RunFailure::NotRun(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "no command",
            )));
        }
        let mut asked = request(command.argv.iter().map(String::as_str), self.deadline);
        asked.env = vec![EnvVar::set(
            REPORT_ENV,
            command.report.to_string_lossy().into_owned(),
        )];
        let mut sink = std::io::sink();
        let shown = match command.output {
            RunnerOutput::Inherit => Shown::Terminal,
            RunnerOutput::Stderr => Shown::Stderr,
            RunnerOutput::Discard => Shown::Discard,
        };
        let output = match (capture, command.output) {
            (None, RunnerOutput::Inherit) => Output::Inherit,
            (None, _) => Output::Tee {
                capture: &mut sink,
                shown,
            },
            (Some(capture), _) => Output::Tee { capture, shown },
        };
        let ran = self.process.run(&asked, command.dir, output);
        match ran.class {
            ProcessClass::Exited(code) => Ok(Ran {
                exit_code: Some(code),
                started: ran.started,
            }),
            ProcessClass::Signalled => Ok(Ran {
                exit_code: None,
                started: ran.started,
            }),
            ProcessClass::NotFound => Err(RunFailure::NotRun(std::io::ErrorKind::NotFound.into())),
            ProcessClass::Uncaptured(why) => Err(RunFailure::Capture(std::io::Error::other(why))),
            class @ ProcessClass::DeadlineExceeded => Err(RunFailure::Stopped {
                why: format!("{class} ({} s)", self.deadline.as_secs()),
            }),
            class @ ProcessClass::OutputCapped => Err(RunFailure::Stopped {
                why: format!(
                    "{class} ({} MiB)",
                    specforge_effects::process::OUTPUT_CAP / (1024 * 1024)
                ),
            }),
            class => Err(RunFailure::NotRun(std::io::Error::other(class.to_string()))),
        }
    }
}

#[cfg(test)]
mod tests;
