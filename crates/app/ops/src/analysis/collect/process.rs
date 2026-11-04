//! The collector machine on the user's machine (ADR 0125): a child process run through the effect plane's
//! Process port (ADR 0335), held to a deadline and an output cap, and the user-level consent store. It is the only
//! place collect reaches the process, the wall clock and the environment.

use crate::machine::{
    CollectorCommand, CollectorMachine, CommandApproval, REPORT_ENV, Ran, RunFailure, RunnerOutput,
};
use serde::{Deserialize, Serialize};
use specforge_effects::process::{DEADLINE_CEILING, request};
use specforge_effects::{EnvVar, Output, ProcessClass, SharedProcess, Shown};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

/// Overrides the user-level consent store (tests, custom homes).
const CONSENT_FILE_ENV: &str = "SPECFORGE_CONSENT_FILE";

/// A child process for the command and a file for the approvals. The store lives outside the project so a
/// cloned repository can't pre-approve its own commands.
#[derive(Clone)]
pub struct ProcessMachine {
    store: PathBuf,
    process: SharedProcess,
    /// When a collector's command is stopped: the runner's 600 s ceiling unless a test says otherwise.
    deadline: Duration,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConsentStore {
    #[serde(default)]
    approved: Vec<CommandApproval>,
}

impl ProcessMachine {
    /// The user's machine: the consent store at `$SPECFORGE_CONSENT_FILE`, else
    /// `$HOME/.specforge/collector-consent.json` (`USERPROFILE`, then the temp dir). Read once, here.
    /// Its commands run through `process`.
    pub fn user(process: SharedProcess) -> Self {
        if let Some(path) = std::env::var_os(CONSENT_FILE_ENV) {
            return Self::with_store(PathBuf::from(path), process);
        }
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        Self::with_store(
            home.join(".specforge").join("collector-consent.json"),
            process,
        )
    }

    /// The same machine with its consent store at `store`.
    pub fn with_store(store: PathBuf, process: SharedProcess) -> Self {
        ProcessMachine {
            store,
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

    /// The store: empty when it does not exist; `Err` naming the file when it exists but cannot be read or parsed,
    /// so nothing in it counts and nothing is written over it.
    fn load(&self) -> Result<ConsentStore, String> {
        let raw = match std::fs::read_to_string(&self.store) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ConsentStore::default());
            }
            Err(e) => return Err(format!("{}: {e}", self.store.display())),
        };
        serde_json::from_str(&raw).map_err(|e| {
            format!(
                "{}: not a consent store ({e}); fix it or move it away, nothing is written over it",
                self.store.display()
            )
        })
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

    fn approved(&self, approval: &CommandApproval) -> bool {
        self.load()
            .is_ok_and(|store| store.approved.contains(approval))
    }

    /// Written privately and atomically (ADR 0193): a link at the store's name is replaced, never followed.
    fn approve(&self, approval: &CommandApproval) -> Result<(), String> {
        let mut consents = self.load()?;
        consents.approved.retain(|c| !approval.replaces(c));
        consents.approved.push(approval.clone());
        let json =
            serde_json::to_string_pretty(&consents).expect("consent serialization cannot fail");
        specforge_common::fresh::replace_private(&self.store, json.as_bytes())
            .map_err(|e| format!("{}: {e}", self.store.display()))
    }
}

#[cfg(test)]
mod tests;
