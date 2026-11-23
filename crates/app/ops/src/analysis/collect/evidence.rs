//! Where one collector's evidence comes from, by collect mode (ADR 0125 D7): what the mode checks before
//! anything runs, what it gathers, and how it says it found none. A run and a no-run read the collector's
//! declared report and captured output, so only they resolve and check them; `--report` reads the files it
//! names and nothing the collector declares.

use super::collector::{
    ConfigFile, capture_path, captures_stdout, command_line, config_file, read_capture,
    reads_no_report, report_glob, report_path,
};
use super::dispatch::{read_glob, read_report};
use super::{Collector, Mode, fail};
use crate::machine::{CollectorCommand, CollectorMachine, Ran, RunFailure, RunnerOutput};
use crate::{Containment, Held, OpError, OpErrorKind};
use specforge_common::codes;
use specforge_project::file_reads::FileSource;
use specforge_protocol_types::CollectReportFile;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A collector's declared report location and captured output, held.
pub(super) struct Declared {
    /// The report, or a glob's literal base directory.
    report: Held,
    /// The report glob, when the report is one (ADR 0389).
    glob: Option<String>,
    capture: Option<Held>,
}

impl Declared {
    /// `collector`'s declared report and capture, held by `containment`: E058 for a report name that is not
    /// relative-without-`..`, an unknown capture, or a glob a single path is asked of; `outside_root` for one that
    /// lands outside.
    fn of(collector: &Collector, root: &Path, containment: &Containment) -> Result<Self, OpError> {
        let report = report_path(collector, root, containment)?;
        let capturing = captures_stdout(collector)
            .map_err(|m| fail(OpErrorKind::PreconditionFailed, codes::E058, m))?;
        let capture = if capturing {
            Some(capture_path(collector, &report, containment)?)
        } else {
            None
        };
        Ok(Declared {
            report,
            glob: report_glob(collector).map(str::to_string),
            capture,
        })
    }

    /// The report files: every match of the glob, else the file or the directory's `*.json` files; only those modified
    /// since `since` when given. Beside them, the glob's matches that land outside the project, not read.
    fn read(
        &self,
        root: &Path,
        since: Option<SystemTime>,
        containment: &Containment,
        project: &FileSource<'_>,
    ) -> Result<(Vec<CollectReportFile>, Vec<String>), OpError> {
        match &self.glob {
            Some(glob) => read_glob(glob, project, root, since, containment),
            None => Ok((
                read_report(&self.report, root, since, containment)?,
                Vec::new(),
            )),
        }
    }

    /// Where the report is, as a message names it: the glob, else the declared location.
    fn shown(&self) -> String {
        self.glob
            .clone()
            .unwrap_or_else(|| self.report.named.display().to_string())
    }

    /// The captured output, if the collector captures one and it is there.
    fn stdout(&self) -> Option<String> {
        self.capture.as_ref().and_then(read_capture)
    }
}

/// Where one collector's evidence comes from: decided for every selected collector before any command runs.
pub(super) enum Source<'m> {
    /// Write the config file, run the approved `argv`, then read the declared report written since it started, and
    /// the capture.
    Run {
        output: RunnerOutput,
        declared: Declared,
        /// The config file the command reads (ADR 0348 D2).
        config: Option<ConfigFile>,
        argv: Vec<String>,
    },
    /// Read the declared report and the capture the runner already wrote.
    Written(Declared),
    /// Read these files, and nothing the collector declares.
    Given(&'m [PathBuf]),
    /// Read nothing: the collector declares `check_outcomes`, and the served rows are its evidence.
    Outcomes,
    /// Read nothing: the collector reads through its ports when it is called (ADR 0345 D2).
    Ports,
}

/// What one collector's source gave.
pub(super) struct Evidence {
    pub(super) files: Vec<CollectReportFile>,
    pub(super) stdout: Option<String>,
    pub(super) exit_code: Option<i32>,
    /// Whether the collector's command ran.
    pub(super) ran: bool,
    /// The report glob's matches that land outside the project (a link out of it), not read (ADR 0389 D6).
    pub(super) unread: Vec<String>,
}

impl<'m> Source<'m> {
    /// `collector`'s source under `mode`, decided without running anything or writing a file. Under a run
    /// that it has a command; whether the command may run is settled next, for every collector at once
    /// (`approval::settle_runs`).
    pub(super) fn plan(
        collector: &Collector,
        mode: Mode<'m>,
        filter: Option<&str>,
        root: &Path,
        containment: &Containment,
    ) -> Result<Self, OpError> {
        match mode {
            Mode::Reports(_) if collector.reads_ports() => Err(reads_no_report(collector)),
            Mode::Reports(paths) => Ok(Source::Given(paths)),
            _ if collector.reads_outcomes() => Ok(Source::Outcomes),
            _ if collector.reads_ports() => Ok(Source::Ports),
            Mode::NoRun => Declared::of(collector, root, containment).map(Source::Written),
            Mode::Run(output) => {
                let declared = Declared::of(collector, root, containment)?;
                let config = config_file(collector, root, containment, &declared.report.at)?;
                let argv = command_line(
                    collector,
                    &declared.report.at,
                    config.as_ref().map(|config| config.held.at.as_path()),
                    filter,
                );
                if argv.is_empty() {
                    return Err(fail(
                        OpErrorKind::SchemaMismatch,
                        codes::E045,
                        format!(
                            "collector '{}' of {} declares no command, so it cannot run the tests; collect from the \
                             report they already wrote",
                            collector.name, collector.extension
                        ),
                    ));
                }
                Ok(Source::Run {
                    output,
                    declared,
                    config,
                    argv,
                })
            }
        }
    }

    /// Gather the evidence: under `Run`, announce then run (E045 when it cannot start or its capture cannot be
    /// written), then read; a report glob is listed over `project`. No report file and no captured output is E045,
    /// worded for the source.
    pub(super) fn gather(
        &self,
        collector: &Collector,
        root: &Path,
        containment: &Containment,
        project: &FileSource<'_>,
        machine: &dyn CollectorMachine,
        announce: &mut dyn FnMut(&Collector, &[String]),
    ) -> Result<Evidence, OpError> {
        let mut unread = Vec::new();
        let (files, stdout, exit_code, ran) = match self {
            Source::Run {
                output,
                declared,
                config,
                argv,
            } => {
                announce(collector, argv);
                let ran = run_command(
                    machine,
                    root,
                    (declared, !collector.told_its_report()),
                    config.as_ref(),
                    argv,
                    *output,
                )
                .map_err(|failure| {
                    let message = failure.said(collector, argv);
                    fail(OpErrorKind::SchemaMismatch, codes::E045, message)
                })?;
                let files;
                (files, unread) = declared.read(root, Some(ran.started), containment, project)?;
                (files, declared.stdout(), ran.exit_code, true)
            }
            Source::Written(declared) => {
                let files;
                (files, unread) = declared.read(root, None, containment, project)?;
                (files, declared.stdout(), None, false)
            }
            Source::Given(paths) => {
                let files = paths
                    .iter()
                    .map(|p| read_report(&containment.resolve(p)?, root, None, containment))
                    .collect::<Result<Vec<_>, OpError>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                (files, None, None, false)
            }
            Source::Outcomes | Source::Ports => (Vec::new(), None, None, false),
        };
        // The served rows are all a reader of the checks' outcomes needs, however few; a ports reader reads its own.
        if !matches!(self, Source::Outcomes | Source::Ports)
            && files.is_empty()
            && stdout.as_deref().is_none_or(str::is_empty)
        {
            let mut message = self.nothing_found(collector);
            if !unread.is_empty() {
                message.push_str(&format!("; {}", super::warnings::unread_paths(&unread)));
            }
            return Err(fail(OpErrorKind::SchemaMismatch, codes::E045, message));
        }
        Ok(Evidence {
            files,
            stdout,
            exit_code,
            ran,
            unread,
        })
    }

    /// What "no evidence" says for this source.
    fn nothing_found(&self, collector: &Collector) -> String {
        match self {
            Source::Run { declared, .. } => format!(
                "{} produced no report at {} (did the tests build?)",
                collector.name,
                declared.shown()
            ),
            Source::Written(declared) => format!(
                "no {} report at {}; run the tests to write one (`specforge collect` runs them)",
                collector.name,
                declared.shown()
            ),
            Source::Given(_) => "the report files given don't exist".to_string(),
            Source::Outcomes => format!("{} read no outcome", collector.name),
            Source::Ports => format!("{} read nothing through its ports", collector.name),
        }
    }
}

/// Why a collector's command did not run to its end.
enum NotRan {
    /// Its program is not on PATH.
    NotFound,
    /// Anything else, said.
    Said(String),
}

impl NotRan {
    /// What E045 (or the skip reason) says: a missing program names the collector's install line (ADR 0348 D4).
    fn said(self, collector: &Collector, argv: &[String]) -> String {
        match self {
            NotRan::Said(message) => message,
            NotRan::NotFound => {
                let program = argv.first().map_or("", String::as_str);
                match &collector.install {
                    Some(line) => {
                        format!("`{program}` was not found on PATH; install it: `{line}`")
                    }
                    None => format!("`{program}` was not found on PATH"),
                }
            }
        }
    }
}

impl From<String> for NotRan {
    fn from(message: String) -> Self {
        NotRan::Said(message)
    }
}

/// Run the collector's declared command in the project root through `machine`. A report *file* left by
/// an earlier run is removed first; a report *directory* is left alone (other tools may keep files
/// there) and filtered by modification time when it's read. The config file the command reads is written
/// fresh before it starts (ADR 0348 D2). A captured standard output still reaches the chosen output as it's
/// produced, and is also written to `capture`. Every file it removes, creates or writes is a held one, and the
/// report, the capture and the config are names the run owns: a link at one is replaced, never followed. A report
/// glob's files are the tool's: nothing is removed or created for them, and the command is told its base directory,
/// never in `SPECFORGE_REPORT` (ADR 0389).
fn run_command(
    machine: &dyn CollectorMachine,
    root: &Path,
    (declared, told_nothing): (&Declared, bool),
    config: Option<&ConfigFile>,
    argv: &[String],
    output: RunnerOutput,
) -> Result<Ran, NotRan> {
    let (report, capture) = (&declared.report, declared.capture.as_ref());
    let owned = declared.glob.is_none();
    // The report's name is the run's (ADR 0171): a file or a link at it is unlinked, never what a link names.
    if owned && !report.real.is_dir() {
        match report.remove() {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("{}: {e}", report.named.display()).into());
            }
            _ => {}
        }
    }
    for file in owned
        .then_some(report)
        .into_iter()
        .chain(capture)
        .chain(config.map(|config| &config.held))
    {
        file.create_parent()
            .map_err(|e| format!("{}: {e}", file.named.display()))?;
    }
    if let Some(ConfigFile { held, text }) = config {
        held.create_owned_file()
            .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()))
            .map_err(|e| format!("{}: {e}", held.named.display()))?;
    }
    let mut kept = match capture {
        Some(held) => Some(
            held.create_owned_file()
                .map_err(|e| format!("{}: {e}", held.named.display()))?,
        ),
        None => None,
    };
    let command = CollectorCommand {
        argv,
        dir: root,
        report: &report.at,
        report_in_env: owned && told_nothing,
        output,
    };
    machine
        .run(
            &command,
            kept.as_mut().map(|f| f as &mut dyn std::io::Write),
        )
        .map_err(|failure| match (failure, capture) {
            (RunFailure::NotFound, _) => NotRan::NotFound,
            (RunFailure::Capture(e), Some(held)) => {
                NotRan::Said(format!("{}: {e}", held.named.display()))
            }
            (RunFailure::Capture(e) | RunFailure::NotRun(e), _) => {
                NotRan::Said(format!("failed to run `{}`: {e}", argv.join(" ")))
            }
            (RunFailure::Stopped { why }, _) => NotRan::Said(format!("`{}` {why}", argv.join(" "))),
        })
}
