//! A test file's test calls as definitions (ADR 0393 D4).
//!
//! The matches of one call merge into one [`Call`] (`queries/js-tests.scm` matches a call once per pattern that names
//! it). A call is counted when its name is static and not empty (its first argument a string, or a template with no
//! substitution: node names an empty one `<anonymous>` or after its function) and it is a `describe` or `suite`, a
//! `test` or `it` with `skip`, `todo` or `only` or a function argument, or a subtest `<name>.test(…)` with a function
//! argument inside a test. It is minted when it and every call around it are counted and no call around it is skipped:
//! a call with a dynamic name (`.each`, `.concurrent` included) mints nothing, nor anything inside it, and raises
//! nothing; node never runs a skipped call's function, so nothing inside it is a test. A minted call is a `function`
//! item with the attribute `test`, its path the names of the calls around it, outermost first, then its own.
//!
//! node's JUnit report writes a test or a suite as a `<testsuite>` exactly when a test, suite or subtest ran inside it,
//! and as a `<testcase>` otherwise: a `describe` with nothing inside, one with no function, and any `.skip` call are
//! test cases. So a minted call carries the test id node-test mints from the same path ([`TestId::js`]) when it is
//! called with `.skip`, or when no runner call sits inside it, minted or not (a subtest with a dynamic name, a
//! function reference or only options still makes its parent a `<testsuite>`). A call whose options may skip it
//! (`{ skip: … }`) and that holds runner calls carries none: whether node reports it or what it holds is decided when
//! it runs. Shortcut: a subtest a helper registers out of the call's span (`await check(t)`) is not seen, so that test
//! keeps an id node reports as a suite; upgrade if X2 names such a test missing in a real project.

use specforge_code_facts::{location, Definition, ItemId, ItemKind, Names, TestId, Visibility};
use specforge_extension_sdk::host::{SyntaxMatch, SyntaxRole, SyntaxSpan};
use specforge_javascript_workspace::Package;

use crate::items::decoded;

/// What a call is, as the query's `@definition.<kind>` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Test,
    Suite,
    Subtest,
}

impl Role {
    fn of(kind: &str) -> Option<Role> {
        Some(match kind {
            "test" => Role::Test,
            "suite" => Role::Suite,
            "subtest" => Role::Subtest,
            _ => return None,
        })
    }
}

/// One call: every match of its span, merged.
struct Call {
    span: SyntaxSpan,
    role: Role,
    /// Its first argument's value, when that is a string or a template.
    literal: Option<String>,
    /// Its template has a substitution.
    dynamic: bool,
    modifier: bool,
    /// Called with `.skip`: node never runs its function.
    skip: bool,
    /// An options argument has a `skip` key: node may skip it.
    skip_option: bool,
    callback: bool,
    /// The index of the innermost call whose span holds this one.
    parent: Option<usize>,
}

impl Call {
    fn merge(&mut self, found: &SyntaxMatch) {
        let named = |name: &'static str| found.captures.iter().filter(move |c| c.name == name);
        // A quantified capture may match more than once: the first match naming the call gives its parts.
        if self.literal.is_none() && named("name").next().is_some() {
            self.literal = Some(decoded(&named("part").collect::<Vec<_>>()));
        }
        self.dynamic |= named("dynamic").next().is_some();
        self.modifier |= named("modifier").next().is_some();
        self.skip |= named("modifier").any(|c| c.text == "skip");
        self.skip_option |= named("skip_option").next().is_some();
        self.callback |= named("callback").next().is_some();
    }

    /// Its name when static and not empty.
    fn name(&self) -> Option<&str> {
        self.literal
            .as_deref()
            .filter(|name| !self.dynamic && !name.is_empty())
    }
}

/// The test calls of a file as merged calls, in source order (the reader sorts matches by start, then by end
/// descending, so the matches of one call are adjacent and a holder comes before what it holds).
fn calls(matches: &[SyntaxMatch]) -> Vec<Call> {
    let mut calls: Vec<Call> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let found = matches
        .iter()
        .filter(|m| m.role == SyntaxRole::Definition)
        .filter_map(|m| Some((Role::of(&m.kind)?, m)));
    for (role, found) in found {
        let (start, end) = (found.span.start.byte, found.span.end.byte);
        if let Some(last) = calls
            .last_mut()
            .filter(|c| (c.span.start.byte, c.span.end.byte) == (start, end))
        {
            last.merge(found);
            continue;
        }
        while let Some(&top) = open.last() {
            let held = calls[top].span;
            if held.start.byte <= start && end <= held.end.byte {
                break;
            }
            open.pop();
        }
        let mut call = Call {
            span: found.span,
            role,
            literal: None,
            dynamic: false,
            modifier: false,
            skip: false,
            skip_option: false,
            callback: false,
            parent: open.last().copied(),
        };
        call.merge(found);
        open.push(calls.len());
        calls.push(call);
    }
    calls
}

/// Whether the call `at` counts as a test or a suite on its own, whatever is around it.
fn counted(calls: &[Call], at: usize) -> bool {
    let call = &calls[at];
    call.name().is_some()
        && match call.role {
            Role::Suite => true,
            Role::Test => call.modifier || call.callback,
            Role::Subtest => {
                call.callback && call.parent.is_some_and(|p| calls[p].role != Role::Suite)
            }
        }
}

/// Whether the call `at` may register a test node under the call around it when that one runs, minted or not: any
/// test or suite call, and a subtest call inside a test (in a suite's body `<name>.test(…)` is no subtest).
fn registers(calls: &[Call], at: usize) -> bool {
    let call = &calls[at];
    call.parent.is_some_and(|parent| match call.role {
        Role::Test | Role::Suite => true,
        Role::Subtest => calls[parent].role != Role::Suite,
    })
}

/// The test calls of `file` (project-relative; `under` the directory of `package`, which owns it), parsed as
/// `matches` by the test query.
pub(crate) fn read(
    package: &Package,
    file: &str,
    under: &str,
    matches: &[SyntaxMatch],
) -> Vec<Definition> {
    let calls = calls(matches);
    let segment = package.segment();
    // Each call's path when minted; a holder comes before what it holds.
    let mut paths: Vec<Option<Vec<String>>> = Vec::with_capacity(calls.len());
    let mut holds = vec![false; calls.len()];
    let top: Vec<String> = Vec::new();
    for (at, call) in calls.iter().enumerate() {
        let around = match call.parent {
            // node never runs a skipped call's function: nothing inside it is a test.
            Some(parent) if calls[parent].skip || calls[parent].skip_option => None,
            Some(parent) => paths[parent].as_ref(),
            None => Some(&top),
        };
        let path = around
            .zip(call.name())
            .filter(|_| counted(&calls, at))
            .map(|(around, name)| {
                let mut path = around.clone();
                path.push(name.to_string());
                path
            });
        if let Some(parent) = call.parent.filter(|_| registers(&calls, at)) {
            holds[parent] = true;
        }
        paths.push(path);
    }
    calls
        .iter()
        .zip(paths)
        .enumerate()
        .filter_map(|(at, (call, path))| {
            let path = path?;
            let leaf = call.skip || !holds[at];
            Some(Definition {
                id: ItemId::js(&segment, under, &path),
                package: package.id.clone(),
                kind: ItemKind::Function,
                visibility: Visibility::js(false),
                file: file.to_string(),
                at: location(file, call.span.start, call.span.end),
                signature: None,
                doc: None,
                attrs: vec!["test".to_string()],
                names: Names::new(),
                test_id: leaf.then(|| TestId::js(&segment, under, &path)),
            })
        })
        .collect()
}
