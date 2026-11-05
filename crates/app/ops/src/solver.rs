//! The SMT solver the prove pass asks and doctor checks (ADR 0079).
//!
//! One port, [`Solver`]: whether the solver runs here, and its answer to one
//! SMT-LIB2 script. [`Z3`] is the production adapter: it runs `z3` from `PATH`
//! through the effect plane's Process port (ADR 0335), ends every call at a
//! deadline (its probe at [`Z3::PROBE_TIMEOUT`], a solve at its own), and words
//! the one W098 that says z3 does not run. Tests use
//! `specforge_ops::testing::ScriptedSolver`.

use std::time::Duration;
#[cfg(test)]
use std::time::Instant;

use specforge_common::{Diagnostic, codes};
use specforge_effects::process::request;
use specforge_effects::{Output, ProcessClass, ProcessOutcome, SharedProcess};

/// What `specforge analyze --prove` (the prove pass) and doctor (its toolchain
/// finding) ask an SMT solver.
pub trait Solver: Send + Sync {
    /// Whether the solver runs here: its version banner, or the W098 that says
    /// why not. Asked once per operation and never cached, so a solver
    /// installed while a server runs is seen on its next call.
    fn availability(&self) -> Availability;
    /// The solver's whole stdout for one SMT-LIB2 script, or why there is none.
    fn solve(&self, script: &str) -> Result<String, SolveFailure>;
}

/// Whether a solver runs here.
#[derive(Debug, Clone, PartialEq)]
pub enum Availability {
    /// It runs: the first line of its version banner.
    Ready { version: String },
    /// It does not: the W098 the prove pass reports and doctor states as its
    /// toolchain finding, word for word.
    Unavailable(Diagnostic),
}

/// Why a solve produced no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveFailure {
    /// The solver ran past its timeout and was stopped.
    TimedOut { after: Duration },
    /// The solver could not be run or ended without an answer.
    Failed,
}

/// The production adapter: `z3` from `PATH`, run by `process`.
#[derive(Clone)]
pub struct Z3 {
    process: SharedProcess,
    program: std::path::PathBuf,
    timeout: Duration,
    probe_timeout: Duration,
}

impl Z3 {
    /// How long one solve may take unless `specforge analyze --prove-timeout`
    /// says otherwise.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
    /// How long `z3 --version` may take before z3 counts as not running.
    pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

    /// z3 from `PATH`, run by `process`, each solve stopped past `timeout` ([`Self::DEFAULT_TIMEOUT`] unless
    /// `--prove-timeout` says otherwise).
    pub fn new(process: SharedProcess, timeout: Duration) -> Self {
        Self {
            process,
            program: std::path::PathBuf::from("z3"),
            timeout,
            probe_timeout: Self::PROBE_TIMEOUT,
        }
    }

    /// How long one solve may take.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(any(test, feature = "testing"))]
impl Z3 {
    /// The W098 for a z3 that is not on `PATH` (what `ScriptedSolver::not_found` answers).
    pub(crate) fn not_on_path() -> Diagnostic {
        unavailable(Unrunnable::NotOnPath)
    }
}

#[cfg(test)]
impl Z3 {
    /// The adapter over another program and probe timeout: a test's fake `z3`.
    pub(crate) fn at(
        program: impl AsRef<std::path::Path>,
        timeout: Duration,
        probe_timeout: Duration,
    ) -> Self {
        Self {
            program: program.as_ref().to_path_buf(),
            probe_timeout,
            ..Self::new(std::sync::Arc::new(specforge_process::Runner), timeout)
        }
    }
}

impl Z3 {
    /// `z3` with `args`, stopped past `deadline`.
    fn run(&self, args: &[&str], deadline: Duration) -> ProcessOutcome {
        // A request carries words: a program path that is not one is refused, never mangled into another path.
        let Some(program) = self.program.to_str() else {
            return ProcessOutcome::of(ProcessClass::CannotStart(format!(
                "its program path is not UTF-8: {}",
                self.program.display()
            )));
        };
        let argv = std::iter::once(program).chain(args.iter().copied());
        let mut asked = request(argv, deadline);
        asked.text = true;
        self.process
            .run(&asked, &std::env::temp_dir(), Output::Buffer)
    }
}

impl Solver for Z3 {
    fn availability(&self) -> Availability {
        let ran = self.run(&["--version"], self.probe_timeout);
        let why = match ran.class {
            ProcessClass::Exited(0) => {
                return Availability::Ready {
                    version: ran.text().lines().next().unwrap_or("z3").trim().to_string(),
                };
            }
            ProcessClass::NotFound => Unrunnable::NotOnPath,
            ProcessClass::DeadlineExceeded => Unrunnable::ProbeTimedOut(self.probe_timeout),
            other => Unrunnable::Ran(other),
        };
        Availability::Unavailable(unavailable(why))
    }

    fn solve(&self, script: &str) -> Result<String, SolveFailure> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "specforge-prove-{}-{}.smt2",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&tmp, script).map_err(|_| SolveFailure::Failed)?;
        let ran = self.run(&[&tmp.to_string_lossy()], self.timeout);
        let _ = std::fs::remove_file(&tmp);
        match ran.class {
            ProcessClass::Exited(_) => Ok(ran.text()),
            ProcessClass::DeadlineExceeded => Err(SolveFailure::TimedOut {
                after: self.timeout,
            }),
            _ => Err(SolveFailure::Failed),
        }
    }
}

/// Why z3 does not run here.
enum Unrunnable {
    NotOnPath,
    /// `z3 --version` ran or tried to and gave no version: how, in the words that follow it.
    Ran(ProcessClass),
    ProbeTimedOut(Duration),
}

/// The one W098 for a z3 that does not run: what the prove pass reports and
/// doctor states.
fn unavailable(why: Unrunnable) -> Diagnostic {
    const SKIPPED: &str = "`specforge analyze --prove` skips its consistency and entailment checks";
    const REINSTALL: &str =
        "reinstall z3 (https://github.com/Z3Prover/z3) so that `z3 --version` runs";
    let (message, suggestion) = match why {
        Unrunnable::NotOnPath => (
            format!("the SMT solver z3 is not on PATH: {SKIPPED}"),
            "install z3 (https://github.com/Z3Prover/z3) and put it on PATH",
        ),
        Unrunnable::Ran(class) => (
            format!("the SMT solver z3 does not run (`z3 --version` {class}): {SKIPPED}"),
            REINSTALL,
        ),
        Unrunnable::ProbeTimedOut(after) => (
            format!(
                "the SMT solver z3 does not run (`z3 --version` did not answer within {after:?}): {SKIPPED}"
            ),
            REINSTALL,
        ),
    };
    Diagnostic::new(codes::W098, message).with_suggestion(suggestion)
}

/// A real z3, or a failure that says how to get one: for the tests of what z3
/// answers.
#[cfg(test)]
pub(crate) fn real_z3() -> Z3 {
    let z3 = Z3::new(
        std::sync::Arc::new(specforge_process::Runner),
        Z3::DEFAULT_TIMEOUT,
    );
    assert!(
        matches!(z3.availability(), Availability::Ready { .. }),
        "z3 is required for this test: install it (brew install z3 / apt-get install z3); CI installs it"
    );
    z3
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    /// A fake `z3` (a shell script) in a temp directory.
    #[cfg(unix)]
    fn fake_z3(script: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("z3");
        std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, path)
    }

    #[specforge_test(
        behavior = "prove_declared_bounds_and_claims",
        verify = "a solver call ends at its timeout, even when the solver leaves a child process"
    )]
    fn process_outliving_the_timeout_is_killed() {
        let started = Instant::now();
        let z3 = Z3::at("sleep", Duration::from_secs(30), Duration::from_secs(5));
        let out = z3.run(&["30"], Duration::from_millis(200));
        assert_eq!(out.class, ProcessClass::DeadlineExceeded);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[cfg(unix)]
    #[specforge_test(
        behavior = "prove_declared_bounds_and_claims",
        verify = "a solver call ends at its timeout, even when the solver leaves a child process"
    )]
    fn a_solver_that_leaves_a_child_is_stopped_at_the_timeout() {
        // `sh` forks `sleep`, which keeps the stdout pipe after `sh` is killed.
        let started = Instant::now();
        let z3 = Z3::at("sh", Duration::from_secs(30), Duration::from_secs(5));
        let out = z3.run(&["-c", "sleep 30; true"], Duration::from_millis(200));
        assert_eq!(out.class, ProcessClass::DeadlineExceeded);
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:?}",
            started.elapsed()
        );
    }

    #[cfg(unix)]
    #[specforge_test(
        behavior = "prove_declared_bounds_and_claims",
        verify = "a solver call ends at its timeout, even when the solver leaves a child process"
    )]
    fn a_slow_solve_is_timed_out_after_the_adapters_timeout() {
        let (_dir, fake) = fake_z3(
            r#"[ "$1" = "--version" ] && { echo "Z3 version 4.13.0 - fake"; exit 0; }; sleep 30; true"#,
        );
        let z3 = Z3::at(fake, Duration::from_millis(200), Duration::from_secs(5));
        let started = Instant::now();
        let out = z3.solve("(check-sat)");
        assert_eq!(
            out,
            Err(SolveFailure::TimedOut {
                after: Duration::from_millis(200)
            })
        );
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[cfg(unix)]
    #[specforge_test(
        behavior = "prove_declared_bounds_and_claims",
        verify = "a solver call ends at its timeout, even when the solver leaves a child process"
    )]
    fn a_hung_probe_is_unavailable_after_the_probe_timeout() {
        let (_dir, fake) = fake_z3("sleep 30");
        let z3 = Z3::at(fake, Duration::from_secs(30), Duration::from_millis(200));
        let started = Instant::now();
        let Availability::Unavailable(w098) = z3.availability() else {
            panic!("a hung probe is unavailable")
        };
        assert!(
            w098.message.contains("did not answer within 200ms"),
            "{}",
            w098.message
        );
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_missing_program_is_unavailable() {
        let z3 = Z3::at(
            "/nonexistent/specforge-test/z3",
            Duration::from_secs(1),
            Duration::from_secs(1),
        );
        let Availability::Unavailable(w098) = z3.availability() else {
            panic!("a program that is not there is not available")
        };
        assert_eq!(w098, Z3::not_on_path());
        assert_eq!(
            w098.message,
            "the SMT solver z3 is not on PATH: `specforge analyze --prove` skips its consistency and entailment checks"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_z3_whose_version_fails_does_not_run() {
        let (_dir, fake) = fake_z3("exit 1");
        let z3 = Z3::at(fake, Duration::from_secs(1), Duration::from_secs(5));
        let Availability::Unavailable(w098) = z3.availability() else {
            panic!("a z3 that exits 1 does not run")
        };
        assert!(
            w098.message.contains("`z3 --version` exited with status 1"),
            "{}",
            w098.message
        );
    }

    #[specforge_test(port = "SmtSolver", verify = "SmtSolver contract is satisfied")]
    fn the_real_z3_keeps_the_port_contract() {
        let z3 = real_z3();
        let Availability::Ready { version } = z3.availability() else {
            unreachable!("real_z3 checked it")
        };
        assert!(version.starts_with("Z3 version"), "{version}");
        let sat = z3
            .solve("(declare-const x Real)\n(assert (> x 1.0))\n(check-sat)\n")
            .unwrap();
        assert_eq!(sat.lines().next(), Some("sat"));
        let unsat = z3
            .solve("(declare-const x Real)\n(assert (> x 1.0))\n(assert (< x 0.0))\n(check-sat)\n")
            .unwrap();
        assert_eq!(unsat.lines().next(), Some("unsat"));
    }
}
