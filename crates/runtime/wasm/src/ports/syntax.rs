//! The `syntax` port (ADR 0345 D1): source files a guest names parsed by the host's reader with the guest's query,
//! inside the call's fs grant (git's at a revision). Like fs and git it reads, so it is performed in every mode and
//! never journaled; what it could not parse is noted on the call (W196, I211).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use specforge_effects::contained::withheld;
use specforge_effects::fs::{FILE_CEILING, FileKind, WALK_VISITS};
use specforge_effects::{EffectRequest, FsOp, FsRequest, OutcomeClass};
use specforge_protocol_types::{
    HostError, NotParsed, SyntaxAnswer, SyntaxFile, SyntaxFiles, SyntaxOutcome, SyntaxRequest,
    SyntaxSource,
};
use specforge_source::{Language, Loaded, SyntaxWork};

use super::fs::no_root_inside;
use super::git::{git_failed, git_path, posix, sha_of};
use super::{PortSession, failed};

/// A request's language no grammar reads.
const NO_GRAMMAR: &str = "no_grammar";
/// A request's query the reader refuses.
const INVALID_QUERY: &str = "invalid_query";

impl PortSession {
    pub fn syntax_parse(&mut self, request: SyntaxRequest) -> Result<SyntaxAnswer, HostError> {
        let (_, call) = self.lent()?;
        let reader = Arc::clone(&call.syntax);
        let Some(language) = Language::named(&request.language) else {
            let known: Vec<&str> = Language::ALL.iter().map(|l| l.name()).collect();
            return Err(failed(
                NO_GRAMMAR,
                format!(
                    "no grammar reads `{}`: the reader knows {}",
                    request.language,
                    known.join(", ")
                ),
            ));
        };
        let query = reader
            .query(language, &request.query)
            .map_err(|fault| failed(INVALID_QUERY, format!("the query at {fault}")))?;
        let Asked { paths, load, short } = match &request.source {
            SyntaxSource::Worktree { root } => self.worktree(root, &request.files)?,
            SyntaxSource::Revision { commit } => self.revision(commit, &request.files)?,
        };
        let (_, call) = self.lent()?;
        let read = reader.read(&query, &paths, &|path| load(path));
        let (files, kept) = self.held(read.files);
        // The answer counts the files it returns; the call's work is every parse made.
        let answered: SyntaxWork = read.per_file[..kept].iter().copied().sum();
        let mut not_parsed = Vec::new();
        let mut broken = Vec::new();
        for file in &files {
            match &file.outcome {
                SyntaxOutcome::NotParsed {
                    reason:
                        reason @ (NotParsed::Deadline | NotParsed::TooLarge | NotParsed::Unreadable),
                } => not_parsed.push((file.path.as_str(), *reason)),
                SyntaxOutcome::Parsed { errors, .. } if *errors > 0 => {
                    broken.push((file.path.as_str(), *errors));
                }
                _ => {}
            }
        }
        call.noted(&not_parsed, &broken, read.work);
        Ok(SyntaxAnswer {
            files,
            truncated: short,
            parsed: count(answered.parsed),
            cached: count(answered.cached),
        })
    }

    /// The files of `root` asked: a glob walked inside the grant, each path's read granted (one refused is the whole
    /// request's `denied(scope)` and E090), then read whole by the granted port.
    fn worktree(&self, root: &str, files: &SyntaxFiles) -> Result<Asked, HostError> {
        let (paths, short) = match files {
            SyntaxFiles::Paths(paths) => (paths.clone(), false),
            SyntaxFiles::Glob(glob) => {
                let walked = self.fs(FsRequest {
                    root: root.to_string(),
                    path: String::new(),
                    op: FsOp::Walk {
                        glob: Some(glob.clone()),
                        max_entries: u32::try_from(WALK_VISITS).unwrap_or(u32::MAX),
                    },
                })?;
                if walked.class != OutcomeClass::Ok {
                    return Err(failed(walked.class, format!("{root}: {}", walked.class)));
                }
                (walked.entries, walked.truncated)
            }
        };
        let (kind, call) = self.lent()?;
        let mut requests = BTreeMap::new();
        for path in &paths {
            no_root_inside(path)?;
            let request = FsRequest {
                root: root.to_string(),
                path: path.clone(),
                op: FsOp::Read {
                    max_bytes: FILE_CEILING,
                },
            };
            Self::granted(kind, call, &EffectRequest::Fs(request.clone()))?;
            requests.insert(path.clone(), request);
        }
        let fs = call.grant.fs(kind, Arc::clone(&call.fs));
        let roots = Arc::clone(&call.roots);
        let load = move |path: &str| -> Loaded {
            let Some(request) = requests.get(path) else {
                return Loaded::NotParsed(NotParsed::Missing);
            };
            let outcome = fs.read_whole(&roots, request);
            match outcome.class {
                OutcomeClass::Ok
                    if outcome.stat.is_some_and(|stat| stat.kind == FileKind::File) =>
                {
                    Loaded::Bytes(outcome.bytes)
                }
                OutcomeClass::Capped => Loaded::NotParsed(NotParsed::TooLarge),
                OutcomeClass::Missing => Loaded::NotParsed(NotParsed::Missing),
                // What the port refuses to read: a withheld name, a link out of the root, a file with a second name.
                OutcomeClass::Denied | OutcomeClass::Outside => {
                    Loaded::NotParsed(NotParsed::Denied)
                }
                _ => Loaded::NotParsed(NotParsed::Unreadable),
            }
        };
        Ok(Asked {
            paths,
            load: Box::new(load),
            short,
        })
    }

    /// The files of `commit` asked: a glob matched over its tree, each read as a blob inside the git grant; a withheld
    /// path is `denied`, a blob the port did not read `too_large`.
    fn revision(&self, commit: &str, files: &SyntaxFiles) -> Result<Asked, HostError> {
        let sha = sha_of(commit)?;
        let paths = match files {
            SyntaxFiles::Paths(paths) => paths.clone(),
            SyntaxFiles::Glob(glob) => {
                let (git, place) = self.place("tree")?;
                git.tree(&place, &sha, std::path::Path::new(""))
                    .map_err(git_failed)?
                    .into_iter()
                    .filter(|(_, file)| *file)
                    .map(|(path, _)| posix(&path))
                    .filter(|path| specforge_common::path_glob::matches(glob, path))
                    .collect()
            }
        };
        let mut wanted: Vec<PathBuf> = Vec::new();
        for path in paths.iter().filter(|path| !withheld(path)) {
            wanted.push(git_path(path)?);
        }
        let (git, place) = self.place("blobs")?;
        let mut blobs: BTreeMap<String, Option<Vec<u8>>> = git
            .blobs(&place, &sha, &wanted)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, bytes)| (posix(&path), bytes))
            .collect();
        // What the commit holds as a file, to tell a blob the port did not read from a path it does not hold.
        let sizes: BTreeMap<String, Option<u64>> = git
            .sizes(&place, &sha, &wanted)
            .map_err(git_failed)?
            .into_iter()
            .map(|(path, size)| (posix(&path), size))
            .collect();
        let mut loaded: BTreeMap<String, Loaded> = BTreeMap::new();
        for path in &paths {
            let one = if withheld(path) {
                Loaded::NotParsed(NotParsed::Denied)
            } else {
                match blobs.remove(path).flatten() {
                    Some(bytes) => Loaded::Bytes(bytes),
                    None if sizes.get(path).copied().flatten().is_some() => {
                        Loaded::NotParsed(NotParsed::TooLarge)
                    }
                    None => Loaded::NotParsed(NotParsed::Missing),
                }
            };
            loaded.insert(path.clone(), one);
        }
        let load = move |path: &str| -> Loaded {
            loaded
                .get(path)
                .cloned()
                .unwrap_or(Loaded::NotParsed(NotParsed::Missing))
        };
        Ok(Asked {
            paths,
            load: Box::new(load),
            short: false,
        })
    }

    /// `files` held to the call's answer cap: once their encoded size passes it, that file and the later ones are
    /// `not_parsed(capped)`. How many came before it.
    fn held(&self, files: Vec<SyntaxFile>) -> (Vec<SyntaxFile>, usize) {
        let mut room = self.answer_cap;
        let mut kept = 0;
        let files = files
            .into_iter()
            .enumerate()
            .map(|(at, mut file)| {
                let size = serde_json::to_vec(&file).map_or(u64::MAX, |bytes| bytes.len() as u64);
                if kept < at || size > room {
                    file.outcome = SyntaxOutcome::NotParsed {
                        reason: NotParsed::Capped,
                    };
                } else {
                    room -= size;
                    kept += 1;
                }
                file
            })
            .collect();
        (files, kept)
    }
}

/// What a request asks of the reader.
struct Asked {
    /// The paths, in the order asked.
    paths: Vec<String>,
    /// What loads each.
    load: Box<dyn Fn(&str) -> Loaded + Sync>,
    /// Whether a walk stopped short.
    short: bool,
}

fn count(files: usize) -> u32 {
    u32::try_from(files).unwrap_or(u32::MAX)
}
