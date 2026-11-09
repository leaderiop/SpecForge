//! Which source files the pack reads, and their parse by the host's reader.
//!
//! The host lists the project's files of each source extension (one `files` read per extension: a glob has no
//! alternation). A listed file is read when a package the pack reads owns it ([`Workspace::owner`]: a file under none
//! is outside the pack, as a `.rs` file no crate reaches is outside `@rust/source`) and its path inside that package is
//! neither a test file (a runner pack holds the tests; their helpers are no API) nor generated output. A package is
//! read unless its `package.json`, inside the package above it, is a test's or generated output's ([`read_packages`]). The kept files of one grammar are
//! parsed in one request, the files an answer's cap left unparsed asked again. A listing the host truncated is
//! replaced, for its extension, by a parse of its glob whose answer the same rules filter.

use std::collections::{BTreeMap, BTreeSet};

use specforge_code_facts::parse_all;
use specforge_extension_sdk::host::{
    NotParsed, SyntaxFile, SyntaxFiles, SyntaxMatch, SyntaxOutcome, SyntaxRequest, SyntaxSource,
};
use specforge_extension_sdk::prelude::*;
use specforge_javascript_workspace::{Package, Workspace};

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

/// A directory segment that holds generated output the host's walk does not already skip (the walk skips its own
/// list of ignored directories: `specforge_effects::fs::SKIPPED`).
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

/// Whether `path` (inside a package) is a test file: by its name, or a directory segment that holds tests.
pub(crate) fn is_test(path: &str) -> bool {
    let (dirs, name) = split(path);
    is_test_name(name) || dirs.iter().any(|dir| TEST_DIRS.contains(dir))
}

/// Whether `path` (inside a package) is generated output: a minified file, or a directory segment that holds a
/// build's output.
pub(crate) fn is_generated(path: &str) -> bool {
    let (dirs, name) = split(path);
    [".min.js", ".min.mjs", ".min.cjs"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
        || dirs.iter().any(|dir| GENERATED_DIRS.contains(dir))
}

/// Whether a path inside a package is read: it is neither a test file nor generated output.
fn readable(under: &str) -> bool {
    !is_test(under) && !is_generated(under)
}

/// Whether the pack reads the project-relative `path`: a package it reads owns it, and the path inside that package is
/// readable. The rules judge the path inside the package, never the directories above it.
pub(crate) fn kept(workspace: &Workspace, path: &str) -> bool {
    workspace
        .owner(path)
        .is_some_and(|(_, under)| readable(under))
}

/// `workspace` without the packages the pack does not read: one whose `package.json`, inside the nearest package the
/// pack reads above it, is not readable (a fixture under `tests/`, a copy under `build/`), and so every package below
/// it; their files fall to that package, which does not read them either. A package with none above it is read.
pub(crate) fn read_packages(mut workspace: Workspace) -> Workspace {
    let depth = |dir: &str| dir.split('/').filter(|s| !s.is_empty()).count();
    let mut ordered: Vec<&Package> = workspace.packages().iter().collect();
    ordered.sort_by_key(|p| (depth(&p.dir), p.dir.as_str()));
    let mut read: Vec<&Package> = Vec::new();
    for package in ordered {
        let manifest = match package.dir.as_str() {
            "" => "package.json".to_string(),
            dir => format!("{dir}/package.json"),
        };
        let inside = read
            .iter()
            .filter_map(|above| Some((above.dir.len(), above.holds(&manifest)?)))
            .max_by_key(|(len, _)| *len);
        if inside.is_none_or(|(_, under)| readable(under)) {
            read.push(package);
        }
    }
    let read: BTreeSet<String> = read.into_iter().map(|p| p.dir.clone()).collect();
    workspace.retain(|p| read.contains(&p.dir));
    workspace
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
        parsed.extend(all_parsed(
            request(language, SyntaxFiles::Paths(paths)),
            |_| true,
        )?);
    }
    // The host walks a truncated listing's glob itself; a file the rules drop is never the pack's error (a minified
    // bundle past the deadline).
    for listing in globbed {
        parsed.extend(all_parsed(
            request(listing.language, SyntaxFiles::Glob(listing.glob())),
            |path| kept(workspace, path),
        )?);
    }
    parsed.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(parsed)
}

/// `files` in `language`, read from the project root with the pack's query.
fn request(language: &str, files: SyntaxFiles) -> SyntaxRequest {
    SyntaxRequest {
        source: SyntaxSource::Worktree {
            root: "project".to_string(),
        },
        files,
        language: language.to_string(),
        query: QUERY.to_string(),
    }
}

/// `file`'s matches; `None` for a file deleted since the listing. A file the reader could not parse otherwise (one
/// the fs port withholds included) is the collector's error, never an item dropped.
fn answered(file: SyntaxFile) -> Result<Option<Parsed>, String> {
    match file.outcome {
        SyntaxOutcome::Parsed { matches, .. } => Ok(Some(Parsed {
            path: file.path,
            matches,
        })),
        SyntaxOutcome::NotParsed {
            reason: NotParsed::Missing,
        } => Ok(None),
        SyntaxOutcome::NotParsed { reason } => Err(format!(
            "{}: the host's reader did not parse it ({}); {COLLECTOR} reads every kept source file or none",
            file.path,
            reason.name()
        )),
    }
}

/// The files of `request` that `keep` accepts, every one parsed whatever the answer's cap ([`parse_all`]): a glob's
/// walk stopped short is the collector's error, as is a file whose matches alone pass the cap.
fn all_parsed(request: SyntaxRequest, keep: impl Fn(&str) -> bool) -> Result<Vec<Parsed>, String> {
    parse_all(COLLECTOR, request, keep)?
        .into_iter()
        .filter_map(|file| answered(file).transpose())
        .collect()
}
