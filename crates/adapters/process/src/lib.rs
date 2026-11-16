//! The adapter behind the effect plane's `Process` port (ADR 0335): the one runner every spawn goes through. It
//! resolves the program ([`locate`]: through absolute PATH entries only, × PATHEXT on Windows, the request's own when
//! it sets them), passes the parent's environment (for a program a guest runs, only what [`inherited`] keeps) with the
//! request's variables, less a secret deny-list applied last ([`secret`]; for a program a request or an extension asks
//! for, the credential filter [`passed`]), holds the run and its output to the request's deadline
//! (at most 600 s) and output cap, and ends the program's whole process group (a Job object on Windows) when either
//! passes, or when the host itself is interrupted ([`stop_on_interrupt`]). A program that exits cleanly is never
//! killed: a child it left holding its output is read for a short grace, then left alone.

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
/// a name ending `_TOKEN`, `_AUTHTOKEN`, `_AUTH` (npm's `NPM_CONFIG__AUTH`), `_SECRET`, `PASSWORD` (`PGPASSWORD` too),
/// `_PASSWD`, `_PWD` (`MYSQL_PWD`; never `PWD` or `OLDPWD`, the working directories), `_PAT` (a personal access token,
/// `GH_PAT`), `_KEY` (`_API_KEY`, `_SECRET_KEY`, `_PRIVATE_KEY`) or `_CREDENTIALS` (`GOOGLE_APPLICATION_CREDENTIALS`
/// among them), or starting `AWS_`, in any case on every system. The one deny-list (ADR 0335 D1); only a grant passes a
/// secret.
pub fn secret(name: &OsStr) -> bool {
    let name = name.to_string_lossy().to_ascii_uppercase();
    [
        "_TOKEN",
        "_AUTHTOKEN",
        "_AUTH",
        "_SECRET",
        "PASSWORD",
        "_PASSWD",
        "_PWD",
        "_PAT",
        "_KEY",
        "_CREDENTIALS",
    ]
    .iter()
    .any(|end| name.ends_with(end))
        || name.starts_with("AWS_")
}

/// Whether `name` (any case) is where cargo finds a registry credential beside a token, which [`secret`] withholds: a
/// credential provider (`CARGO_REGISTRY_CREDENTIAL_PROVIDER`, `CARGO_REGISTRIES_<NAME>_CREDENTIAL_PROVIDER`,
/// `CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS`) or a provider's own setting (`CARGO_CREDENTIAL_*`).
pub fn credential(name: &OsStr) -> bool {
    let name = name.to_string_lossy().to_ascii_uppercase();
    name.starts_with("CARGO_CREDENTIAL_")
        || name == "CARGO_REGISTRY_CREDENTIAL_PROVIDER"
        || name == "CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS"
        || name
            .strip_prefix("CARGO_REGISTRIES_")
            .is_some_and(|rest| rest.ends_with("_CREDENTIAL_PROVIDER"))
}

/// What a program a request or an extension asks for receives of one variable of the host's environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Passed {
    /// The value as the host holds it.
    Whole,
    /// The value with every URL's user and password removed (`https://user:pass@host/` is `https://host/`).
    Without(OsString),
    /// Nothing: the variable is removed.
    Withheld,
}

/// The credential filter, one for a program a guest runs and for a collector's command the host runs: what of the
/// variable `name`, holding `value`, the program receives. A [`secret`] or a cargo [`credential`] is withheld. A URL
/// carrying a user and password (a proxy, a registry index, a rustup or npm server, a database) passes without them,
/// whatever the variable's name, so the program still reaches the same host and a server that needs them refuses it
/// in the open, where a withheld proxy would send it elsewhere. A [`proxy`] written without a scheme
/// (`user:pass@proxy:8080`, as curl reads one) is cleaned the same way. A value that is not Unicode and may hold one is
/// withheld, as it cannot be cleaned. A URL with neither passes whole.
pub fn passed(name: &OsStr, value: &OsStr) -> Passed {
    if secret(name) || credential(name) {
        return Passed::Withheld;
    }
    match value.to_str() {
        Some(text) => {
            let clean = if proxy(name) && !text.contains("://") {
                without_userinfo(&format!("http://{text}"))
                    .map(|clean| clean["http://".len()..].to_string())
            } else {
                without_userinfo(text)
            };
            clean.map_or(Passed::Whole, |clean| Passed::Without(clean.into()))
        }
        None => {
            let lossy = value.to_string_lossy();
            if lossy.contains("://") && lossy.contains('@') {
                Passed::Withheld
            } else {
                Passed::Whole
            }
        }
    }
}

/// Whether `name` (any case) holds a proxy a program fetches through: one ending `_PROXY` (`HTTP_PROXY`,
/// `HTTPS_PROXY`, `ALL_PROXY`, `CARGO_HTTP_PROXY`, …), never one ending `NO_PROXY`, which lists hosts.
pub fn proxy(name: &OsStr) -> bool {
    let name = name.to_string_lossy().to_ascii_uppercase();
    name.ends_with("_PROXY") && !name.ends_with("NO_PROXY")
}

/// `text` with the user information removed from each URL in it (`scheme://<userinfo>@host`), None when it holds none
/// to remove. A user with a password is removed under any scheme; a user alone only under an HTTP scheme (`https`,
/// `sparse+https`, `git+http`, …), where a token is written as the user, never under another (`ssh://git@host` names
/// the account ssh logs in as, no secret). The user information runs to the last `@` of the authority, so a password
/// holding an `@`, or an unencoded `/`, `?` or `#` (`postgres://user:pa#ss@db/x`), is removed whole: once a `user:`
/// starts the authority and what follows the `:` is no port, the authority runs past those to its `@`.
pub fn without_userinfo(text: &str) -> Option<String> {
    let delimiter = |c: char| matches!(c, '/' | '?' | '#') || c.is_whitespace();
    let mut clean = String::with_capacity(text.len());
    let mut rest = text;
    let mut changed = false;
    while let Some(at) = rest.find("://") {
        let scheme_start = rest[..at]
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
            .map_or(0, |i| i + 1);
        let scheme = rest[scheme_start..at].to_ascii_lowercase();
        let http = scheme
            .rsplit('+')
            .next()
            .is_some_and(|s| s == "http" || s == "https");
        let start = at + 3;
        let after = &rest[start..];
        // The URL ends at whitespace; its authority, read strictly, at the first delimiter.
        let url = &after[..after.find(char::is_whitespace).unwrap_or(after.len())];
        let strict = &url[..url.find(delimiter).unwrap_or(url.len())];
        let password_shape = strict.split_once(':').is_some_and(|(user, after)| {
            !user.is_empty()
                && !user.starts_with('[')
                && !user.contains('@')
                && (after.is_empty() || !after.bytes().all(|b| b.is_ascii_digit()))
        });
        // The first `@` of the URL then each further one reached without a delimiter: the userinfo's end.
        let userinfo_end = url
            .find('@')
            .filter(|&first| first < strict.len() || password_shape)
            .map(|first| {
                let mut end = first;
                while let Some(next) = url[end + 1..].find(|c: char| c == '@' || delimiter(c)) {
                    let next = end + 1 + next;
                    if url.as_bytes()[next] != b'@' {
                        break;
                    }
                    end = next;
                }
                end
            });
        clean.push_str(&rest[..start]);
        let authority_end = match userinfo_end {
            Some(end) if http || url[..end].contains(':') => {
                let host = &url[end + 1..];
                let host_end = host.find(delimiter).unwrap_or(host.len());
                clean.push_str(&host[..host_end]);
                changed = true;
                end + 1 + host_end
            }
            _ => {
                clean.push_str(strict);
                strict.len()
            }
        };
        rest = &after[authority_end..];
    }
    clean.push_str(rest);
    changed.then_some(clean)
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

/// Whether the declared entry `known` names the variable `name`: the same name, or, for an entry ending `*`, a name it
/// starts. In any case on Windows.
fn names(known: &str, name: &str) -> bool {
    match known.strip_suffix('*') {
        Some(prefix) => name
            .get(..prefix.len())
            .is_some_and(|head| same_variable(prefix, head)),
        None => same_variable(known, name),
    }
}

/// Whether a program a guest runs inherits the variable `name`: what any program needs to work (the locale's `LC_*`
/// among it), or one its capability `declared`, by name or through a declared prefix (`CARGO_*`). In any case on
/// Windows. What it inherits then goes through the credential filter ([`passed`]), as a collector command's whole
/// environment does: a declared name or prefix never passes a secret, a cargo credential, or a URL's password.
pub fn inherited(name: &OsStr, declared: &[String]) -> bool {
    let name = name.to_string_lossy();
    let is = |known: &&str| same_variable(known, &name);
    name.starts_with("LC_")
        || NEEDED.iter().any(is)
        || (cfg!(windows) && NEEDED_ON_WINDOWS.iter().any(is))
        || declared.iter().any(|known| names(known, &name))
}

/// The extensions Windows tries when `PATHEXT` is not set.
const PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// Where the runner finds a program, before it runs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Located {
    /// Run this: a name with a directory as written (or with the extension Windows finds for it), or a bare name in an
    /// absolute `PATH` entry.
    At(PathBuf),
    /// A bare name found only through this relative or empty `PATH` entry, which names a directory relative to where
    /// the program runs, so a file planted there would run in the user's name: refused.
    Relative(PathBuf),
    /// Found nowhere.
    Missing,
}

impl Located {
    /// Why a program found only through a relative `PATH` entry is not run, and the remedy.
    pub fn refusal(program: &str, entry: &Path) -> String {
        let entry = if entry.as_os_str().is_empty() {
            "an empty PATH entry (the directory it runs in)".to_string()
        } else {
            format!("the relative PATH entry `{}`", entry.display())
        };
        format!(
            "`{program}` is found only through {entry}, and a relative PATH entry is never searched: name the program \
             by an absolute path, or put an absolute directory on PATH"
        )
    }
}

/// `program` as the runner finds it, with `path` (a `PATH` value) and `pathext`: on Windows the extensions it tries
/// ([`pathext`]; a name with no extension tries each in turn, and std quotes a `.cmd` or `.bat` batch-safely), None
/// elsewhere, where a file runs when it is executable. A name with a directory is taken as written. A bare name is
/// looked for in each absolute entry of `path` in turn; a relative or empty entry is skipped, as Go's `exec.LookPath`
/// and Rust's guidance do, and judged against `base` (where the program runs) only to say so.
pub fn locate(
    program: &str,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
    base: &Path,
) -> Located {
    let named = Path::new(program);
    if named.components().count() > 1 || named.is_absolute() {
        return Located::At(runnable(named, pathext).unwrap_or_else(|| named.to_path_buf()));
    }
    let Some(path) = path else {
        return Located::Missing;
    };
    let mut relative = None;
    for entry in std::env::split_paths(path) {
        if entry.is_absolute() {
            if let Some(found) = runnable(&entry.join(program), pathext) {
                return Located::At(found);
            }
        } else if relative.is_none()
            && runnable(&base.join(&entry).join(program), pathext).is_some()
        {
            relative = Some(entry);
        }
    }
    relative.map_or(Located::Missing, Located::Relative)
}

/// The extensions Windows tries for a name with none: `PATHEXT`'s value (`value`), or [`PATHEXT`] when unset. None
/// on any other system.
pub fn pathext(value: Option<OsString>) -> Option<OsString> {
    cfg!(windows).then(|| value.unwrap_or_else(|| PATHEXT.into()))
}

/// `candidate` as a program this user may run: with `pathext` (Windows), the file itself when it has an extension,
/// else the first of `pathext`'s that exists; without, the file when it is executable.
fn runnable(candidate: &Path, pathext: Option<&OsStr>) -> Option<PathBuf> {
    let Some(pathext) = pathext else {
        return executable(candidate).then(|| candidate.to_path_buf());
    };
    if candidate.extension().is_some() {
        return candidate.is_file().then(|| candidate.to_path_buf());
    }
    pathext
        .to_string_lossy()
        .split(';')
        .filter(|ext| ext.starts_with('.') && ext.len() > 1)
        .find_map(|ext| {
            let mut name = candidate.as_os_str().to_os_string();
            name.push(ext);
            let tried = PathBuf::from(name);
            tried.is_file().then_some(tried)
        })
}

impl Process for Runner {
    fn which(&self, program: &str) -> Option<PathBuf> {
        let base = std::env::current_dir().unwrap_or_default();
        let path = std::env::var_os("PATH");
        let pathext = pathext(std::env::var_os("PATHEXT"));
        match locate(program, path.as_deref(), pathext.as_deref(), &base) {
            Located::At(found) if executable(&found) => Some(found),
            _ => None,
        }
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
        let path = variable(request, "PATH");
        let pathext = pathext(variable(request, "PATHEXT"));
        let found = match locate(program, path.as_deref(), pathext.as_deref(), dir) {
            Located::At(found) => found,
            Located::Relative(entry) => {
                return at(ProcessClass::CannotStart(Located::refusal(program, &entry)));
            }
            Located::Missing => return at(ProcessClass::NotFound),
        };
        let mut command = Command::new(found);
        // The program sees its name as it was asked for, as a shell's run gives it.
        #[cfg(unix)]
        std::os::unix::process::CommandExt::arg0(&mut command, program);
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
        // The deny-list last: a request cannot put a secret back. A program a request or an extension asks for goes
        // through the whole credential filter; the host's own build keeps what fetching its crates needs.
        if !ceiling.keeps_secrets() {
            let filtered = ceiling.filters_credentials();
            let sets = |name: &OsStr| {
                let name = name.to_string_lossy();
                request
                    .env
                    .iter()
                    .any(|var| same_variable(&var.name, &name))
            };
            for (name, value) in std::env::vars_os() {
                let kept = match (filtered, secret(&name)) {
                    (true, _) => passed(&name, &value),
                    (false, true) => Passed::Withheld,
                    (false, false) => Passed::Whole,
                };
                match kept {
                    Passed::Whole => {}
                    Passed::Withheld => {
                        command.env_remove(&name);
                    }
                    // Cleaned only where the program inherits the host's value: not over a value the request sets.
                    Passed::Without(clean) => {
                        let inherits = request
                            .inherit
                            .as_ref()
                            .is_none_or(|declared| inherited(&name, declared));
                        if inherits && !sets(&name) {
                            command.env(&name, clean);
                        }
                    }
                }
            }
            for var in &request.env {
                let name = OsStr::new(&var.name);
                if secret(name) || (filtered && credential(name)) {
                    command.env_remove(name);
                }
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
            // A missing working directory fails the spawn too (NotFound on Unix, "the directory name is invalid" on
            // Windows): told apart, it is not a missing program.
            Err(_) if !dir.is_dir() => {
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
        .find(|var| same_variable(&var.name, name))
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
