//! Which test items count as compiled out on the host: a `cfg(…)` attribute whose predicate is false here
//! (ADR 0393 D5). Only platform atoms decide; a feature, `test` or any other atom is unknown, so a test a forgotten
//! feature compiled out stays missing instead of being hidden as skipped.

/// A predicate's value on the host: `None` when it does not decide.
type Value = Option<bool>;

/// Whether `attr` (an item's recorded attribute, `cfg(windows)`) is a `cfg(…)` whose predicate parses and is false on
/// the host. Anything that does not parse is not excluded.
pub(crate) fn excludes(attr: &str) -> bool {
    let mut parser = Parser { rest: attr };
    parser.ident() == Some("cfg")
        && matches!(parser.list().as_deref(), Some([Some(false)]))
        && parser.rest.trim().is_empty()
}

// shortcut: assumes a unix host (dogfood runs on Linux and macOS); upgrade when it runs on Windows.
/// The value of the atom `name` (`name = "value"` when `value` is given) on the host.
fn holds(name: &str, value: Option<&str>) -> Value {
    match (name, value) {
        ("unix", None) | ("target_family", Some("unix")) => Some(true),
        ("windows", None) | ("target_family", Some("windows")) | ("target_os", Some("windows")) => {
            Some(false)
        }
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
            return Some(holds(name, Some(self.string()?)));
        }
        Some(match name {
            "all" => all(&self.list()?),
            "any" => any(&self.list()?),
            "not" => match self.list()?[..] {
                [one] => one.map(|v| !v),
                _ => return None,
            },
            _ => holds(name, None),
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
