//! `specforge-code-junit`: the JUnit reader every runner pack links (ADR 0329 D7).
//!
//! An extension library (ADR 0328 D3): an rlib a builtin links, never a component of its own. The wire has no
//! guest-to-guest call, so `@javascript/node-test` (and the vitest and `@rust/test` packs after it) read their report
//! here; `specforge-code-facts` names each test and mints its facts (ADR 0347 D1).
//!
//! - [`cases`] reads a report's `<testcase>`s, each with the `<testsuite>`s around it and the [`names`] its name
//!   tags.
//! - [`names`] reads the `[<ns>:<id>]` tags in a test's name: the entities the test says it proves.

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
    /// The `file` attribute as the report wrote it; a pack makes it project-relative with
    /// `specforge_code_facts::relative`.
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
