//! `specforge-code-junit`: the JUnit reader every runner pack links (ADR 0329 D7), and the one function that mints
//! `@code/core`'s test facts from it.
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so `@javascript/node-test` (and the vitest and `@rust/test` packs after it) read their report
//! here, name each test with [`TestId`] and answer [`facts`], and so mint identical ids.
//!
//! - [`cases`] reads a report's `<testcase>`s, each with the `<testsuite>`s around it.
//! - [`relative`] makes a report's absolute file path project-relative, whatever spelling the host's root has.
//! - [`TestId`] names a test `js:<package>/<file>#<s1 > … > name>` (ADR 0329 D2).
//! - [`facts`] mints a `test` and a `test_run` fact per id from the same case, so their statuses agree.
//!
//! gsd-junit keeps its own copy of the reader: gsd may not depend outside `gsd/` (R-9).

#[cfg(test)]
mod tests;
mod xml;

use std::collections::BTreeMap;
use std::fmt;

use specforge_extension_sdk::prelude::{Fact, TestStatus};

use xml::{events, Event};

/// One `<testcase>`, with the `<testsuite>`s it is nested in.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    /// The names of the enclosing `<testsuite>`s, outermost first; a test directly under `<testsuites>` has none.
    pub suites: Vec<String>,
    pub name: String,
    pub classname: String,
    /// The `file` attribute as the report wrote it; a pack makes it project-relative with [`relative`] for its
    /// [`TestId`].
    pub file: Option<String>,
    pub seconds: Option<f64>,
    /// `<failure>` or `<error>`: failed; `<skipped>` (a `todo` too): skipped; neither: passed.
    pub status: TestStatus,
}

/// Every `<testcase>` of the JUnit report `text`, in document order. A report that stops inside a test, or holds no
/// `<testsuite>` or `<testsuites>`, is an error: a failing last test must not vanish. An empty text has no case.
pub fn cases(text: &str) -> Result<Vec<Case>, String> {
    let mut found = Vec::new();
    let mut open: Option<Case> = None;
    let mut suites: Vec<String> = Vec::new();
    let mut rooted = false;
    for event in events(text)? {
        match event {
            Event::Open { name, attrs, empty } if name == "testsuite" => {
                rooted = true;
                if !empty {
                    suites.push(attr(&attrs, "name"));
                }
            }
            Event::Open { name, .. } if name == "testsuites" => rooted = true,
            Event::Open { name, attrs, empty } if name == "testcase" => {
                let file = attr(&attrs, "file");
                let case = Case {
                    suites: suites.clone(),
                    name: attr(&attrs, "name"),
                    classname: attr(&attrs, "classname"),
                    file: (!file.is_empty()).then_some(file),
                    seconds: attr(&attrs, "time")
                        .trim()
                        .parse()
                        .ok()
                        .filter(|s: &f64| s.is_finite() && *s >= 0.0),
                    status: TestStatus::Passed,
                };
                if empty {
                    found.push(case);
                } else {
                    open = Some(case);
                }
            }
            Event::Open { name, .. } => {
                if let Some(case) = open.as_mut() {
                    match name.as_str() {
                        "failure" | "error" => case.status = TestStatus::Failed,
                        "skipped" if case.status == TestStatus::Passed => {
                            case.status = TestStatus::Skipped;
                        }
                        _ => {}
                    }
                }
            }
            Event::Close(name) if name == "testcase" => found.extend(open.take()),
            Event::Close(name) if name == "testsuite" => {
                suites.pop();
            }
            Event::Close(_) => {}
        }
    }
    if open.is_some() {
        return Err("the report stops inside a <testcase>".to_string());
    }
    if !rooted && !text.trim().is_empty() {
        return Err(
            "no <testsuite> or <testsuites> element: this is not a JUnit report".to_string(),
        );
    }
    Ok(found)
}

/// The value of the attribute `key`, or `""`.
fn attr(attrs: &[(String, String)], key: &str) -> String {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// `path` relative to the project root `cwd`, with `/` separators, or `None` when it lies outside it. Both are read
/// the same way first: a `\\?\` (or `\\?\UNC\`) verbatim prefix is stripped, `\` is folded to `/`, a drive letter
/// compares case-insensitively and a trailing `/` is dropped. The host's `cwd` is the canonical root, so the other
/// spelling of macOS's `/private` alias (`/private/var` and `/var`, `/private/tmp` and `/tmp`) is tried too. A
/// relative `path` is taken as project-relative already, without its `./`.
pub fn relative(path: &str, cwd: &str) -> Option<String> {
    let path = normal(path);
    if !is_absolute(&path) {
        return Some(path.trim_start_matches("./").to_string());
    }
    let cwd = normal(cwd);
    aliases(&cwd).iter().find_map(|root| {
        aliases(&path)
            .iter()
            .find_map(|spelling| under(spelling, root))
    })
}

/// `path` with no verbatim prefix, `/` separators and no trailing `/`.
fn normal(path: &str) -> String {
    let path = match path.strip_prefix(r"\\?\UNC\") {
        Some(share) => format!(r"\\{share}"),
        None => path.strip_prefix(r"\\?\").unwrap_or(path).to_string(),
    };
    let path = path.replace('\\', "/");
    match path.trim_end_matches('/') {
        "" if path.starts_with('/') => "/".to_string(),
        trimmed => trimmed.to_string(),
    }
}

/// Whether `path` (read by [`normal`]) is rooted: `/…` or a drive, `C:/…`.
fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || drive(path).is_some()
}

/// The drive letter `path` starts with, `C:`, if any.
fn drive(path: &str) -> Option<char> {
    let mut chars = path.chars();
    let letter = chars.next().filter(char::is_ascii_alphabetic)?;
    (chars.next() == Some(':')).then_some(letter)
}

/// `path`, and its other spelling under macOS's `/private` alias.
fn aliases(path: &str) -> Vec<String> {
    let mut spellings = vec![path.to_string()];
    for (long, short) in [("/private/var", "/var"), ("/private/tmp", "/tmp")] {
        for (from, to) in [(long, short), (short, long)] {
            match path.strip_prefix(from) {
                Some(rest) if rest.is_empty() || rest.starts_with('/') => {
                    spellings.push(format!("{to}{rest}"));
                }
                _ => {}
            }
        }
    }
    spellings
}

/// `path` below `root`, both read by [`normal`]: what follows `root/`, or `None`.
fn under(path: &str, root: &str) -> Option<String> {
    let (path, root) = match (drive(path), drive(root)) {
        (Some(a), Some(b)) if a.eq_ignore_ascii_case(&b) => (&path[2..], &root[2..]),
        (None, None) => (path, root),
        _ => return None,
    };
    let rest = path.strip_prefix(root.trim_end_matches('/'))?;
    let rest = rest.strip_prefix('/')?;
    (!rest.is_empty()).then(|| rest.to_string())
}

/// A test's id (ADR 0329 D2): `js:<package>/<file>#<s1 > … > name>`, the suite path outermost first and a top-level
/// test `#<name>`. An unscoped npm name cannot start with `@`, so a scoped package kept whole stays injective.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TestId {
    package: String,
    file: String,
    path: Vec<String>,
}

impl TestId {
    /// `case` in the JavaScript package `package` (its name, `@scope/` kept), its `file` the path under the
    /// package's directory.
    pub fn js(package: &str, file: &str, case: &Case) -> TestId {
        TestId {
            package: package.to_string(),
            file: file.to_string(),
            path: case.suites.iter().chain([&case.name]).cloned().collect(),
        }
    }

    /// The id of the run of this test: `run:<test id>`.
    pub fn run(&self) -> String {
        format!("run:{self}")
    }
}

impl fmt::Display for TestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "js:{}/{}#{}",
            self.package,
            self.file,
            self.path.join(" > ")
        )
    }
}

/// The `test` and `test_run` facts of `cases`, two per distinct id, by id (ADR 0329 D3):
///
/// - `test` (id the test id, titled by its suite path and name): `package`, `file` (the id's, project-relative with
///   `/`, so the two never disagree), `suite`, `name` and `status`;
/// - `test_run` (`run:<test id>`): `test`, `status` and `duration_ms`, when the report timed it.
///
/// Both statuses come from the same case. Cases sharing an id merge: the worst status wins (failed, then skipped,
/// then passed) and their durations add up.
pub fn facts(cases: impl IntoIterator<Item = (TestId, Case)>) -> Vec<Fact> {
    let mut merged: BTreeMap<TestId, Case> = BTreeMap::new();
    for (id, case) in cases {
        match merged.get_mut(&id) {
            Some(kept) => {
                if rank(case.status) > rank(kept.status) {
                    kept.status = case.status;
                }
                kept.seconds = match (kept.seconds, case.seconds) {
                    (Some(a), Some(b)) => Some(a + b),
                    (a, b) => a.or(b),
                };
            }
            None => {
                merged.insert(id, case);
            }
        }
    }
    merged
        .into_iter()
        .flat_map(|(id, case)| {
            let status = case.status.as_str();
            let test = Fact::new("test", id.to_string())
                .title(id.path.join(" > "))
                .field("package", id.package.as_str())
                .field("file", id.file.as_str())
                .field("suite", case.suites.clone())
                .field("name", case.name.as_str())
                .field("status", status);
            let mut run = Fact::new("test_run", id.run())
                .field("test", id.to_string())
                .field("status", status);
            if let Some(seconds) = case.seconds {
                // shortcut: a fact's number is read as an integer (ADR 0327 D5), so a run under 1 ms is 0.
                run = run.field("duration_ms", (seconds * 1000.0).round() as u64);
            }
            [test, run]
        })
        .collect()
}

/// How bad a status is: a merge keeps the worst.
fn rank(status: TestStatus) -> u8 {
    match status {
        TestStatus::Passed => 0,
        TestStatus::Skipped => 1,
        TestStatus::Failed => 2,
    }
}
