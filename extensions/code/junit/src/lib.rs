//! `specforge-code-junit`: the JUnit reader every runner pack links (ADR 0329 D7).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so `@javascript/node-test` (and the vitest and `@rust/test` packs after it) read their report
//! here; `specforge-code-facts` names each test and mints its facts (ADR 0347 D1).
//!
//! - [`cases`] reads a report's `<testcase>`s, each with the `<testsuite>`s around it and the [`names`] its name
//!   tags.
//! - [`names`] reads the `[<ns>:<id>]` tags in a test's name: the entities the test says it proves.
//! - [`relative`] makes a report's absolute file path project-relative, whatever spelling the host's root has.

#[cfg(test)]
mod tests;
mod xml;

use specforge_extension_sdk::prelude::TestStatus;

use xml::{events, Event};

/// One `<testcase>`, with the `<testsuite>`s it is nested in.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    /// The names of the enclosing `<testsuite>`s, outermost first; a test directly under `<testsuites>` has none.
    pub suites: Vec<String>,
    pub name: String,
    pub classname: String,
    /// The `file` attribute as the report wrote it; a pack makes it project-relative with [`relative`].
    pub file: Option<String>,
    pub seconds: Option<f64>,
    /// `<failure>` or `<error>`: failed; `<skipped>` (a `todo` too): skipped; neither: passed.
    pub status: TestStatus,
    /// The ids its name tags, [`names`] of `name`.
    pub names: Vec<String>,
}

impl Case {
    /// Its test path: the suites it is nested in, outermost first, then its name. What a runner pack names it by.
    pub fn path(&self) -> Vec<String> {
        self.suites.iter().chain([&self.name]).cloned().collect()
    }
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
                let name = attr(&attrs, "name");
                let case = Case {
                    suites: suites.clone(),
                    names: names(&name),
                    name,
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

/// The entity ids the `[<ns>:<id>]` tags in `test_name` name, in order of first appearance, each once: the test says
/// it proves them. `<ns>` is an ASCII lower-case letter, then lower-case letters, digits, `-` or `_`; it marks the
/// text as a tag and is not read further, since entity ids are not namespaced (`[gsd:T1]` and `[req:T1]` both name
/// `T1`). `<id>` is the text up to the next `]`, trimmed: non-empty, with no whitespace, `:`, `[` or `]`. Anything
/// else is prose, not a tag: `[1]`, `[TR1]`, `[GSD:x]`, `[gsd: a b]`, `[gsd:dispatch phase="03"]`, an unclosed `[`.
pub fn names(test_name: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut rest = test_name;
    while let Some(open) = rest.find('[') {
        rest = &rest[open + 1..];
        if let Some(id) = tag(rest) {
            if !found.iter().any(|f| f == id) {
                found.push(id.to_string());
            }
        }
    }
    found
}

/// The id of the tag `text` holds, `text` being what follows a `[`: `<ns>:<id>]…`, by the grammar of [`names`].
fn tag(text: &str) -> Option<&str> {
    let (ns, after) = text.split_once(':')?;
    let mut ns = ns.chars();
    let marked = ns.next().is_some_and(|c| c.is_ascii_lowercase())
        && ns.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    let (id, _) = after.split_once(']')?;
    let id = id.trim();
    let plain =
        !id.is_empty() && !id.contains(|c: char| c.is_whitespace() || matches!(c, ':' | '[' | ']'));
    (marked && plain).then_some(id)
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
/// relative `path` is taken as project-relative already. Either way its `.` and `..` segments are resolved, and a
/// path that climbs out of the project through `..` lies outside it, as an absolute one elsewhere does.
pub fn relative(path: &str, cwd: &str) -> Option<String> {
    let path = normal(path);
    if !is_absolute(&path) {
        return contained(&path);
    }
    let cwd = normal(cwd);
    aliases(&cwd)
        .iter()
        .find_map(|root| {
            aliases(&path)
                .iter()
                .find_map(|spelling| under(spelling, root))
        })
        .and_then(|rest| contained(&rest))
}

/// The project-relative `path` with its `.` and `..` segments resolved, or `None` when a `..` climbs out of the
/// project or nothing is left.
fn contained(path: &str) -> Option<String> {
    let mut kept: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                kept.pop()?;
            }
            name => kept.push(name),
        }
    }
    (!kept.is_empty()).then(|| kept.join("/"))
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
