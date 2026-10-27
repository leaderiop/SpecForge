//! Running an outside program with a deadline: the one place a host crate spawns a process it must not wait on
//! forever (the solver, the git CLI).

use std::io::{Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// What a finished process left: its exit status and stdout.
pub struct Ran {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    /// What it wrote to stderr (kept for the caller that words a failure).
    pub stderr: Vec<u8>,
}

impl Ran {
    /// Stdout as text (invalid UTF-8 replaced).
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

/// Why a process gave no output in time.
#[derive(Debug)]
pub enum RunError {
    NotFound,
    CannotStart(std::io::Error),
    TimedOut,
    Lost,
}

/// After the process exits, how long its output may still take to arrive.
const DRAIN_GRACE: Duration = Duration::from_millis(100);

/// Run `cmd` (stdin null) and read its stdout and stderr, stopping at
/// `timeout`: the process is killed then, and output a child process keeps
/// open past the deadline is not waited for (its reader thread ends when that
/// child does).
pub fn run(cmd: &mut Command, timeout: Duration) -> Result<Ran, RunError> {
    run_with(cmd, None, timeout)
}

/// [`run`] with `input` written to the process's stdin, then closed.
pub fn run_with(
    cmd: &mut Command,
    input: Option<Vec<u8>>,
    timeout: Duration,
) -> Result<Ran, RunError> {
    let mut child = cmd
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RunError::NotFound
            } else {
                RunError::CannotStart(e)
            }
        })?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let mut stdout = child.stdout.take().ok_or(RunError::Lost)?;
    let mut stderr = child.stderr.take().ok_or(RunError::Lost)?;
    let (sender, received) = std::sync::mpsc::channel();
    let errors = sender.clone();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = sender.send((true, buf));
    });
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        let _ = errors.send((false, buf));
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::TimedOut);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => return Err(RunError::Lost),
        }
    };
    let left = deadline
        .saturating_duration_since(Instant::now())
        .max(DRAIN_GRACE);
    let (mut stdout, mut stderr) = (None, None);
    while stdout.is_none() || stderr.is_none() {
        let (is_stdout, buf) = received
            .recv_timeout(left)
            .map_err(|_| RunError::TimedOut)?;
        if is_stdout {
            stdout = Some(buf);
        } else {
            stderr = Some(buf);
        }
    }
    Ok(Ran {
        status,
        stdout: stdout.unwrap_or_default(),
        stderr: stderr.unwrap_or_default(),
    })
}
