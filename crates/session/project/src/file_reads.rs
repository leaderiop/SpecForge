//! The files a pass or command reads (ADR 0293): the host reads what the extension declared (`reads`), through the
//! disk it was given, and hands the facts and capped text to the guest, which has no file system.
//!
//! Every read of a file goes through `source.disk`, so a session's recording disk makes an edit to a read file, the
//! creation of an absent one and a new file under a globbed directory a check input (ADR 0107).
//!
//! A read for text withholds the text of a file that looks like a credential (`error: "denied"`, ADR 0293 D3).
//!
//! shortcut: the facts of every declared file are recomputed on each re-check, with no memo across runs (the `lines`
//! fact needs the bytes, so a stamp-keyed digest would not save the read); upgrade when `specforge doctor` shows
//! reads that are slow.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use specforge_protocol_types::{FileFact, FileFacts, FileRead};
use specforge_registry::{Disk, EntitySnapshot, paths_named, read_problem};

use crate::judged::file_digest;

/// The most text of one file a read sends.
pub const TEXT_CAP: usize = 64 * 1024;
/// The most text one read sends in all.
pub const TEXT_BUDGET: usize = 1024 * 1024;
/// The most files one read names.
pub const MAX_FILES: usize = 2000;
/// The most entries (files and directories) one glob visits, matched or not.
pub const MAX_VISITS: usize = 20_000;
/// Directories a glob does not enter unless a literal segment of the pattern names them: version control, the tool's
/// own state, and the usual build and dependency output.
const SKIPPED: [&str; 9] = [
    ".git",
    ".specforge",
    ".next",
    ".venv",
    "dist",
    "node_modules",
    "target",
    "vendor",
    ".svn",
];

/// Where the files are read from.
#[derive(Clone, Copy)]
pub struct FileSource<'a> {
    /// Every `exists`, `list` and `read` goes through it.
    pub disk: &'a dyn Disk,
    pub project_root: &'a Path,
    pub spec_root: &'a Path,
}

impl crate::Environment {
    /// The file reads over `disk`, rooted at this project.
    pub fn file_source<'a>(&'a self, disk: &'a dyn Disk) -> FileSource<'a> {
        FileSource {
            disk,
            project_root: self.root(),
            spec_root: self.spec_root(),
        }
    }
}

/// The reads made in one compile: passes that declare equal reads over the same entities share one walk of the disk.
#[derive(Default)]
pub struct Memo(Vec<(Vec<FileRead>, BTreeMap<String, FileFacts>)>);

impl Memo {
    /// [`read`], made once for each distinct `reads`.
    pub fn read(
        &mut self,
        reads: &[FileRead],
        entities: &EntitySnapshot,
        source: &FileSource<'_>,
    ) -> BTreeMap<String, FileFacts> {
        if let Some((_, facts)) = self.0.iter().find(|(made, _)| made == reads) {
            return facts.clone();
        }
        let facts = read(reads, entities, source);
        self.0.push((reads.to_vec(), facts.clone()));
        facts
    }
}

/// A read in progress: where the files are, and the project root with its links resolved (when the disk has links),
/// which every path read must stay under (ADR 0293: a link out of the project is `outside_project`).
struct Ctx<'a> {
    source: &'a FileSource<'a>,
    real_root: Option<PathBuf>,
}

impl Ctx<'_> {
    /// Whether `at`, with its links resolved, leaves the project. shortcut: the check and the read are two calls, so a
    /// link swapped between them is read; the same race `ContainedDisk` closes by holding the file, which a
    /// recording disk cannot do.
    fn escapes(&self, at: &Path) -> bool {
        match (&self.real_root, self.source.disk.canonical(at)) {
            (Some(root), Some(real)) => !real.starts_with(root),
            _ => false,
        }
    }
}

/// The facts `reads` ask for, by read name. A read that is malformed (`read_problem`) or repeats an earlier name is
/// left out: the declaration check reported it (W021), and the guest sees `files[name]` absent.
pub fn read(
    reads: &[FileRead],
    entities: &EntitySnapshot,
    source: &FileSource<'_>,
) -> BTreeMap<String, FileFacts> {
    let ctx = Ctx {
        source,
        real_root: source.disk.canonical(source.project_root),
    };
    let mut files = BTreeMap::new();
    for read in reads {
        if read_problem(read).is_some() || files.contains_key(&read.name) {
            continue;
        }
        let mut budget = TEXT_BUDGET;
        let facts = match (&read.field, &read.glob) {
            (Some(field), _) => field_facts(read, field, entities, &ctx, &mut budget),
            (_, Some(glob)) => glob_facts(read, glob, &ctx, &mut budget),
            _ => continue,
        };
        files.insert(read.name.clone(), facts);
    }
    files
}

fn field_facts(
    read: &FileRead,
    field: &str,
    entities: &EntitySnapshot,
    ctx: &Ctx<'_>,
    budget: &mut usize,
) -> FileFacts {
    let source = ctx.source;
    let (kind, declared) = field.split_once('.').unwrap_or((field, ""));
    let base = match read.base.as_deref() {
        Some("spec") => source.spec_root,
        _ => source.project_root,
    };
    // One fact per (entity, path): the same path named twice by one entity is one file.
    let mut named: BTreeMap<(String, String), Option<PathBuf>> = BTreeMap::new();
    for record in entities.records().iter().filter(|r| r.kind == kind) {
        for written in paths_named(record, declared) {
            let inside = inside(source.project_root, written, base);
            let shown = inside
                .as_ref()
                .map_or_else(|| written.replace('\\', "/"), |rel| slashed(rel));
            named
                .entry((record.id.clone(), shown))
                .or_insert_with(|| inside.map(|rel| source.project_root.join(rel)));
        }
    }
    let truncated = named.len() > MAX_FILES;
    let facts = named
        .into_iter()
        .take(MAX_FILES)
        .map(|((entity, path), at)| fact(ctx, path, Some(entity), at.as_deref(), read.text, budget))
        .collect();
    FileFacts { facts, truncated }
}

fn glob_facts(read: &FileRead, glob: &str, ctx: &Ctx<'_>, budget: &mut usize) -> FileFacts {
    let source = ctx.source;
    let segments: Vec<&str> = glob.split('/').filter(|s| *s != ".").collect();
    let named_here: Vec<&str> = segments.iter().copied().filter(|s| !is_wild(s)).collect();
    // Directories above the first wildcard are walked to directly: only the rest of the pattern decides.
    let literal = segments.iter().take_while(|s| !is_wild(s)).count();
    let literal = literal.min(segments.len().saturating_sub(1));
    let start = segments[..literal].join("/");
    let bounded = !segments.contains(&"**");
    let plan = Walk {
        glob,
        segments: segments.len(),
        bounded,
        named: &named_here,
    };
    let mut found = Found::default();
    let first = source.project_root.join(&start);
    // A literal prefix that is itself a link out of the project is not entered.
    if !ctx.escapes(&first) {
        let real = source.disk.canonical(&first);
        walk(ctx, &start, real.as_deref(), &plan, &mut found);
    }
    let Found {
        mut files,
        truncated,
        ..
    } = found;
    files.sort();
    files.dedup();
    let facts = files
        .into_iter()
        .map(|path| {
            let at = source.project_root.join(&path);
            fact(ctx, path, None, Some(&at), read.text, budget)
        })
        .collect();
    FileFacts { facts, truncated }
}

struct Walk<'a> {
    glob: &'a str,
    /// The pattern's segments: with no `**`, nothing deeper can match.
    segments: usize,
    bounded: bool,
    /// The pattern's literal segments, which name a directory in [`SKIPPED`] on purpose.
    named: &'a [&'a str],
}

/// What a walk has found and spent.
#[derive(Default)]
struct Found {
    files: Vec<String>,
    visits: usize,
    truncated: bool,
}

/// The files under `dir` (project-relative, `/` separated; empty is the root; `real` is `dir` with its links
/// resolved, when the disk has links) whose path matches the glob, in order. A directory reached through a link is
/// not entered: a link into an ancestor would loop, and one out of the project would leave it.
fn walk(ctx: &Ctx<'_>, dir: &str, real: Option<&Path>, plan: &Walk<'_>, found: &mut Found) {
    let source = ctx.source;
    let at = if dir.is_empty() {
        source.project_root.to_path_buf()
    } else {
        source.project_root.join(dir)
    };
    let Some(names) = source.disk.list(&at) else {
        return;
    };
    for name in names {
        found.visits += 1;
        if found.truncated || found.visits > MAX_VISITS {
            found.truncated = true;
            return;
        }
        if SKIPPED.contains(&name.as_str()) && !plan.named.contains(&name.as_str()) {
            continue;
        }
        let rel = if dir.is_empty() {
            name.clone()
        } else {
            format!("{dir}/{name}")
        };
        let child = source.project_root.join(&rel);
        if source.disk.is_dir(&child) {
            let depth = rel.matches('/').count() + 1;
            if plan.bounded && depth >= plan.segments {
                continue;
            }
            let child_real = source.disk.canonical(&child);
            let linked = match (real, &child_real) {
                (Some(parent), Some(child)) => child != &parent.join(&name),
                _ => false,
            };
            if !linked {
                walk(ctx, &rel, child_real.as_deref(), plan, found);
            }
        } else if specforge_common::path_glob::matches(plan.glob, &rel) {
            if found.files.len() == MAX_FILES {
                found.truncated = true;
            } else {
                found.files.push(rel);
            }
        }
    }
}

/// The facts of the file at `at` (`None`: the path is outside the project and is not read).
fn fact(
    ctx: &Ctx<'_>,
    path: String,
    entity: Option<String>,
    at: Option<&Path>,
    text: bool,
    budget: &mut usize,
) -> FileFact {
    let mut fact = FileFact {
        path,
        entity,
        ..FileFact::default()
    };
    let source = ctx.source;
    let Some(at) = at else {
        fact.error = Some("outside_project".into());
        return fact;
    };
    fact.exists = source.disk.exists(at);
    if !fact.exists {
        return fact;
    }
    if ctx.escapes(at) {
        fact.exists = false;
        fact.error = Some("outside_project".into());
        return fact;
    }
    // A text read never carries a secret-looking file, by its name or the name a link resolves to. Its existence is
    // still a fact and still a check input.
    if text
        && (withheld(&fact.path)
            || source
                .disk
                .canonical(at)
                .is_some_and(|real| withheld(&slashed(&real))))
    {
        fact.error = Some("denied".into());
        return fact;
    }
    match source.disk.read(at) {
        Ok(bytes) => {
            fact.bytes = Some(bytes.len() as u64);
            let newlines = bytes.iter().filter(|b| **b == b'\n').count() as u64;
            let open_line = !bytes.is_empty() && bytes.last() != Some(&b'\n');
            fact.lines = Some(newlines + u64::from(open_line));
            fact.digest = Some(file_digest(&bytes));
            if text {
                match String::from_utf8(bytes) {
                    _ if fact.bytes > Some(TEXT_CAP as u64) => {
                        fact.error = Some("too_large".into())
                    }
                    Ok(body) if body.len() > *budget => fact.error = Some("budget".into()),
                    Ok(body) => {
                        *budget -= body.len();
                        fact.text = Some(body);
                    }
                    Err(_) => fact.error = Some("not_utf8".into()),
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fact.exists = false,
        Err(error) if error.kind() == std::io::ErrorKind::FileTooLarge => {
            fact.error = Some("too_large".into());
        }
        Err(_) => fact.error = Some("unreadable".into()),
    }
    fact
}

/// `joined` relative to `root`, lexically normalised, when it stays inside: an absolute path (even one under the
/// root) or one that climbs out with `..` is outside. A link is judged where the file is read ([`Ctx::escapes`]).
fn inside(root: &Path, written: &str, base: &Path) -> Option<PathBuf> {
    if Path::new(written).has_root() || Path::new(written).is_absolute() {
        return None;
    }
    let joined = base.join(written);
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.strip_prefix(root)
        .ok()
        .map(Path::to_path_buf)
        .filter(|rel| !rel.as_os_str().is_empty())
}

/// Whether the file at `path` looks like it holds a credential, which a read for text does not hand to a guest:
/// `.env` and `.env.*`, `*.pem`, `*.key`, `id_rsa*`, `id_ed25519*`, `.npmrc`, `.netrc`, `credentials*` and
/// `.git/config`. shortcut: names only, and no project can opt a file in; upgrade when a project needs to.
fn withheld(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    let name = path.rsplit('/').next().unwrap_or_default();
    name == ".env"
        || name.starts_with(".env.")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
        || name == ".npmrc"
        || name == ".netrc"
        || name.starts_with("credentials")
        || path == ".git/config"
        || path.ends_with("/.git/config")
}

fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_wild(segment: &str) -> bool {
    segment.contains(['*', '?'])
}
