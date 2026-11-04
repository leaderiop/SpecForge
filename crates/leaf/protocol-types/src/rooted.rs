//! The `<root>/` prefix inside a string value (ADR 0336 D6): the one parser every path reader uses, on the host and in
//! a guest. It reads syntax only: where a path may lead (`..`, absolute, links, case) is the host's `Contained`.

use std::fmt;
use std::ops::Range;

/// A root a path in a value starts at: the spec root, the project root, or a root `specforge.json` names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Root {
    Spec,
    Project,
    Named(String),
}

impl Root {
    /// The root `name` names, `None` when it is not `[a-z][a-z0-9_-]*`.
    pub fn parse(name: &str) -> Option<Root> {
        let mut chars = name.chars();
        let word = chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        word.then(|| match name {
            "spec" => Root::Spec,
            "project" => Root::Project,
            other => Root::Named(other.to_string()),
        })
    }

    pub fn as_str(&self) -> &str {
        match self {
            Root::Spec => "spec",
            Root::Project => "project",
            Root::Named(name) => name,
        }
    }
}

impl fmt::Display for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A value's path as written: `"<root>/rest"` names its root, a bare path takes the reader's default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootedPath {
    pub root: Root,
    /// The text after the prefix, exactly as written; empty for `<root>` alone, which names the root itself.
    pub path: String,
    /// The bytes of the written root's name, without its `<` and `>`; `None` for a bare path.
    pub written_root: Option<Range<usize>>,
}

/// Why a value's text cannot be read as a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrefixProblem {
    /// It starts with `<` and no `>` closes the root, or text other than `/` follows the `>`.
    UnclosedRoot,
    /// What the `<` and `>` hold (at these bytes) is no root's name.
    BadRootName { at: Range<usize> },
    /// A `\` at this byte: reported, never rewritten.
    Backslash { at: usize },
}

impl RootedPath {
    /// Read `text`: a text that starts with `<` must be `<name>` or `<name>/rest`; any other names `default`.
    pub fn parse(text: &str, default: Root) -> Result<RootedPath, PrefixProblem> {
        if let Some(at) = text.find('\\') {
            return Err(PrefixProblem::Backslash { at });
        }
        let Some(inner) = text.strip_prefix('<') else {
            return Ok(RootedPath {
                root: default,
                path: text.to_string(),
                written_root: None,
            });
        };
        let (name, after) = inner.split_once('>').ok_or(PrefixProblem::UnclosedRoot)?;
        let path = match after {
            "" => "",
            _ => after.strip_prefix('/').ok_or(PrefixProblem::UnclosedRoot)?,
        };
        let at = 1..1 + name.len();
        let root = Root::parse(name).ok_or(PrefixProblem::BadRootName { at: at.clone() })?;
        Ok(RootedPath {
            root,
            path: path.to_string(),
            written_root: Some(at),
        })
    }
}

#[cfg(test)]
mod tests;
