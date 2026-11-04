//! What one call may do through the `specforge:host` ports (ADR 0335 D4): a [`PortSession`], made per call from its
//! kind and what its caller lent ([`PortCall`]). It decides every request before any effect, and it is the one place
//! a guest's request meets the host: what only the host may set is taken out of it here, in every runtime.
//!
//! An answer holds what may reach a guest and nothing else (ADR 0288): an attached secret is the granted port's, on
//! the host side, and a program a guest runs inherits no variable a declared secret names.

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use specforge_common::Diagnostic;
use specforge_effects::contained::withheld;
use specforge_effects::fs::{FileKind, READ_CEILING};
use specforge_effects::git::{GitPlace, Located, Sha};
use specforge_effects::http::{self, guest_body_cap};
use specforge_effects::journal::{Answer, Blob, LentRecord};
use specforge_effects::process::{DEADLINE_CEILING, OUTPUT_CAP, Output, ProcessClass};
use specforge_effects::{
    Denial as Refusal, Denied, EffectJournal, EffectRequest, Effects, FsOp, FsRequest, Grant,
    Granted, Outcome, OutcomeClass, Ports, Roots, Served, SharedFs, SharedGit, UnixMillis,
};
use specforge_protocol_types::{
    Denial, EnvVar, Failure, FsContent, FsEntry, FsEntryKind, FsListing, GitBlob, GitBlobs,
    GitRepo, GitSized, GitStatus, GitTreeEntry, HostError, HttpRequest, HttpResponse, Operation,
    PortScope, ProcessOutput, ProcessRequest, ProcessScope,
};

use crate::sandbox::{ANSWER_SHARE_OF_MEMORY, Limits, PORT_BUDGET_MS};

/// What an effectful call is lent: the effects its http and process requests are served through (the mode, the ports
/// a live mode holds, the journal), the local ports every mode performs, and its extension's grant.
#[derive(Clone)]
pub struct PortCall {
    pub effects: Arc<Effects>,
    pub fs: SharedFs,
    pub git: SharedGit,
    pub roots: Arc<Roots>,
    pub grant: Arc<Grant>,
    /// Every variable a declared secret names, of any extension: no program a guest runs inherits one.
    pub secrets: Vec<String>,
    /// E090, one per request the grant refused: the caller's to report.
    pub denials: Arc<Mutex<Vec<Diagnostic>>>,
}

impl PortCall {
    /// `grant` over `ports`, its http and process requests served through `effects`: the run's mode (ADR 0337 D4).
    pub fn new(
        effects: Effects,
        ports: &Ports,
        roots: Roots,
        grant: Grant,
        secrets: Vec<String>,
    ) -> Self {
        PortCall {
            effects: Arc::new(effects),
            fs: Arc::clone(&ports.fs),
            git: Arc::clone(&ports.git),
            roots: Arc::new(roots),
            grant: Arc::new(grant),
            secrets,
            denials: Arc::default(),
        }
    }

    /// [`Self::new`], `online`, with a record in `journal`.
    pub fn online(
        ports: &Ports,
        journal: EffectJournal,
        roots: Roots,
        grant: Grant,
        secrets: Vec<String>,
    ) -> Self {
        let effects = Effects::online(ports.clone(), journal);
        Self::new(effects, ports, roots, grant, secrets)
    }

    /// The denials kept so far, taken: what the caller reports once the call ended.
    pub fn take_denials(&self) -> Vec<Diagnostic> {
        std::mem::take(
            &mut *self
                .denials
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

/// What one call may do, and what it has spent.
pub struct PortSession {
    extension: String,
    kind: Option<Operation>,
    lent: Option<PortCall>,
    /// Any byte payload of an answer is held to this.
    answer_cap: u64,
    memory_mb: u32,
    /// What its requests took so far: not the guest's time, and held to the port budget.
    in_ports: Duration,
}

/// Variables a program's lookup or loader reads: a guest's request cannot set them, so the program a person approved
/// is the program that runs. shortcut: names only; extend the list when a platform adds a loader variable.
fn host_only_variable(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    name == "PATH" || name == "PATHEXT" || name.starts_with("LD_") || name.starts_with("DYLD_")
}

fn failed(class: impl ToString, message: impl Into<String>) -> HostError {
    HostError::Failed(Failure {
        class: class.to_string(),
        message: message.into(),
    })
}

fn out_of_scope(why: impl Into<String>) -> HostError {
    HostError::Denied(Denial::Scope(why.into()))
}

/// `bytes` held to `cap`, and whether that cut them.
fn held(mut bytes: Vec<u8>, cap: u64) -> (Vec<u8>, bool) {
    let cut = bytes.len() as u64 > cap;
    bytes.truncate(usize::try_from(cap).unwrap_or(usize::MAX));
    (bytes, cut)
}

/// What a guest asks a cap to be (0: the most it may have), held to `most`.
fn asked_cap(asked: u64, most: u64) -> u64 {
    if asked == 0 { most } else { asked.min(most) }
}

fn millis(span: std::time::Duration) -> u64 {
    u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
}

/// A commit a guest names: hex, so it can be neither an option nor a revision expression.
fn sha_of(text: &str) -> Result<Sha, HostError> {
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(Sha::new(text))
    } else {
        Err(out_of_scope(format!("'{text}' is no commit id")))
    }
}

/// A path a guest names under the repository's top: relative, and never climbing.
fn git_path(text: &str) -> Result<PathBuf, HostError> {
    let path = PathBuf::from(text);
    let normal = Path::new(text)
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    // A backslash is a separator on one platform and a name on another: what `withheld` judged may not be what opens.
    if normal && !text.contains('\\') {
        Ok(path)
    } else {
        Err(out_of_scope(format!(
            "'{text}' is not a path from the repository's top"
        )))
    }
}

impl PortSession {
    /// A call of a kind no port is for, or made outside any session: every port answers `denied(call-kind)`.
    pub fn pure() -> Self {
        PortSession {
            extension: String::new(),
            kind: None,
            lent: None,
            answer_cap: 0,
            memory_mb: 0,
            in_ports: Duration::ZERO,
        }
    }

    /// A call of `extension`, of `kind`, lent `lent` (none: no port is lent, so every request is `denied(scope)`),
    /// under `limits`.
    pub fn effectful(
        extension: &str,
        kind: Operation,
        lent: Option<PortCall>,
        limits: Limits,
    ) -> Self {
        PortSession {
            extension: extension.to_string(),
            kind: Some(kind),
            lent,
            answer_cap: u64::from(limits.memory_mb) * 1024 * 1024 / ANSWER_SHARE_OF_MEMORY,
            memory_mb: limits.memory_mb,
            in_ports: Duration::ZERO,
        }
    }

    /// How long this call has spent in its ports: what the guest's deadline does not count (ADR 0037 D3).
    pub fn time_in_ports(&self) -> Duration {
        self.in_ports
    }

    /// One port request, its time counted as the port's. A call that spent [`PORT_BUDGET_MS`] in its ports is
    /// answered `failed(deadline)`, as a guest cannot trap for time it does not count.
    pub fn timed<T>(
        &mut self,
        request: impl FnOnce(&mut Self) -> Result<T, HostError>,
    ) -> Result<T, HostError> {
        if self.budget_left() == 0 {
            return Err(HostError::Failed(Failure {
                class: "deadline".to_string(),
                message: format!("the call spent its {PORT_BUDGET_MS} ms budget in host ports"),
            }));
        }
        let started = Instant::now();
        let answer = request(self);
        self.in_ports += started.elapsed();
        answer
    }

    /// What is left of the call's budget for its ports, in milliseconds.
    fn budget_left(&self) -> u64 {
        PORT_BUDGET_MS.saturating_sub(millis(self.in_ports))
    }

    /// What the call was lent, when its kind may use a port at all.
    fn lent(&self) -> Result<(Operation, &PortCall), HostError> {
        let Some(kind) = self.kind.filter(|kind| kind.is_effectful()) else {
            return Err(HostError::Denied(Denial::CallKind(
                "only a command, an MCP tool or resource and a collector may call a host port"
                    .to_string(),
            )));
        };
        match &self.lent {
            Some(call) => Ok((kind, call)),
            None => Err(out_of_scope(format!(
                "no port is lent to this call of '{}'",
                self.extension
            ))),
        }
    }

    /// `refusal` as a guest is answered it, with its E090 kept when the grant's decision is one (ADR 0335 D2.7).
    fn refused(call: &PortCall, refusal: Refusal) -> HostError {
        let message = format!("`{}`: {}", refusal.request, refusal.detail);
        if matches!(
            refusal.why,
            Denied::CallKind | Denied::Undeclared | Denied::Ceiling
        ) {
            call.denials
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(refusal.diagnostic());
        }
        match refusal.why {
            Denied::CallKind => HostError::Denied(Denial::CallKind(message)),
            _ => HostError::Denied(Denial::Scope(message)),
        }
    }

    /// The capability that permits `request`, or why the grant refuses it.
    fn granted<'c>(
        kind: Operation,
        call: &'c PortCall,
        request: &EffectRequest,
    ) -> Result<&'c Granted, HostError> {
        call.grant
            .permitted(kind, request)
            .map_err(|refusal| Self::refused(call, refusal))
    }

    /// One HTTP request, through the call's effects, under its grant (which attaches its secret on the host side).
    pub fn http(&mut self, request: HttpRequest) -> Result<HttpResponse, HostError> {
        let (kind, call) = self.lent()?;
        // What only the host may set, whatever a guest (in process, a Rust value) put there.
        let request = HttpRequest {
            allow_private: false,
            secret: None,
            body_cap: asked_cap(request.body_cap, guest_body_cap(u64::from(self.memory_mb))),
            timeout_ms: asked_cap(
                request.timeout_ms,
                millis(http::TIMEOUT_CEILING).min(self.budget_left()),
            ),
            max_redirects: request.max_redirects.min(http::MAX_REDIRECTS),
            ..request
        };
        let asked = EffectRequest::Http(request.clone());
        let lent = lent_record(call, Self::granted(kind, call, &asked)?);
        let mut live = None;
        let served = call.effects.serve_lent(lent, &asked, now(), |ports| {
            let granted = call.grant.http(kind, Arc::clone(&ports.http));
            let outcome = granted.request(&request, &http::Any);
            let answer = answered(outcome.class, &outcome.body);
            live = Some(outcome);
            answer
        });
        let (class, status, headers, body, final_url, detail) = match (served, live) {
            (Served::Outcome { .. }, Some(outcome)) => (
                outcome.class,
                outcome.status,
                outcome.headers,
                outcome.body,
                outcome.final_url,
                outcome.detail,
            ),
            (Served::Outcome { answer, .. }, None) => (
                answer.outcome.class,
                status_of(answer.outcome.class),
                Vec::new(),
                answer.payload.unwrap_or_default(),
                request.url.clone(),
                None,
            ),
            (Served::Miss, _) => return Err(not_in_journal(&asked)),
            (_, _) => return Err(not_performed(&asked)),
        };
        let detail = detail.unwrap_or_else(|| class.to_string());
        match (class, status) {
            (OutcomeClass::Denied, _) => Err(out_of_scope(detail)),
            (OutcomeClass::Ok | OutcomeClass::Status(_) | OutcomeClass::Capped, Some(status)) => {
                let (body, cut) = held(body, self.answer_cap);
                Ok(HttpResponse {
                    status,
                    headers,
                    body,
                    truncated: cut || class == OutcomeClass::Capped,
                    final_url,
                })
            }
            _ => Err(failed(class, detail)),
        }
    }

    /// An fs request the grant permits, performed here: a local read in every mode, never journaled.
    fn fs(&self, request: FsRequest) -> Result<specforge_effects::FsOutcome, HostError> {
        let (kind, call) = self.lent()?;
        // `<root>/…` names another root than the one the grant judges.
        if request.path.starts_with('<') {
            return Err(out_of_scope(format!(
                "'{}' names a root inside its path: name it as the request's root",
                request.path
            )));
        }
        let asked = EffectRequest::Fs(request.clone());
        Self::granted(kind, call, &asked)?;
        let outcome = call
            .grant
            .fs(kind, Arc::clone(&call.fs))
            .run(&call.roots, &request);
        match outcome.class {
            OutcomeClass::Denied => Err(out_of_scope(format!(
                "`{}` is withheld",
                specforge_effects::grant::one_line(&asked)
            ))),
            _ => Ok(outcome),
        }
    }

    pub fn fs_stat(&mut self, root: &str, path: &str) -> Result<Option<FsEntry>, HostError> {
        let outcome = self.fs(FsRequest {
            root: root.to_string(),
            path: path.to_string(),
            op: FsOp::Stat,
        })?;
        match (outcome.class, outcome.stat) {
            (OutcomeClass::Ok, Some(stat)) => Ok(Some(FsEntry {
                kind: match stat.kind {
                    FileKind::File => FsEntryKind::File,
                    FileKind::Dir => FsEntryKind::Dir,
                    FileKind::Other => FsEntryKind::Other,
                },
                size: stat.size,
                modified_ms: stat.modified_ms,
            })),
            (OutcomeClass::Missing, _) => Ok(None),
            (class, _) => Err(failed(class, format!("{root}/{path}: {class}"))),
        }
    }

    pub fn fs_read(
        &mut self,
        root: &str,
        path: &str,
        max_bytes: u64,
    ) -> Result<FsContent, HostError> {
        let cap = asked_cap(max_bytes, self.answer_cap.min(READ_CEILING));
        let outcome = self.fs(FsRequest {
            root: root.to_string(),
            path: path.to_string(),
            op: FsOp::Read { max_bytes: cap },
        })?;
        if outcome.class != OutcomeClass::Ok {
            return Err(failed(
                outcome.class,
                format!("{root}/{path}: {}", outcome.class),
            ));
        }
        let (bytes, cut) = held(outcome.bytes, cap);
        Ok(FsContent {
            bytes,
            truncated: cut || outcome.truncated,
        })
    }

    pub fn fs_walk(
        &mut self,
        root: &str,
        path: &str,
        glob: Option<&str>,
        max_entries: u32,
    ) -> Result<FsListing, HostError> {
        let outcome = self.fs(FsRequest {
            root: root.to_string(),
            path: path.to_string(),
            op: FsOp::Walk {
                glob: glob.map(str::to_string),
                max_entries,
            },
        })?;
        if outcome.class != OutcomeClass::Ok {
            return Err(failed(
                outcome.class,
                format!("{root}/{path}: {}", outcome.class),
            ));
        }
        let mut room = self.answer_cap;
        let mut entries = outcome.entries;
        let kept = entries
            .iter()
            .take_while(|entry| {
                let fits = entry.len() as u64 <= room;
                room = room.saturating_sub(entry.len() as u64);
                fits
            })
            .count();
        let cut = kept < entries.len();
        entries.truncate(kept);
        Ok(FsListing {
            entries,
            truncated: cut || outcome.truncated,
        })
    }

    /// One program, its argv as a capability declares it, in the project's directory, through the call's effects.
    pub fn process_run(&mut self, request: ProcessRequest) -> Result<ProcessOutput, HostError> {
        let (kind, call) = self.lent()?;
        if let Some(var) = request.env.iter().find(|var| host_only_variable(&var.name)) {
            return Err(out_of_scope(format!(
                "a guest cannot set {}: it chooses what the approved command runs",
                var.name
            )));
        }
        let mut env = request.env;
        // Removed after what the guest asked: a program cannot echo a secret into its answer.
        env.extend(call.secrets.iter().map(EnvVar::removed));
        let request = ProcessRequest {
            env,
            output_cap: asked_cap(request.output_cap, self.answer_cap.min(OUTPUT_CAP)),
            ..request
        };
        let asked = EffectRequest::Process(request.clone());
        let granted = Self::granted(kind, call, &asked)?;
        // What the capability declares is the most it runs for, under the port's ceiling.
        let most = match &granted.declared.port {
            PortScope::Process(ProcessScope {
                deadline_ms: Some(declared),
                ..
            }) => u64::from(*declared).min(millis(DEADLINE_CEILING)),
            _ => millis(DEADLINE_CEILING),
        };
        let request = ProcessRequest {
            deadline_ms: asked_cap(request.deadline_ms, most.min(self.budget_left())),
            ..request
        };
        let asked = EffectRequest::Process(request.clone());
        let lent = lent_record(call, granted);
        let mut live = None;
        let served = call.effects.serve_lent(lent, &asked, now(), |ports| {
            let granted = call.grant.process(kind, Arc::clone(&ports.process));
            let outcome = granted.run(&request, call.roots.project(), Output::Buffer);
            let answer = answered(outcome.class.outcome(), &outcome.stdout);
            live = Some(outcome);
            answer
        });
        let (class, stdout, stderr) = match (served, live) {
            (Served::Outcome { .. }, Some(outcome)) => {
                (outcome.class, outcome.stdout, outcome.stderr)
            }
            (Served::Outcome { answer, .. }, None) => {
                let class = match answer.outcome.class {
                    OutcomeClass::Ok => ProcessClass::Exited(0),
                    OutcomeClass::Exit(code) => ProcessClass::Exited(code),
                    OutcomeClass::Capped => ProcessClass::OutputCapped,
                    other => ProcessClass::CannotStart(other.to_string()),
                };
                (class, answer.payload.unwrap_or_default(), Vec::new())
            }
            (Served::Miss, _) => return Err(not_in_journal(&asked)),
            (_, _) => return Err(not_performed(&asked)),
        };
        let exit_code = match class {
            ProcessClass::Exited(code) => Some(code),
            ProcessClass::OutputCapped => None,
            other => {
                let program = request.argv.first().map_or("", String::as_str);
                return Err(failed(other.outcome(), format!("{program} {other}")));
            }
        };
        let (stdout, cut_out) = held(stdout, self.answer_cap);
        let (stderr, cut_err) = held(stderr, self.answer_cap - stdout.len() as u64);
        Ok(ProcessOutput {
            exit_code,
            stdout,
            stderr,
            truncated: cut_out || cut_err || exit_code.is_none(),
        })
    }

    /// The git port and the project's directory, when the grant lets this call read git.
    fn git(&self, question: &str) -> Result<(SharedGit, &Path), HostError> {
        let (kind, call) = self.lent()?;
        call.grant
            .permits_git(kind, question)
            .map_err(|refusal| Self::refused(call, refusal))?;
        Ok((Arc::clone(&call.git), call.roots.project()))
    }

    /// The project's repository, for a question that needs one.
    fn place(&self, question: &str) -> Result<(SharedGit, GitPlace), HostError> {
        let (git, dir) = self.git(question)?;
        match git.locate(dir) {
            Located::Repo(place) => Ok((git, place)),
            Located::NotARepository => Err(failed(
                "not_a_repository",
                format!("{} is in no git repository", dir.display()),
            )),
            Located::Unavailable(why) => Err(failed("unavailable", why)),
        }
    }

    pub fn git_locate(&mut self) -> Result<Option<GitRepo>, HostError> {
        let (git, dir) = self.git("locate")?;
        match git.locate(dir) {
            Located::Repo(place) => Ok(Some(GitRepo {
                toplevel: place.toplevel.display().to_string(),
                shallow: place.shallow,
            })),
            Located::NotARepository => Ok(None),
            Located::Unavailable(why) => Err(failed("unavailable", why)),
        }
    }

    pub fn git_commit(&mut self, revision: &str) -> Result<Option<String>, HostError> {
        // No option, and nothing but a name: `<revision>^{commit}` is what the port asks.
        if revision.is_empty()
            || revision.starts_with('-')
            || revision.chars().any(char::is_whitespace)
        {
            return Err(out_of_scope(format!("'{revision}' is no revision")));
        }
        let (git, place) = self.place("commit")?;
        git.commit(&place, revision)
            .map(|sha| sha.map(|sha| sha.as_str().to_string()))
            .map_err(git_failed)
    }

    pub fn git_merge_base(&mut self, a: &str, b: &str) -> Result<Option<String>, HostError> {
        let (a, b) = (sha_of(a)?, sha_of(b)?);
        let (git, place) = self.place("merge-base")?;
        git.merge_base(&place, &a, &b)
            .map(|sha| sha.map(|sha| sha.as_str().to_string()))
            .map_err(git_failed)
    }

    pub fn git_tree(&mut self, commit: &str, dir: &str) -> Result<Vec<GitTreeEntry>, HostError> {
        let (commit, dir) = (sha_of(commit)?, git_path(dir)?);
        let (git, place) = self.place("tree")?;
        let entries: Vec<GitTreeEntry> = git
            .tree(&place, &commit, &dir)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, file)| GitTreeEntry {
                path: path.to_string_lossy().replace('\\', "/"),
                file,
            })
            .collect();
        let bytes: u64 = entries.iter().map(|entry| entry.path.len() as u64).sum();
        self.fits(bytes, "a tree")?;
        Ok(entries)
    }

    pub fn git_sizes(
        &mut self,
        commit: &str,
        paths: &[String],
    ) -> Result<Vec<GitSized>, HostError> {
        let commit = sha_of(commit)?;
        let paths: Vec<PathBuf> = paths
            .iter()
            .map(|path| git_path(path))
            .collect::<Result<_, _>>()?;
        let (git, place) = self.place("sizes")?;
        Ok(git
            .sizes(&place, &commit, &paths)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, size)| GitSized {
                path: path.to_string_lossy().replace('\\', "/"),
                size,
            })
            .collect())
    }

    pub fn git_blobs(&mut self, commit: &str, paths: &[String]) -> Result<GitBlobs, HostError> {
        let commit = sha_of(commit)?;
        let mut wanted = Vec::with_capacity(paths.len());
        for path in paths {
            // A file the disk withholds is withheld at a commit too.
            if withheld(path) {
                return Err(out_of_scope(format!("`{path}` is withheld")));
            }
            wanted.push(git_path(path)?);
        }
        let (git, place) = self.place("blobs")?;
        let mut room = self.answer_cap;
        let mut truncated = false;
        let blobs = git
            .blobs(&place, &commit, &wanted)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, bytes)| {
                let bytes = bytes.filter(|bytes| {
                    let fits = !truncated && bytes.len() as u64 <= room;
                    truncated |= !fits;
                    room = room.saturating_sub(bytes.len() as u64);
                    fits
                });
                GitBlob {
                    path: path.to_string_lossy().replace('\\', "/"),
                    bytes,
                }
            })
            .collect();
        Ok(GitBlobs { blobs, truncated })
    }

    pub fn git_branch(&mut self) -> Result<String, HostError> {
        let (git, place) = self.place("branch")?;
        git.branch(&place).map_err(git_failed)
    }

    pub fn git_status(&mut self) -> Result<GitStatus, HostError> {
        let (git, place) = self.place("status")?;
        let status = git.status(&place).map_err(git_failed)?;
        let bytes: u64 = status
            .entries
            .iter()
            .map(|entry| entry.path.len() as u64)
            .sum();
        self.fits(bytes, "a status")?;
        Ok(status)
    }

    /// A listing that has no way to say it was cut fails `capped` past the answer's cap.
    fn fits(&self, bytes: u64, what: &str) -> Result<(), HostError> {
        if bytes > self.answer_cap {
            return Err(failed(
                OutcomeClass::Capped,
                format!(
                    "{what} of {bytes} bytes is past this call's {} byte answer",
                    self.answer_cap
                ),
            ));
        }
        Ok(())
    }
}

/// Whose a request's record is: `call`'s extension, through `granted`, with the secret it attaches (ADR 0335 D4).
fn lent_record<'c>(call: &'c PortCall, granted: &'c Granted) -> LentRecord<'c> {
    LentRecord {
        extension: call.grant.extension(),
        capability: &granted.declared.name,
        secret_env: granted.secret.as_ref().map(|secret| secret.env.as_str()),
    }
}

fn git_failed(error: specforge_effects::git::GitError) -> HostError {
    failed("unavailable", error.0)
}

/// `body` as the journal keeps it: its class, its digest and the bytes.
fn answered(class: OutcomeClass, body: &[u8]) -> Answer {
    Answer {
        outcome: Outcome {
            class,
            digest: Blob::of(body).sha256,
        },
        payload: Some(body.to_vec()),
    }
}

/// The status a journaled class stands for.
fn status_of(class: OutcomeClass) -> Option<u16> {
    match class {
        OutcomeClass::Ok => Some(200),
        OutcomeClass::Status(status) => Some(status),
        _ => None,
    }
}

fn now() -> UnixMillis {
    UnixMillis::now_or_end()
}

fn not_performed(request: &EffectRequest) -> HostError {
    failed(
        "not_performed",
        format!(
            "`{}` is not performed in this call's effects mode",
            specforge_effects::grant::one_line(request)
        ),
    )
}

fn not_in_journal(request: &EffectRequest) -> HostError {
    failed(
        "not_in_journal",
        format!(
            "`{}` is not in the effects journal",
            specforge_effects::grant::one_line(request)
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spent_port_budget_fails_the_request_before_it_runs() {
        let mut session = PortSession::pure();
        session.in_ports = Duration::from_millis(PORT_BUDGET_MS);
        let mut ran = false;
        let answer: Result<(), HostError> = session.timed(|_| {
            ran = true;
            Ok(())
        });
        assert!(
            matches!(&answer, Err(HostError::Failed(failure)) if failure.class == "deadline"),
            "{answer:?}"
        );
        assert!(!ran);
    }

    #[test]
    fn a_requests_time_is_the_ports_and_shortens_the_next_deadline() {
        let mut session = PortSession::pure();
        let answer: Result<(), HostError> = session.timed(|_| {
            std::thread::sleep(Duration::from_millis(5));
            Ok(())
        });
        assert!(answer.is_ok());
        assert!(session.time_in_ports() >= Duration::from_millis(5));

        session.in_ports = Duration::from_millis(PORT_BUDGET_MS - 100);
        assert_eq!(session.budget_left(), 100);
    }
}
