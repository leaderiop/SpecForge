//! `specforge run` (ADR 0339): the one runner for people and agents, over the `Process` port under the user grant. A
//! run passes its program's stdin, stdout, stderr and exit through; `--format json` captures it instead, through the
//! effects journal in the mode asked. A deadline is 124, a program not found 127 and one that cannot start 126.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specforge_effects::journal::{Answer, Blob};
use specforge_effects::process::{Ceiling, Output, USER_CEILING};
use specforge_effects::{
    EffectJournal, EffectRequest, Effects, EffectsMode, Outcome, ProcessClass, ProcessRequest,
    Served, UnixMillis,
};

use super::Verbs;
use crate::{OpError, OpErrorKind};

/// The most bytes a run captured under `--format json` may write to each of stdout and stderr: past it, it is stopped.
pub const CAPTURE_CAP: u64 = 4 * 1024 * 1024;
/// The most bytes of each stream a captured answer keeps, so its record holds it whole.
pub const ANSWER_BYTES: usize = 1024 * 1024;

/// What `specforge run` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The program, then its arguments.
    pub argv: Vec<String>,
    /// When it is stopped (exit 124); at most a day.
    pub deadline: Option<Duration>,
}

/// `run --format json`'s answer: what the program did, captured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunAnswer {
    pub argv: Vec<String>,
    /// The exit `run` passes through: the program's own, or 124, 126 or 127.
    pub exit: i32,
    /// `exited`, `signalled`, `deadline`, `output_capped`, `not_found` or `cannot_start`.
    pub outcome: String,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    /// The deadline the run was held to: `--deadline`, or the 24 h a run without one gets.
    pub deadline_ms: u64,
    /// A stream passed what the answer keeps, or the run was stopped at its cap.
    pub truncated: bool,
}

/// `run --which`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhichAnswer {
    pub programs: Vec<Program>,
}

/// Where one program was found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Program {
    pub name: String,
    pub found: bool,
    /// None: the runner finds no such program.
    pub path: Option<String>,
}

/// The exit `run` passes through for a run that ended as `class`: the program's own, 124 at its deadline, 127 when not
/// found, 126 when it cannot start, else 1.
pub fn exit_of(class: &ProcessClass) -> i32 {
    match class {
        ProcessClass::Exited(code) => *code,
        ProcessClass::DeadlineExceeded => 124,
        ProcessClass::NotFound => 127,
        ProcessClass::CannotStart(_) | ProcessClass::NoDirectory => 126,
        ProcessClass::Signalled
        | ProcessClass::OutputCapped
        | ProcessClass::Uncaptured(_)
        | ProcessClass::Lost(_) => 1,
    }
}

fn outcome_word(class: &ProcessClass) -> &'static str {
    match class {
        ProcessClass::Exited(_) => "exited",
        ProcessClass::Signalled => "signalled",
        ProcessClass::DeadlineExceeded => "deadline",
        ProcessClass::OutputCapped => "output_capped",
        ProcessClass::NotFound => "not_found",
        ProcessClass::CannotStart(_) | ProcessClass::NoDirectory => "cannot_start",
        ProcessClass::Uncaptured(_) | ProcessClass::Lost(_) => "lost",
    }
}

/// The request `run` asks, permitted by the user grant.
fn request(verbs: &Verbs, run: &Run, output_cap: u64) -> Result<ProcessRequest, OpError> {
    if run.argv.is_empty() {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            "name the program to run",
        ));
    }
    let deadline = run.deadline.unwrap_or(USER_CEILING).min(USER_CEILING);
    let request = ProcessRequest {
        argv: run.argv.clone(),
        deadline_ms: u64::try_from(deadline.as_millis()).unwrap_or(u64::MAX),
        output_cap,
        ..ProcessRequest::default()
    };
    verbs.permit(&EffectRequest::Process(request.clone()))?;
    Ok(request)
}

/// `run` passing the program's stdin, stdout and stderr through, in the current directory: how it ended. Never
/// journaled: what it did is the terminal's, and nothing could answer it again.
pub fn pass_through(verbs: &Verbs, run: &Run) -> Result<ProcessClass, OpError> {
    let request = request(verbs, run, u64::MAX)?;
    let outcome =
        verbs
            .ports()
            .process
            .run_within(&request, verbs.cwd(), Output::Inherit, Ceiling::User);
    Ok(outcome.class)
}

/// `run --format json`: the program run with a null stdin and its output captured, through the effects journal in
/// `mode` (its record is the user's; `cached` performs it, since a program asks for something to happen, and `replay`
/// answers from the journal alone).
pub fn capture(verbs: &Verbs, run: &Run, mode: EffectsMode) -> Result<RunAnswer, OpError> {
    let mut request = request(verbs, run, CAPTURE_CAP)?;
    // A captured run reads nothing from the terminal.
    request.stdin = Some(Vec::new());
    let asked = EffectRequest::Process(request.clone());
    let effects = Effects::of(
        mode,
        verbs.ports().clone(),
        Vec::new(),
        EffectJournal::for_root(verbs.root()),
    );
    let now = UnixMillis::now_or_end();
    let served = effects.serve_user(verbs.ports(), &asked, now, |ports| {
        let started = Instant::now();
        let outcome =
            ports
                .process
                .run_within(&request, verbs.cwd(), Output::Buffer, Ceiling::User);
        let mut answer = answered(
            &run.argv,
            &outcome.class,
            &outcome.stdout,
            &outcome.stderr,
            started,
        );
        answer.deadline_ms = request.deadline_ms;
        let payload = serde_json::to_vec(&answer).unwrap_or_default();
        Answer {
            outcome: Outcome {
                class: outcome.class.outcome(),
                digest: Blob::of(&outcome.stdout).sha256,
            },
            payload: Some(payload),
        }
    });
    match served {
        Served::Outcome { answer, .. } => answer
            .payload
            .as_deref()
            .and_then(|payload| serde_json::from_slice(payload).ok())
            .ok_or_else(|| {
                OpError::new(
                    OpErrorKind::Internal,
                    OpErrorKind::Internal.as_str(),
                    "the journal's record of this run holds no answer",
                )
            }),
        Served::Miss => Err(OpError::new(
            OpErrorKind::PreconditionFailed,
            crate::NOT_IN_JOURNAL,
            format!(
                "`{}` was never run with its output captured: replay answers from the effects journal alone",
                run.argv.join(" ")
            ),
        )),
        Served::NotPerformed(_) | Served::Local => Err(OpError::new(
            OpErrorKind::Internal,
            OpErrorKind::Internal.as_str(),
            "the run was not performed",
        )),
    }
}

/// The answer of a run that ended as `class`, each stream cut to [`ANSWER_BYTES`].
fn answered(
    argv: &[String],
    class: &ProcessClass,
    stdout: &[u8],
    stderr: &[u8],
    started: Instant,
) -> RunAnswer {
    let kept = |bytes: &[u8]| {
        let cut = &bytes[..bytes.len().min(ANSWER_BYTES)];
        specforge_effects::process::text(cut)
    };
    RunAnswer {
        argv: argv.to_vec(),
        exit: exit_of(class),
        outcome: outcome_word(class).to_string(),
        stdout: kept(stdout),
        stderr: kept(stderr),
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        deadline_ms: 0,
        truncated: *class == ProcessClass::OutputCapped
            || stdout.len() > ANSWER_BYTES
            || stderr.len() > ANSWER_BYTES,
    }
}

/// Where the runner finds each of `programs`, running none.
pub fn which(verbs: &Verbs, programs: &[String]) -> WhichAnswer {
    WhichAnswer {
        programs: programs
            .iter()
            .map(|name| {
                let path = verbs.ports().process.which(name);
                Program {
                    name: name.clone(),
                    found: path.is_some(),
                    path: path.map(|path| path.to_string_lossy().into_owned()),
                }
            })
            .collect(),
    }
}
