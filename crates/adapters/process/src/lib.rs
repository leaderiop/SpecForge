//! The adapter behind the effect plane's `Process` port (ADR 0335): the one runner every spawn goes through. It
//! resolves the program (PATH × PATHEXT on Windows, the request's own when it sets them), passes the parent's
//! environment (for a program a guest runs, only what [`inherited`] keeps) with the request's variables, less a secret
//! deny-list applied last, holds the run and its output to the
//! request's deadline (at most 600 s) and output cap, and ends the program's whole process group (a Job object on
//! Windows) when either passes, or when the host itself is interrupted ([`stop_on_interrupt`]). A program that exits
//! cleanly is never killed: a child it left holding its output is read for a short grace, then left alone.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use specforge_effects::process::{OUTPUT_CAP, same_variable, started_now, text};
use specforge_effects::{
    Ceiling, Output, Process, ProcessClass, ProcessOutcome, ProcessRequest, Shown,
};

mod group;

pub use group::{live, stop_all, stop_on_interrupt};

/// After the program exits, how long its output may still take to arrive; past it, what a child it left still holds
/// is not waited for.
const DRAIN_GRACE: Duration = Duration::from_millis(100);
/// How often the runner looks at the clock and the program while no output arrives.
const TICK: Duration = Duration::from_millis(5);

/// The one process runner.
#[derive(Debug, Clone, Copy, Default)]
pub struct Runner;

/// Whether the environment variable `name` looks like a secret a program the host runs is not given, whoever set it:
/// a name ending `_TOKEN`, `_SECRET`, `_PASSWORD`, `_KEY` or `_CREDENTIALS` (`GOOGLE_APPLICATION_CREDENTIALS`
/// among them), or starting `AWS_` (any case). The one deny-list (ADR 0335 D1); only a grant passes a secret.
pub fn secret(name: &OsStr) -> bool {
    let name = name.to_string_lossy().to_ascii_uppercase();
    ["_TOKEN", "_SECRET", "_PASSWORD", "_KEY", "_CREDENTIALS"]
        .iter()
        .any(|end| name.ends_with(end))
        || name.starts_with("AWS_")
}

/// What any program needs of the host's environment to work.
const NEEDED: &[&str] = &["PATH", "HOME", "TMPDIR", "TEMP", "TMP", "LANG", "LANGUAGE"];

/// What a program needs on Windows beside [`NEEDED`]: without `SystemRoot` a program cannot even start its sockets.
const NEEDED_ON_WINDOWS: &[&str] = &[
    "PATHEXT",
    "SystemRoot",
    "SystemDrive",
    "windir",
    "ComSpec",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "ProgramData",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "OS",
];

/// Whether a program a guest runs inherits the variable `name`: what any program needs to work (the locale's `LC_*`
/// among it), or one its capability `declared`. In any case on Windows.
pub fn inherited(name: &OsStr, declared: &[String]) -> bool {
    let name = name.to_string_lossy();
    let is = |known: &&str| same_variable(known, &name);
    name.starts_with("LC_")
        || NEEDED.iter().any(is)
        || (cfg!(windows) && NEEDED_ON_WINDOWS.iter().any(is))
        || declared.iter().any(|known| same_variable(known, &name))
}

/// The extensions Windows tries when `PATHEXT` is not set.
const PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// `program` as Windows finds it in `path` (a `PATH` value) with `pathext` (a `PATHEXT` value, `.COM;.EXE;.BAT;.CMD`
/// when unset): a name with no extension tries each extension in turn (std quotes a `.cmd` or `.bat` batch-safely),
/// in each directory in turn; a name with a directory is tried there only. `None`: nothing matches, and the caller
/// spawns the name as written.
pub fn resolve(program: &str, path: Option<&OsStr>, pathext: Option<&OsStr>) -> Option<PathBuf> {
    let extensions: Vec<String> = pathext
        .map_or_else(|| PATHEXT.into(), |p| p.to_string_lossy().into_owned())
        .split(';')
        .filter(|ext| ext.starts_with('.') && ext.len() > 1)
        .map(str::to_string)
        .collect();
    let named = Path::new(program);
    let candidates = |dir: &Path| -> Vec<PathBuf> {
        if named.extension().is_some() {
            vec![dir.join(program)]
        } else {
            extensions
                .iter()
                .map(|ext| dir.join(format!("{program}{ext}")))
                .collect()
        }
    };
    dirs(program, path)?
        .iter()
        .flat_map(|dir| candidates(dir))
        .find(|candidate| candidate.is_file())
}

/// Where `program` is looked for: as named, when it names a directory, else each directory of `path` (none: no
/// `PATH`).
fn dirs(program: &str, path: Option<&OsStr>) -> Option<Vec<PathBuf>> {
    let named = Path::new(program);
    if named.components().count() > 1 || named.is_absolute() {
        Some(vec![PathBuf::new()])
    } else {
        Some(std::env::split_paths(path?).collect())
    }
}

impl Process for Runner {
    fn which(&self, program: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH");
        if cfg!(windows) {
            return resolve(
                program,
                path.as_deref(),
                std::env::var_os("PATHEXT").as_deref(),
            );
        }
        dirs(program, path.as_deref())?
            .iter()
            .map(|dir| dir.join(program))
            .find(|candidate| executable(candidate))
    }

    fn run_within(
        &self,
        request: &ProcessRequest,
        dir: &Path,
        output: Output<'_>,
        ceiling: Ceiling,
    ) -> ProcessOutcome {
        let started = started_now();
        let at = |class| ProcessOutcome {
            started,
            ..ProcessOutcome::of(class)
        };
        let Some((program, args)) = request.argv.split_first() else {
            return at(ProcessClass::CannotStart("no command".into()));
        };
        let program: OsString = if cfg!(windows) {
            let path = variable(request, "PATH");
            let pathext = variable(request, "PATHEXT");
            resolve(program, path.as_deref(), pathext.as_deref())
                .map_or_else(|| program.into(), PathBuf::into_os_string)
        } else {
            program.into()
        };
        let mut command = Command::new(program);
        command.args(args).current_dir(dir);
        if let Some(declared) = &request.inherit {
            command.env_clear();
            command.envs(std::env::vars_os().filter(|(name, _)| inherited(name, declared)));
        }
        for var in &request.env {
            match &var.value {
                Some(value) => command.env(&var.name, value),
                None => command.env_remove(&var.name),
            };
        }
        // The deny-list last: a request cannot put a secret back.
        let named = request.env.iter().map(|var| OsString::from(&var.name));
        for name in std::env::vars_os().map(|(name, _)| name).chain(named) {
            if secret(&name) && !ceiling.keeps_secrets() {
                command.env_remove(name);
            }
        }
        let terminal = ceiling.reads_the_terminal();
        command.stdin(match (&request.stdin, terminal) {
            (Some(_), _) => Stdio::piped(),
            (None, true) => Stdio::inherit(),
            (None, false) => Stdio::null(),
        });
        // A program in front of the person, on their terminal, leads its own group: that group takes the terminal.
        #[cfg(unix)]
        let in_front = request.stdin.is_none()
            && terminal
            && matches!(output, Output::Inherit)
            && group::Foreground::wanted();
        #[cfg(unix)]
        if in_front {
            group::Foreground::take_in_child(&mut command);
        }
        let (stdout, stderr) = match &output {
            Output::Buffer => (Stdio::piped(), Stdio::piped()),
            Output::Tee {
                shown: Shown::Discard,
                ..
            } => (Stdio::piped(), Stdio::null()),
            Output::Tee { .. } => (Stdio::piped(), Stdio::inherit()),
            Output::Inherit => (Stdio::inherit(), Stdio::inherit()),
        };
        command.stdout(stdout).stderr(stderr);
        group::isolate(&mut command);
        let mut child = match command.spawn() {
            Ok(child) => child,
            // A missing working directory also spawns NotFound: told apart, it is not a missing program.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !dir.is_dir() => {
                return at(ProcessClass::NoDirectory);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return at(ProcessClass::NotFound);
            }
            Err(e) => return at(ProcessClass::CannotStart(e.to_string())),
        };
        let mut group = match group::Group::of(&mut child) {
            Ok(group) => group,
            Err(why) => return at(ProcessClass::CannotStart(why)),
        };
        #[cfg(unix)]
        let _terminal = in_front
            .then(|| group::Foreground::hand_to(libc::pid_t::try_from(child.id()).unwrap_or(0)));
        let deadline = Duration::from_millis(request.deadline_ms).min(ceiling.limit());
        let cap = request.output_cap.min(OUTPUT_CAP);
        let mut outcome = Watch {
            child: &mut child,
            group: &mut group,
            deadline: Instant::now() + deadline,
            cap,
        }
        .run(request.stdin.clone(), output);
        outcome.started = started;
        if request.text {
            outcome.stdout = text(&outcome.stdout).into_bytes();
            outcome.stderr = text(&outcome.stderr).into_bytes();
        }
        outcome
    }
}

/// Whether `path` is a file this user may run.
fn executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::metadata(path)
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    path.is_file()
}

/// The value `name` has in the program's environment: the request's last word on it, else the parent's.
fn variable(request: &ProcessRequest, name: &str) -> Option<OsString> {
    match request
        .env
        .iter()
        .rev()
        .find(|var| var.name.eq_ignore_ascii_case(name))
    {
        Some(var) => var.value.as_ref().map(OsString::from),
        None => std::env::var_os(name),
    }
}

/// A started program, watched to its end.
struct Watch<'a> {
    child: &'a mut Child,
    group: &'a mut group::Group,
    deadline: Instant,
    cap: u64,
}

/// Which stream a chunk came from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stream {
    Out,
    Err,
}

impl Watch<'_> {
    fn run(self, stdin: Option<Vec<u8>>, mut output: Output<'_>) -> ProcessOutcome {
        if let (Some(input), Some(mut pipe)) = (stdin, self.child.stdin.take()) {
            std::thread::spawn(move || {
                let _ = pipe.write_all(&input);
            });
        }
        let (send, received) = mpsc::channel::<(Stream, Option<Vec<u8>>)>();
        let mut open = 0;
        if let Some(pipe) = self.child.stdout.take() {
            open += 1;
            forward(pipe, Stream::Out, send.clone());
        }
        if let Some(pipe) = self.child.stderr.take() {
            open += 1;
            forward(pipe, Stream::Err, send.clone());
        }
        drop(send);
        let mut shown: Box<dyn Write> = match &output {
            Output::Tee {
                shown: Shown::Terminal,
                ..
            } => Box::new(std::io::stdout()),
            Output::Tee {
                shown: Shown::Stderr,
                ..
            } => Box::new(std::io::stderr()),
            _ => Box::new(std::io::sink()),
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let (mut out_len, mut err_len) = (0u64, 0u64);
        let mut status = None;
        let mut exited_at: Option<Instant> = None;
        // Some: the run was stopped (the group is ended); None: the program exited (a process it started may still
        // hold its output past the grace: what it wrote so far is kept, and it is left running).
        let stopped = loop {
            if status.is_none() {
                match self.child.try_wait() {
                    Ok(Some(done)) => {
                        // Reaped: its group id may soon be another's, so it is never killed from here on.
                        self.group.release();
                        status = Some(done);
                        exited_at = Some(Instant::now());
                    }
                    Ok(None) => {}
                    Err(e) => break Some(ProcessClass::Lost(e.to_string())),
                }
            }
            if status.is_some() && open == 0 {
                break None;
            }
            let now = Instant::now();
            // A child left holding the output does not keep the run open past the grace, nor is it killed.
            if exited_at.is_some_and(|at| now >= at + DRAIN_GRACE) {
                break None;
            }
            if now >= self.deadline {
                // A program that exited in time is not past its deadline, whatever it left behind.
                if status.is_some() {
                    break None;
                }
                break Some(ProcessClass::DeadlineExceeded);
            }
            match received.recv_timeout(TICK.min(self.deadline - now)) {
                Ok((stream, Some(chunk))) => {
                    let len = if stream == Stream::Out {
                        &mut out_len
                    } else {
                        &mut err_len
                    };
                    *len += chunk.len() as u64;
                    if *len > self.cap {
                        break Some(ProcessClass::OutputCapped);
                    }
                    match (&mut output, stream) {
                        (Output::Buffer, Stream::Out) => out.extend_from_slice(&chunk),
                        (Output::Buffer, Stream::Err) => err.extend_from_slice(&chunk),
                        (Output::Tee { capture, .. }, Stream::Out) => {
                            if let Err(e) = capture.write_all(&chunk) {
                                break Some(ProcessClass::Uncaptured(format!("{e}")));
                            }
                            // The terminal going away mustn't lose the capture.
                            let _ = shown.write_all(&chunk).and_then(|()| shown.flush());
                        }
                        _ => {}
                    }
                }
                Ok((_, None)) => open -= 1,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => open = 0,
            }
        };
        let class = match stopped {
            Some(class) => {
                // A reaped program's group is not killed: its id may be another's.
                if status.is_none() {
                    self.group.kill(self.child);
                    let _ = self.child.wait();
                }
                class
            }
            None => match status {
                Some(status) => match status.code() {
                    Some(code) => ProcessClass::Exited(code),
                    None => ProcessClass::Signalled,
                },
                None => ProcessClass::Lost("it ended without a status".into()),
            },
        };
        ProcessOutcome {
            stdout: out,
            stderr: err,
            ..ProcessOutcome::of(class)
        }
    }
}

/// Read `pipe` to its end on a thread, sending each chunk as `stream`, then `None`.
fn forward(
    mut pipe: impl Read + Send + 'static,
    stream: Stream,
    send: mpsc::Sender<(Stream, Option<Vec<u8>>)>,
) {
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if send.send((stream, Some(buf[..n].to_vec()))).is_err() {
                        return;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        let _ = send.send((stream, None));
    });
}
