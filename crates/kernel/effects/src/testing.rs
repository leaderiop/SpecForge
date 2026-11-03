//! The ports' fakes, and the contract every adapter of a port keeps: an adapter's tests and its fake's run the same
//! `*_contract` function.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use specforge_protocol_types::{
    FsOp, FsRequest, HttpRequest, OutcomeClass, ProcessRequest, Redirected, Unreachable,
};

use crate::fs::{FileKind, FsStat, READ_CEILING};
use crate::http::{Any, scheme, status_class};
use crate::{
    Contained, Fs, FsOutcome, Http, HttpOutcome, Lookup, Output, Ports, Process, ProcessClass,
    ProcessOutcome, Redirects, Resolved, RootName, Roots,
};

/// Ports of fakes that answer nothing: every program is not found, every URL unreachable, every file missing.
pub fn ports() -> Ports {
    Ports {
        process: Arc::new(FakeProcess::new()),
        http: Arc::new(FakeHttp::new()),
        fs: Arc::new(FakeFs::new()),
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
    fn run(&self, request: &ProcessRequest, _dir: &Path, output: Output<'_>) -> ProcessOutcome {
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

/// An HTTP port that answers from a route table, follows redirects as every adapter must, and records every request.
#[derive(Default)]
pub struct FakeHttp {
    routes: BTreeMap<String, Route>,
    requests: Mutex<Vec<HttpRequest>>,
}

#[derive(Clone)]
struct Route {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl FakeHttp {
    /// Every URL is unreachable (`unreachable:dns`).
    pub fn new() -> Self {
        FakeHttp::default()
    }

    /// `url` answers `status` with `body`.
    pub fn route(self, url: &str, status: u16, body: impl Into<Vec<u8>>) -> Self {
        self.route_with(url, status, Vec::new(), body)
    }

    /// `url` answers `status` with `headers` and `body`.
    pub fn route_with(
        mut self,
        url: &str,
        status: u16,
        headers: Vec<(&str, &str)>,
        body: impl Into<Vec<u8>>,
    ) -> Self {
        let headers = headers
            .into_iter()
            .map(|(n, v)| (n.to_ascii_lowercase(), v.to_string()))
            .collect();
        self.routes.insert(
            url.to_string(),
            Route {
                status,
                headers,
                body: body.into(),
            },
        );
        self
    }

    /// `from` redirects (302) to `to`.
    pub fn redirect(self, from: &str, to: &str) -> Self {
        self.route_with(from, 302, vec![("location", to)], Vec::new())
    }

    /// The requests made so far, in order (one per request, not per hop).
    pub fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().expect("no request panics").clone()
    }
}

impl Http for FakeHttp {
    fn request(&self, request: &HttpRequest, redirects: &dyn Redirects) -> HttpOutcome {
        self.requests
            .lock()
            .expect("no request panics")
            .push(request.clone());
        let mut url = request.url.clone();
        let mut hops = 0;
        loop {
            let Some(route) = self.routes.get(&url) else {
                return HttpOutcome::unreachable(&url, Unreachable::Dns, "no route");
            };
            let location = route
                .headers
                .iter()
                .find(|(n, _)| n == "location")
                .map(|(_, v)| v.clone());
            if let (300..=399, Some(to)) = (route.status, location) {
                let refused = if hops == request.max_redirects {
                    Some(Redirected::Limit)
                } else if scheme(&url) == "https" && scheme(&to) == "http" {
                    Some(Redirected::Downgrade)
                } else if !redirects.allow(&url, &to) {
                    Some(Redirected::Refused)
                } else {
                    None
                };
                if let Some(why) = refused {
                    return HttpOutcome::failed(&url, OutcomeClass::Redirect(why), to);
                }
                hops += 1;
                url = to;
                continue;
            }
            let cap = usize::try_from(request.body_cap).unwrap_or(usize::MAX);
            let capped = route.body.len() > cap;
            return HttpOutcome {
                class: if capped {
                    OutcomeClass::Capped
                } else {
                    status_class(route.status)
                },
                status: Some(route.status),
                final_url: url,
                headers: route
                    .headers
                    .iter()
                    .filter(|(n, _)| {
                        request
                            .keep_headers
                            .iter()
                            .any(|k| k.eq_ignore_ascii_case(n))
                    })
                    .cloned()
                    .collect(),
                body: route.body[..route.body.len().min(cap)].to_vec(),
                detail: None,
            };
        }
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

/// Where `request` leads in `roots`, as every `Fs` adapter decides it ([`Contained`] over `lookup`): the whole
/// [`Resolved`] (its root, root-relative path, where it lands and whether the written name or the landing is
/// withheld), the root itself an empty path; or the class of a refusal (`outside`) or of a path that is absent as
/// written (`missing`, a case variant included).
pub fn held(
    roots: &Roots,
    lookup: &dyn Lookup,
    request: &FsRequest,
) -> Result<Resolved, OutcomeClass> {
    let name = RootName::of(&request.root);
    let root = roots.get(&name).ok_or(OutcomeClass::Outside)?.to_path_buf();
    if matches!(request.path.as_str(), "" | "." | "./") {
        return Ok(Resolved {
            root: name,
            rel: String::new(),
            at: root,
            real: None,
            exists: true,
            case: None,
            backslashes: false,
            withheld: false,
        });
    }
    match Contained::new(roots, lookup).resolve(&request.path, name) {
        Ok(resolved) if resolved.exists && resolved.case.is_none() => Ok(resolved),
        Ok(_) => Err(OutcomeClass::Missing),
        Err(_) => Err(OutcomeClass::Outside),
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
