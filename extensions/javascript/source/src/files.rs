//! Which source files the pack reads, and their parse by the host's reader.
//!
//! The host lists the project's files of each source extension (one `files` read per extension: a glob has no
//! alternation). A listed file is read when a named package owns it ([`Workspace::owner`]: a file under none is
//! outside the pack, as a `.rs` file no crate reaches is outside `@rust/source`), it is not a test file (a runner pack
//! holds the tests; their helpers are no API) and it is not generated output. The kept files of one grammar are
//! parsed in one request, the files an answer's cap left unparsed asked again. A listing the host truncated is
//! replaced, for its extension, by a parse of its glob whose answer the same rules filter.

use std::collections::BTreeMap;

use specforge_extension_sdk::host::{
    self, NotParsed, SyntaxFile, SyntaxFiles, SyntaxMatch, SyntaxOutcome, SyntaxRequest,
    SyntaxSource,
};
use specforge_extension_sdk::prelude::*;
use specforge_javascript_workspace::Workspace;

use crate::{COLLECTOR, QUERY};

/// The grammar of TypeScript (`.ts .mts .cts`, a `.d.ts` too): its `<T>x` cast parses only there.
pub(crate) const TYPESCRIPT: &str = "typescript";
/// The grammar of TSX, which reads JavaScript and JSX as well (`.tsx .js .jsx .mjs .cjs`).
pub(crate) const TSX: &str = "tsx";

/// One `files` read: the project's files of one extension, listed without text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Listing {
    pub name: &'static str,
    pub extension: &'static str,
    pub language: &'static str,
}

impl Listing {
    /// `**/*.<extension>`.
    pub(crate) fn glob(&self) -> String {
        format!("**/*.{}", self.extension)
    }

    pub(crate) fn input(&self) -> Input {
        Input::files(self.name, FileReadSpec::glob(&self.glob()))
    }
}

/// The listings, one per source extension, with the grammar that reads it.
pub(crate) const LISTINGS: [Listing; 8] = [
    listing("sources_ts", "ts", TYPESCRIPT),
    listing("sources_mts", "mts", TYPESCRIPT),
    listing("sources_cts", "cts", TYPESCRIPT),
    listing("sources_tsx", "tsx", TSX),
    listing("sources_js", "js", TSX),
    listing("sources_jsx", "jsx", TSX),
    listing("sources_mjs", "mjs", TSX),
    listing("sources_cjs", "cjs", TSX),
];

const fn listing(name: &'static str, extension: &'static str, language: &'static str) -> Listing {
    Listing {
        name,
        extension,
        language,
    }
}

/// A test file's name: node `--test`'s, vitest's and jest's default patterns.
fn is_test_name(name: &str) -> bool {
    [".test.", ".spec.", "-test.", "_test."]
        .iter()
        .any(|infix| name.contains(infix))
        || name.starts_with("test-")
        || name.starts_with("test.")
}

/// A directory segment that holds tests, mocks and fixtures of tests.
const TEST_DIRS: [&str; 4] = ["test", "tests", "__tests__", "__mocks__"];

/// A directory segment that holds generated output the host's walk does not already skip (it skips `dist`,
/// `node_modules`, `.next` and `vendor`).
const GENERATED_DIRS: [&str; 8] = [
    "coverage",
    "build",
    "out",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".cache",
    "storybook-static",
];

/// The project-relative `path`'s directory segments and its file name.
fn split(path: &str) -> (Vec<&str>, &str) {
    let mut segments: Vec<&str> = path.split('/').collect();
    let name = segments.pop().unwrap_or_default();
    (segments, name)
}

/// Whether `path` is a test file: by its name, or a directory segment that holds tests.
pub(crate) fn is_test(path: &str) -> bool {
    let (dirs, name) = split(path);
    is_test_name(name) || dirs.iter().any(|dir| TEST_DIRS.contains(dir))
}

/// Whether `path` is generated output: a minified file, or a directory segment that holds a build's output.
pub(crate) fn is_generated(path: &str) -> bool {
    let (dirs, name) = split(path);
    [".min.js", ".min.mjs", ".min.cjs"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
        || dirs.iter().any(|dir| GENERATED_DIRS.contains(dir))
}

/// Whether the pack reads `path`: a named package owns it, and it is neither a test file nor generated output.
pub(crate) fn kept(workspace: &Workspace, path: &str) -> bool {
    workspace.owner(path).is_some() && !is_test(path) && !is_generated(path)
}

/// A parsed file: its project-relative path and the query's matches.
pub(crate) struct Parsed {
    pub path: String,
    pub matches: Vec<SyntaxMatch>,
}

/// Every kept file of the project's listings, parsed, by path.
pub(crate) fn parsed(input: &CollectInput, workspace: &Workspace) -> Result<Vec<Parsed>, String> {
    let mut by_language: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut globbed = Vec::new();
    for listing in &LISTINGS {
        let Some(read) = input.files.get(listing.name) else {
            continue;
        };
        if read.truncated {
            globbed.push(listing);
            continue;
        }
        by_language.entry(listing.language).or_default().extend(
            read.facts
                .iter()
                .filter(|file| file.exists && kept(workspace, &file.path))
                .map(|file| file.path.clone()),
        );
    }
    let mut parsed = Vec::new();
    for (language, mut paths) in by_language {
        paths.sort();
        paths.dedup();
        parsed.extend(all_parsed(language, paths)?);
    }
    for listing in globbed {
        parsed.extend(glob_parsed(listing, workspace)?);
    }
    parsed.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(parsed)
}

/// The host's answer to `files` in `language`, read from the project root with the pack's query.
fn parse(language: &str, files: SyntaxFiles) -> Result<(Vec<SyntaxFile>, bool), String> {
    host::syntax::parse(&SyntaxRequest {
        source: SyntaxSource::Worktree {
            root: "project".to_string(),
        },
        files,
        language: language.to_string(),
        query: QUERY.to_string(),
    })
    .map(|answer| (answer.files, answer.truncated))
    .map_err(|e| format!("the host's source reader refused {COLLECTOR}'s request: {e:?}"))
}

/// What one answered file gives: its matches, nothing (deleted since the listing), or a file to ask again.
enum Answered {
    Parsed(Parsed),
    Skipped,
    Capped(String),
}

/// `file` sorted: a file the reader could not parse otherwise is the collector's error, never an item dropped.
fn answered(file: SyntaxFile) -> Result<Answered, String> {
    match file.outcome {
        SyntaxOutcome::Parsed { matches, .. } => Ok(Answered::Parsed(Parsed {
            path: file.path,
            matches,
        })),
        SyntaxOutcome::NotParsed {
            reason: NotParsed::Capped,
        } => Ok(Answered::Capped(file.path)),
        // Deleted since the listing. A file the fs port withholds is no item lost quietly: it is the error below.
        SyntaxOutcome::NotParsed {
            reason: NotParsed::Missing,
        } => Ok(Answered::Skipped),
        SyntaxOutcome::NotParsed { reason } => Err(format!(
            "{}: the host's reader did not parse it ({}); {COLLECTOR} reads every kept source file or none",
            file.path,
            reason.name()
        )),
    }
}

/// `paths` parsed in `language` until no answer says the cap stopped it: each round asks again for the files the
/// last left unparsed. A file whose matches alone pass the cap is the collector's error.
fn all_parsed(language: &str, mut paths: Vec<String>) -> Result<Vec<Parsed>, String> {
    let mut parsed = Vec::new();
    while !paths.is_empty() {
        let mut again = Vec::new();
        let (files, _) = parse(language, SyntaxFiles::Paths(paths.clone()))?;
        for file in files {
            match answered(file)? {
                Answered::Parsed(file) => parsed.push(file),
                Answered::Capped(path) => again.push(path),
                Answered::Skipped => {}
            }
        }
        if !again.is_empty() && again.len() == paths.len() {
            return Err(format!(
                "{}: its matches alone pass the host's answer cap",
                again[0]
            ));
        }
        paths = again;
    }
    Ok(parsed)
}

/// The files of `listing`'s glob, parsed by the host, which walks them itself: the kept ones' matches, the capped
/// kept ones asked again by path. A walk that stopped short (`truncated`) is the collector's error: no glob can ask
/// for the files past it.
fn glob_parsed(listing: &Listing, workspace: &Workspace) -> Result<Vec<Parsed>, String> {
    let (files, truncated) = parse(listing.language, SyntaxFiles::Glob(listing.glob()))?;
    if truncated {
        return Err(format!(
            "{}: the project holds more files than the host walks; {COLLECTOR} reads every kept source file or \
             none",
            listing.glob()
        ));
    }
    let mut parsed = Vec::new();
    let mut again = Vec::new();
    for file in files {
        // A file the pack does not read is never its error: a minified bundle past the deadline included.
        if !kept(workspace, &file.path) {
            continue;
        }
        match answered(file)? {
            Answered::Parsed(file) => parsed.push(file),
            Answered::Capped(path) => again.push(path),
            Answered::Skipped => {}
        }
    }
    parsed.extend(all_parsed(listing.language, again)?);
    Ok(parsed)
}
