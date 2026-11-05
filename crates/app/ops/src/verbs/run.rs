//! `specforge run` (ADR 0339): the one runner for people and agents, over the `Process` port under the user grant. A
//! run passes its program's stdin, stdout, stderr and exit through; `--format json` captures it instead, through the
//! effects journal in the mode asked. A deadline is 124, a program not found 127 and one that cannot start 126.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specforge_effects::journal::{Answer, Blob, RecordDigest};
use specforge_effects::mode::USER;
use specforge_effects::process::{Ceiling, Output, USER_CEILING};
use specforge_effects::{
    EffectJournal, EffectRequest, Effects, EffectsMode, EnvVar, FsOp, Outcome, ProcessClass,
    ProcessRequest, Served, UnixMillis,
};

use super::{Answered, Effect, Verbs};
use crate::{OpError, OpErrorKind};

/// The most bytes a run captured under `--format json` may write to each of stdout and stderr (the runner's 64 MiB):
/// past it, it is stopped, and its answer says `output_capped` and `truncated`.
pub const CAPTURE_CAP: u64 = specforge_effects::process::OUTPUT_CAP;
/// The most bytes of each stream a captured answer keeps, so its record holds it whole.
pub const ANSWER_BYTES: usize = 1024 * 1024;

/// What `specforge run` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The program, then its arguments.
    pub argv: Vec<String>,
    /// When it is stopped (exit 124); at most a day.
    pub deadline: Option<Duration>,
    /// The directory it runs in, from the current directory and held to the project's roots; the current directory
    /// when none.
    pub dir: Option<PathBuf>,
}

/// `run --format json`'s answer: what the program did, captured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunAnswer {
    pub argv: Vec<String>,
    /// The exit `run` passes through: the program's own, or 124, 126 or 127.
    pub exit: i32,
    pub outcome: RunOutcome,
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

/// How a run ended, as its answer says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Exited,
    Signalled,
    Deadline,
    OutputCapped,
    NotFound,
    CannotStart,
    Lost,
}

/// How a run that ended as `class` is answered, and the exit `run` passes through for it: the program's own, 124 at
/// its deadline, 127 when not found, 126 when it cannot start, else 1.
pub fn ending(class: &ProcessClass) -> (RunOutcome, i32) {
    match class {
        ProcessClass::Exited(code) => (RunOutcome::Exited, *code),
        ProcessClass::Signalled => (RunOutcome::Signalled, 1),
        ProcessClass::DeadlineExceeded => (RunOutcome::Deadline, 124),
        ProcessClass::OutputCapped => (RunOutcome::OutputCapped, 1),
        ProcessClass::NotFound => (RunOutcome::NotFound, 127),
        ProcessClass::CannotStart(_) | ProcessClass::NoDirectory => (RunOutcome::CannotStart, 126),
        ProcessClass::Uncaptured(_) | ProcessClass::Lost(_) => (RunOutcome::Lost, 1),
    }
}

/// Where `run` runs: `--in`'s directory, held to the project's roots as a typed path is, else the current
/// directory.
fn place(verbs: &Verbs, run: &Run) -> Result<PathBuf, OpError> {
    let Some(dir) = &run.dir else {
        return Ok(verbs.cwd().to_path_buf());
    };
    let (located, _) = verbs.held(dir, FsOp::Stat)?;
    if !located.at.is_dir() {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            format!("{} is not a directory to run in", verbs.shown(&located.at)),
        ));
    }
    Ok(located.at)
}

/// The request `run` asks in `place`, permitted by the user grant. Run elsewhere than the current directory, the
/// program is told so by `PWD`, as a shell's `cd` tells it, and the request (so its record) names the directory.
fn request(
    verbs: &Verbs,
    run: &Run,
    place: &Path,
    output_cap: u64,
) -> Result<ProcessRequest, OpError> {
    if run.argv.is_empty() {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            "name the program to run",
        ));
    }
    let deadline = run.deadline.unwrap_or(USER_CEILING).min(USER_CEILING);
    let env = (place != verbs.cwd())
        .then(|| EnvVar {
            name: "PWD".to_string(),
            value: Some(place.to_string_lossy().into_owned()),
        })
        .into_iter()
        .collect();
    let request = ProcessRequest {
        argv: run.argv.clone(),
        env,
        deadline_ms: u64::try_from(deadline.as_millis()).unwrap_or(u64::MAX),
        output_cap,
        ..ProcessRequest::default()
    };
    verbs.permit(&EffectRequest::Process(request.clone()))?;
    Ok(request)
}

/// `run` passing the program's stdin, stdout and stderr through, in the current directory: how it ended. It is
/// journaled for audit (ADR 0337 D3) as its argv's digest and how it ended, with no output: what it printed is the
/// terminal's, so no record answers it again.
pub fn pass_through(verbs: &Verbs, run: &Run) -> Result<ProcessClass, OpError> {
    let place = place(verbs, run)?;
    let request = request(verbs, run, &place, u64::MAX)?;
    let asked = EffectRequest::Process(request.clone());
    let effects = Effects::of(
        EffectsMode::Online,
        verbs.ports().clone(),
        Vec::new(),
        EffectJournal::for_root(verbs.root()),
    );
    let mut ended = None;
    effects.serve_user(verbs.ports(), &asked, UnixMillis::now_or_end(), |ports| {
        let outcome = ports
            .process
            .run_within(&request, &place, Output::Inherit, Ceiling::User);
        let answer = Answer {
            outcome: Outcome {
                class: outcome.class.outcome(),
                digest: Blob::of(&[]).sha256,
            },
            payload: None,
        };
        ended = Some(outcome.class);
        answer
    });
    ended.ok_or_else(|| {
        OpError::new(
            OpErrorKind::Internal,
            OpErrorKind::Internal.as_str(),
            "the run was not performed",
        )
    })
}

/// `run --format json`: the program run with a null stdin and its output captured, through the effects journal in
/// `mode` (its record is the user's; `cached` performs it, since a program asks for something to happen, and `replay`
/// answers from the journal alone).
pub fn capture(
    verbs: &Verbs,
    run: &Run,
    mode: EffectsMode,
) -> Result<Answered<RunAnswer>, OpError> {
    let place = place(verbs, run)?;
    let mut request = request(verbs, run, &place, CAPTURE_CAP)?;
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
        let outcome = ports
            .process
            .run_within(&request, &place, Output::Buffer, Ceiling::User);
        let mut answer = answered(
            &run.argv,
            &outcome.class,
            &outcome.stdout,
            &outcome.stderr,
            started,
        );
        answer.deadline_ms = request.deadline_ms;
        // The kept answer holds no argv, which a typed command may carry a token in; it is the request's, put back
        // when the answer is read.
        answer.argv.clear();
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
        Served::Outcome {
            answer,
            from_journal,
            observed_at,
        } => answer
            .payload
            .as_deref()
            .and_then(|payload| serde_json::from_slice::<RunAnswer>(payload).ok())
            .map(|answer| Answered {
                answer: RunAnswer {
                    argv: run.argv.clone(),
                    ..answer
                },
                effect: Effect {
                    digest: Some(RecordDigest::asked_by(USER, &asked)),
                    observed_at,
                    from_journal,
                },
            })
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
    let (outcome, exit) = ending(class);
    RunAnswer {
        argv: argv.to_vec(),
        exit,
        outcome,
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
