//! Which test items count as compiled out on the host: a `cfg(…)` attribute whose predicate is false here
//! (ADR 0393 D5). Only platform atoms decide; a feature, `test` or any other atom is unknown, so a test a forgotten
//! feature compiled out stays missing instead of being hidden as skipped.
//!
//! The guest learns the host's family from its input: the canonical project root the host passes as `cwd` is a unix
//! path (`/…`) or a Windows one (`C:\…`, `\\?\C:\…`). That decides `unix`, `windows` and `target_family`, and
//! `target_os` on a Windows host. Which unix it is (`target_os = "linux"` or `"macos"`) nothing in the input tells, so
//! there `target_os` is unknown but for `"windows"`: a test gated on another unix's `target_os` stays missing.

/// A predicate's value on the host: `None` when it does not decide.
type Value = Option<bool>;

/// The host's target family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Family {
    Unix,
    Windows,
}

impl Family {
    /// The family whose paths the canonical project root `cwd` is written in; `None` without a root, or for one in
    /// neither form.
    pub(crate) fn of(cwd: Option<&str>) -> Option<Family> {
        let cwd = cwd?;
        let drive = cwd.as_bytes().get(1) == Some(&b':') && cwd.as_bytes()[0].is_ascii_alphabetic();
        if cwd.starts_with('/') {
            Some(Family::Unix)
        } else if drive || cwd.starts_with(r"\\") {
            Some(Family::Windows)
        } else {
            None
        }
    }

    /// Its `target_family` value.
    fn name(self) -> &'static str {
        match self {
            Family::Unix => "unix",
            Family::Windows => "windows",
        }
    }
}

/// Whether `attr` (an item's recorded attribute, `cfg(windows)`) is a `cfg(…)` whose predicate parses and is false on
/// a host of `family` (`None`: nothing is decided). Anything that does not parse is not excluded.
pub(crate) fn excludes(attr: &str, family: Option<Family>) -> bool {
    let Some(family) = family else {
        return false;
    };
    let mut parser = Parser { rest: attr, family };
    parser.ident() == Some("cfg")
        && matches!(parser.list().as_deref(), Some([Some(false)]))
        && parser.rest.trim().is_empty()
}

/// The value of the atom `name` (`name = "value"` when `value` is given) on a host of `family`: a native host is of
/// one family, and a Windows host's `target_os` is `"windows"`.
fn holds(family: Family, name: &str, value: Option<&str>) -> Value {
    match (name, value) {
        ("unix", None) => Some(family == Family::Unix),
        ("windows", None) => Some(family == Family::Windows),
        ("target_family", Some(value)) => Some(value == family.name()),
        ("target_os", Some(value)) if family == Family::Windows => Some(value == "windows"),
        ("target_os", Some("windows")) => Some(false),
        _ => None,
    }
}

/// `all(…)`: false when a member is, true when every member is, else unknown.
fn all(members: &[Value]) -> Value {
    if members.contains(&Some(false)) {
        Some(false)
    } else if members.iter().all(|v| *v == Some(true)) {
        Some(true)
    } else {
        None
    }
}

/// `any(…)`: true when a member is, false when every member is, else unknown.
fn any(members: &[Value]) -> Value {
    if members.contains(&Some(true)) {
        Some(true)
    } else if members.iter().all(|v| *v == Some(false)) {
        Some(false)
    } else {
        None
    }
}

/// A recursive-descent reader of rustc's cfg predicate grammar, evaluating as it reads.
struct Parser<'a> {
    rest: &'a str,
    family: Family,
}

impl<'a> Parser<'a> {
    fn eat(&mut self, c: char) -> bool {
        self.rest = self.rest.trim_start();
        match self.rest.strip_prefix(c) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    fn ident(&mut self) -> Option<&'a str> {
        self.rest = self.rest.trim_start();
        let end = self
            .rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(self.rest.len());
        let (word, rest) = self.rest.split_at(end);
        self.rest = rest;
        (!word.is_empty()).then_some(word)
    }

    fn string(&mut self) -> Option<&'a str> {
        if !self.eat('"') {
            return None;
        }
        let (text, rest) = self.rest.split_once('"')?;
        self.rest = rest;
        Some(text)
    }

    /// One predicate's value; `None` when it does not parse.
    fn predicate(&mut self) -> Option<Value> {
        let name = self.ident()?;
        if self.eat('=') {
            return Some(holds(self.family, name, Some(self.string()?)));
        }
        Some(match name {
            "all" => all(&self.list()?),
            "any" => any(&self.list()?),
            "not" => match self.list()?[..] {
                [one] => one.map(|v| !v),
                _ => return None,
            },
            _ => holds(self.family, name, None),
        })
    }

    /// A parenthesized, comma-separated list of predicates (a trailing comma allowed), each evaluated.
    fn list(&mut self) -> Option<Vec<Value>> {
        if !self.eat('(') {
            return None;
        }
        let mut members = Vec::new();
        loop {
            if self.eat(')') {
                return Some(members);
            }
            members.push(self.predicate()?);
            if !self.eat(',') {
                return self.eat(')').then_some(members);
            }
        }
    }
}
