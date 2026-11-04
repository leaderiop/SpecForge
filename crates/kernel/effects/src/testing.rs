//! The ports' fakes, and the contract every adapter of a port keeps: an adapter's tests and its fake's run the same
//! `*_contract` function.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use specforge_protocol_types::{FsOp, FsRequest, OutcomeClass, ProcessRequest};

use crate::fs::{FileKind, FsStat, READ_CEILING, held};
use crate::git::{
    Blob, CommitFile, Git, GitError, GitPlace, GitStatus, Located, Sha, Sized, TreeEntry,
};
use crate::http::Any;
use crate::{
    Ceiling, Fs, FsOutcome, Http, Lookup, Output, Ports, Process, ProcessClass, ProcessOutcome,
    Roots,
};

mod http;

pub use http::FakeHttp;

/// Ports of fakes that answer nothing: every program is not found, every URL unreachable, every file missing, no
/// directory a repository.
pub fn ports() -> Ports {
    Ports {
        process: Arc::new(FakeProcess::new()),
        http: Arc::new(FakeHttp::new()),
        fs: Arc::new(FakeFs::new()),
        git: Arc::new(FakeGit::none()),
    }
}

/// A process port that answers each program from a script, and records every request.
#[derive(Default)]
pub struct FakeProcess {
    answers: BTreeMap<String, (ProcessClass, Vec<u8>)>,
    requests: Mutex<Vec<ProcessRequest>>,
}

impl FakeProcess {
    /// Every program is not found.
    pub fn new() -> Self {
        FakeProcess::default()
    }

    /// `program` (an argv's first item) ends as `class` having written `stdout`.
    pub fn answering(
        mut self,
        program: &str,
        class: ProcessClass,
        stdout: impl Into<Vec<u8>>,
    ) -> Self {
        self.answers
            .insert(program.to_string(), (class, stdout.into()));
        self
    }

    /// The requests run so far, in order.
    pub fn requests(&self) -> Vec<ProcessRequest> {
        self.requests.lock().expect("no request panics").clone()
    }
}

impl Process for FakeProcess {
    fn run_within(
        &self,
        request: &ProcessRequest,
        _dir: &Path,
        output: Output<'_>,
        _ceiling: Ceiling,
    ) -> ProcessOutcome {
        self.requests
            .lock()
            .expect("no request panics")
            .push(request.clone());
        let Some(program) = request.argv.first() else {
            return ProcessOutcome::of(ProcessClass::CannotStart("no command".into()));
        };
        let Some((class, stdout)) = self.answers.get(program) else {
            return ProcessOutcome::of(ProcessClass::NotFound);
        };
        let cap = usize::try_from(request.output_cap).unwrap_or(usize::MAX);
        let (class, stdout) = if stdout.len() > cap {
            (ProcessClass::OutputCapped, &stdout[..cap])
        } else {
            (class.clone(), &stdout[..])
        };
        let mut outcome = ProcessOutcome::of(class);
        match output {
            Output::Buffer => outcome.stdout = stdout.to_vec(),
            Output::Tee { capture, .. } => {
                if let Err(error) = capture.write_all(stdout) {
                    outcome.class = ProcessClass::Lost(error.to_string());
                }
            }
            Output::Inherit => {}
        }
        outcome
    }
}

/// A file port over files held in memory, by absolute path; a directory is any prefix of a file's path. With a
/// link given ([`Self::with_link`]) it is a disk with links: a path is resolved through them.
#[derive(Default)]
pub struct FakeFs {
    files: BTreeMap<PathBuf, Vec<u8>>,
    links: BTreeMap<PathBuf, PathBuf>,
}

impl FakeFs {
    /// No file exists.
    pub fn new() -> Self {
        FakeFs::default()
    }

    /// The file at `path` holds `bytes`.
    pub fn with_file(mut self, path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> Self {
        self.files.insert(path.into(), bytes.into());
        self
    }

    /// The link at `path` points at `target` (relative to the link's directory, or absolute).
    pub fn with_link(mut self, path: impl Into<PathBuf>, target: impl Into<PathBuf>) -> Self {
        self.links.insert(path.into(), target.into());
        self
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.files.keys().any(|f| f != path && f.starts_with(path))
    }

    /// `path` with its last name's links followed (its parent already real), at most `MAX_LINKS` of them.
    fn followed(&self, path: &Path, hops: usize) -> Option<PathBuf> {
        match self.links.get(path) {
            Some(_) if hops >= crate::contained::MAX_LINKS => None,
            Some(target) => {
                let parent = path.parent().unwrap_or(Path::new(""));
                let landed = crate::contained::landing(self, &parent.join(target));
                self.followed(&landed, hops + 1)
            }
            None => {
                (self.files.contains_key(path) || self.is_dir(path)).then(|| path.to_path_buf())
            }
        }
    }
}

impl Fs for FakeFs {
    fn run(&self, roots: &Roots, request: &FsRequest) -> FsOutcome {
        let held = match held(roots, self, request) {
            Ok(held) => held,
            Err(class) => return FsOutcome::of(class),
        };
        let root = roots.get(&held.root).unwrap_or(&held.at).to_path_buf();
        let rel = held.rel.as_str();
        // Where the path lands: a link is read through.
        let at = held.real.clone().unwrap_or_else(|| held.at.clone());
        let file = self.files.get(&at);
        let stat = match file {
            Some(bytes) => FsStat {
                kind: FileKind::File,
                size: bytes.len() as u64,
                modified_ms: None,
            },
            None if self.is_dir(&at) => FsStat {
                kind: FileKind::Dir,
                size: 0,
                modified_ms: None,
            },
            None => return FsOutcome::of(OutcomeClass::Missing),
        };
        let mut outcome = FsOutcome::of(OutcomeClass::Ok);
        outcome.stat = Some(stat);
        match &request.op {
            FsOp::Stat => {}
            // The written name or where it lands: `notes.txt -> .env` is not read.
            FsOp::Read { .. } if held.withheld => return FsOutcome::of(OutcomeClass::Denied),
            FsOp::Read { max_bytes } => {
                let bytes = file.map(Vec::as_slice).unwrap_or_default();
                let max = usize::try_from((*max_bytes).min(READ_CEILING)).unwrap_or(usize::MAX);
                outcome.truncated = bytes.len() > max;
                outcome.bytes = bytes[..bytes.len().min(max)].to_vec();
            }
            FsOp::Walk { glob, max_entries } => {
                let mut entries: Vec<String> = self
                    .files
                    .keys()
                    .filter_map(|f| f.strip_prefix(&root).ok())
                    .filter(|f| f.starts_with(rel) || rel.is_empty())
                    .map(|f| f.to_string_lossy().replace('\\', "/"))
                    .filter(|f| {
                        glob.as_deref()
                            .is_none_or(|g| specforge_common::path_glob::matches(g, f))
                    })
                    .collect();
                entries.sort();
                let max = *max_entries as usize;
                outcome.truncated = entries.len() > max;
                entries.truncate(max);
                outcome.entries = entries;
            }
        }
        outcome
    }
}

impl Lookup for FakeFs {
    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        if self.links.is_empty() {
            return None;
        }
        self.followed(path, 0)
    }

    fn read_link(&self, path: &Path) -> Option<PathBuf> {
        self.links.get(path).cloned()
    }

    fn names(&self, dir: &Path) -> Option<Vec<String>> {
        let mut names: Vec<String> = self
            .files
            .keys()
            .chain(self.links.keys())
            .filter_map(|f| f.strip_prefix(dir).ok()?.components().next())
            .filter_map(|c| c.as_os_str().to_str().map(str::to_string))
            .collect();
        names.sort();
        names.dedup();
        (!names.is_empty()).then_some(names)
    }
}

/// The `Process` contract: an empty argv cannot start, a program that does not exist is not found, and neither
/// panics or waits.
pub fn process_contract(port: &dyn Process) {
    let dir = std::env::temp_dir();
    let empty = ProcessRequest::default();
    let outcome = port.run(&empty, &dir, Output::Buffer);
    assert!(
        matches!(outcome.class, ProcessClass::CannotStart(_)),
        "{outcome:?}"
    );
    let missing = crate::process::request(
        ["specforge-contract-no-such-program"],
        std::time::Duration::from_secs(5),
    );
    let outcome = port.run(&missing, &dir, Output::Buffer);
    assert_eq!(outcome.class, ProcessClass::NotFound, "{outcome:?}");
    assert!(outcome.started <= std::time::SystemTime::now());
}

/// The `Http` port contract: a name that cannot resolve is unreachable, with no status and no body.
pub fn http_contract(port: &dyn Http) {
    let outcome = port.request(
        &crate::http::get("http://specforge-contract.invalid/"),
        &Any,
    );
    assert!(
        matches!(outcome.class, OutcomeClass::Unreachable(_)),
        "{outcome:?}"
    );
    assert_eq!(outcome.status, None);
    assert!(outcome.body.is_empty());
}

/// The `Fs` contract over `roots`, whose project root holds `a.txt` with `a\n`, a `.env` and no `missing.txt`: the
/// file reads back, a missing one is missing, a path that climbs out of its root is outside, and a withheld name is
/// denied.
pub fn fs_contract(port: &dyn Fs, roots: &Roots) {
    let request = |path: &str, op: FsOp| FsRequest {
        root: "project".into(),
        path: path.into(),
        op,
    };
    let read = port.run(roots, &request("a.txt", FsOp::Read { max_bytes: 1024 }));
    assert_eq!(read.class, OutcomeClass::Ok, "{read:?}");
    assert_eq!(read.bytes, b"a\n");
    assert_eq!(read.stat.map(|s| s.kind), Some(FileKind::File));
    let missing = port.run(roots, &request("missing.txt", FsOp::Stat));
    assert_eq!(missing.class, OutcomeClass::Missing, "{missing:?}");
    let outside = port.run(roots, &request("../a.txt", FsOp::Stat));
    assert_eq!(outside.class, OutcomeClass::Outside, "{outside:?}");
    let denied = port.run(roots, &request(".env", FsOp::Read { max_bytes: 1024 }));
    assert_eq!(denied.class, OutcomeClass::Denied, "{denied:?}");
}

/// The `Git` contract over `dir`, a directory in no repository: it is located as no repository, or as git that
/// cannot be asked (not installed), never as a repository.
pub fn git_contract(port: &dyn Git, dir: &Path) {
    let located = port.locate(dir);
    assert!(
        matches!(located, Located::NotARepository | Located::Unavailable(_)),
        "{located:?}"
    );
}

/// Answers `located`, `head` and `files` as given.
pub struct FakeGit {
    pub located: Located,
    pub head: Result<Option<Sha>, GitError>,
    pub files: Result<Vec<CommitFile>, GitError>,
    /// Other revisions it knows by name: their commit and the files at it.
    pub named: Vec<(String, Sha, Vec<CommitFile>)>,
    /// The other files it holds at every commit (what `blobs` answers).
    pub blobs: Vec<CommitFile>,
    /// What `blobs` answers instead, when set: git failing to read the files.
    pub blobs_fail: Option<GitError>,
    /// The entries its tree holds besides the files and `blobs`, with the size `sizes` answers (`None`: a link or a
    /// submodule).
    pub listed: Vec<(PathBuf, Option<u64>)>,
    /// Regular files its tree lists that git cannot give: `sizes` answers `None` for them.
    pub lost: Vec<PathBuf>,
    /// Every path `blobs` was asked for, in order.
    pub blobs_paths: Mutex<Vec<PathBuf>>,
    /// What `sizes` answers instead, when set: git failing to read the sizes.
    pub sizes_fail: Option<GitError>,
    /// What `tree` answers instead, when set: git failing to list the tree.
    pub tree_fail: Option<GitError>,
    /// What `branch` answers.
    pub branch: String,
    /// What `status` answers.
    pub status: GitStatus,
    asked: AtomicUsize,
    trees: AtomicUsize,
    blobs_asks: AtomicUsize,
    any: AtomicUsize,
}

impl FakeGit {
    /// A repository at `toplevel` whose `HEAD` is `sha` and whose files at it are `files`.
    pub fn repo(toplevel: &Path, sha: &str, files: Vec<CommitFile>) -> Self {
        FakeGit {
            located: Located::Repo(GitPlace {
                toplevel: toplevel
                    .canonicalize()
                    .unwrap_or_else(|_| toplevel.to_path_buf()),
                git_dir: toplevel.join(".git"),
                common_dir: toplevel.join(".git"),
                shallow: false,
            }),
            head: Ok(Some(Sha::new(sha))),
            files: Ok(files),
            named: Vec::new(),
            blobs: Vec::new(),
            blobs_fail: None,
            listed: Vec::new(),
            lost: Vec::new(),
            blobs_paths: Mutex::new(Vec::new()),
            sizes_fail: None,
            tree_fail: None,
            branch: String::new(),
            status: GitStatus::default(),
            asked: AtomicUsize::new(0),
            trees: AtomicUsize::new(0),
            blobs_asks: AtomicUsize::new(0),
            any: AtomicUsize::new(0),
        }
    }

    /// A directory that is no repository.
    pub fn none() -> Self {
        FakeGit {
            located: Located::NotARepository,
            head: Ok(None),
            files: Ok(Vec::new()),
            named: Vec::new(),
            blobs: Vec::new(),
            blobs_fail: None,
            listed: Vec::new(),
            lost: Vec::new(),
            blobs_paths: Mutex::new(Vec::new()),
            sizes_fail: None,
            tree_fail: None,
            branch: String::new(),
            status: GitStatus::default(),
            asked: AtomicUsize::new(0),
            trees: AtomicUsize::new(0),
            blobs_asks: AtomicUsize::new(0),
            any: AtomicUsize::new(0),
        }
    }

    /// How many times the files at a commit were asked for.
    pub fn files_asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }

    /// How many times a commit's tree was asked for.
    pub fn trees_asked(&self) -> usize {
        self.trees.load(Ordering::SeqCst)
    }

    /// How many times named files were asked for.
    pub fn blobs_asked(&self) -> usize {
        self.blobs_asks.load(Ordering::SeqCst)
    }

    /// How many questions of any kind were asked.
    pub fn questions(&self) -> usize {
        self.any.load(Ordering::SeqCst)
    }
}

impl Git for FakeGit {
    fn branch(&self, _: &GitPlace) -> Result<String, GitError> {
        Ok(self.branch.clone())
    }
    fn status(&self, _: &GitPlace) -> Result<GitStatus, GitError> {
        Ok(self.status.clone())
    }
    fn locate(&self, _: &Path) -> Located {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.located.clone()
    }
    fn commit(&self, _: &GitPlace, revision: &str) -> Result<Option<Sha>, GitError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        if revision == "HEAD" {
            return self.head.clone();
        }
        Ok(self
            .named
            .iter()
            .find(|(name, ..)| name == revision)
            .map(|(_, sha, _)| sha.clone()))
    }
    fn merge_base(&self, _: &GitPlace, a: &Sha, _: &Sha) -> Result<Option<Sha>, GitError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        Ok(Some(a.clone()))
    }
    fn files(
        &self,
        _: &GitPlace,
        commit: &Sha,
        _: &Path,
        wanted: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<CommitFile>, GitError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.asked.fetch_add(1, Ordering::SeqCst);
        let files = match self.named.iter().find(|(_, sha, _)| sha == commit) {
            Some((_, _, files)) => files.clone(),
            None => self.files.clone()?,
        };
        Ok(files.into_iter().filter(|(path, _)| wanted(path)).collect())
    }
    fn tree(&self, _: &GitPlace, commit: &Sha, _: &Path) -> Result<Vec<TreeEntry>, GitError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.trees.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = &self.tree_fail {
            return Err(error.clone());
        }
        let files = match self.named.iter().find(|(_, sha, _)| sha == commit) {
            Some((_, _, files)) => files.clone(),
            None => self.files.clone()?,
        };
        Ok(files
            .iter()
            .chain(&self.blobs)
            .map(|(path, _)| (path.clone(), true))
            .chain(
                self.listed
                    .iter()
                    .map(|(path, size)| (path.clone(), size.is_some())),
            )
            .chain(self.lost.iter().map(|path| (path.clone(), true)))
            .collect())
    }
    fn sizes(&self, _: &GitPlace, _: &Sha, paths: &[PathBuf]) -> Result<Vec<Sized>, GitError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = &self.sizes_fail {
            return Err(error.clone());
        }
        let size = |path: &PathBuf| {
            let held = self.blobs.iter().find(|(held, _)| held == path);
            held.map(|(_, bytes)| bytes.len() as u64).or_else(|| {
                let listed = self.listed.iter().find(|(held, _)| held == path);
                listed.and_then(|(_, size)| *size)
            })
        };
        Ok(paths
            .iter()
            .map(|path| (path.clone(), size(path)))
            .collect())
    }
    fn blobs(&self, _: &GitPlace, _: &Sha, paths: &[PathBuf]) -> Result<Vec<Blob>, GitError> {
        self.any.fetch_add(1, Ordering::SeqCst);
        self.blobs_asks.fetch_add(1, Ordering::SeqCst);
        self.blobs_paths
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(paths.iter().cloned());
        if let Some(error) = &self.blobs_fail {
            return Err(error.clone());
        }
        Ok(paths
            .iter()
            .map(|path| {
                let held = self.blobs.iter().find(|(held, _)| held == path);
                (path.clone(), held.map(|(_, bytes)| bytes.clone()))
            })
            .collect())
    }
}

/// A consent store in memory (ADR 0335 D2.4): what it was given, or everything ([`MemoryConsent::approving_everything`]),
/// or an unusable store ([`MemoryConsent::unusable`]), or one that keeps nothing ([`MemoryConsent::unable_to_remember`]).
#[derive(Default)]
pub struct MemoryConsent {
    held: Mutex<crate::Approvals>,
    everything: bool,
    unusable: Option<crate::ConsentError>,
    /// Why a change cannot be kept: it reads, but nothing is written.
    unwritable: Option<crate::ConsentError>,
}

impl MemoryConsent {
    /// A store that approves every capability and root.
    pub fn approving_everything() -> Self {
        MemoryConsent {
            everything: true,
            ..Self::default()
        }
    }

    /// A store that cannot be read: `why` names the reason.
    pub fn unusable(why: &str) -> Self {
        MemoryConsent {
            unusable: Some(crate::ConsentError {
                store: "memory".to_string(),
                why: why.to_string(),
            }),
            ..Self::default()
        }
    }

    /// A store that reads but keeps no change: `why` names the reason.
    pub fn unable_to_remember(why: &str) -> Self {
        MemoryConsent {
            unwritable: Some(crate::ConsentError {
                store: "memory".to_string(),
                why: why.to_string(),
            }),
            ..Self::default()
        }
    }

    fn usable(&self) -> Result<(), crate::ConsentError> {
        self.unusable.clone().map_or(Ok(()), Err)
    }
}

impl crate::Consent for MemoryConsent {
    fn store(&self) -> String {
        "memory".to_string()
    }

    fn list(&self) -> Result<crate::Approvals, crate::ConsentError> {
        self.usable()?;
        Ok(self.held.lock().expect("consent lock").clone())
    }

    fn update(
        &self,
        change: &mut dyn FnMut(&mut crate::Approvals),
    ) -> Result<(), crate::ConsentError> {
        self.usable()?;
        if let Some(error) = &self.unwritable {
            return Err(error.clone());
        }
        change(&mut self.held.lock().expect("consent lock"));
        Ok(())
    }

    fn approved(&self, wanted: &crate::Approval) -> Result<bool, crate::ConsentError> {
        self.usable()?;
        Ok(self.everything || self.held.lock().expect("consent lock").covers(wanted))
    }
}

/// The contract every [`Consent`](crate::Consent) adapter keeps, given an empty store: nothing is approved; an
/// approval covers itself and a narrower scope; approving under the same key replaces it; a revoke removes it.
pub fn consent_contract(consent: &dyn crate::Consent) {
    use crate::{
        Approval, ApprovedAt, CapabilityApproval, KeyedBy, ProjectKey, Revoke, RootApproval,
    };
    use specforge_protocol_types::{GitOp, GitScope, PortScope};
    let git = |ops: &[GitOp]| CapabilityApproval {
        project: ProjectKey("/p".to_string()),
        extension: "@acme/x".to_string(),
        provenance: KeyedBy::Builtin,
        scope: "history".to_string(),
        granted: PortScope::Git(GitScope { ops: ops.to_vec() }),
        call_kinds: Vec::new(),
        secret_env: None,
        digest: None,
        specforge: String::new(),
        approved_at: ApprovedAt::default(),
    };
    let both = Approval::Capability(git(&[GitOp::Read, GitOp::Write]));
    let read = Approval::Capability(git(&[GitOp::Read]));
    assert!(
        consent
            .list()
            .expect("an empty store lists")
            .capabilities
            .is_empty()
    );
    assert!(!consent.approved(&read).expect("an empty store answers"));
    consent
        .approve(read.clone())
        .expect("an approval is stored");
    assert!(consent.approved(&read).expect("it answers"));
    assert!(
        !consent.approved(&both).expect("it answers"),
        "a wider scope asks again"
    );
    consent
        .approve(both.clone())
        .expect("an approval is stored");
    assert!(
        consent.approved(&read).expect("it answers"),
        "a wider approval covers a narrower grant"
    );
    assert_eq!(
        consent.list().expect("it lists").capabilities.len(),
        1,
        "the same key is replaced"
    );
    let root = Approval::Root(RootApproval {
        project: ProjectKey("/p".to_string()),
        root: "vendor".to_string(),
        target: "/v".to_string(),
        approved_at: ApprovedAt::default(),
    });
    consent
        .approve(root.clone())
        .expect("a root approval is stored");
    assert!(consent.approved(&root).expect("it answers"));
    let gone = consent
        .revoke(&Revoke::Project {
            project: ProjectKey("/p".to_string()),
        })
        .expect("a revoke is stored");
    assert_eq!(gone, 2);
    assert_eq!(
        consent.list().expect("it lists"),
        crate::Approvals::default()
    );
}
