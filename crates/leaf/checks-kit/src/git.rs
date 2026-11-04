//! The syntax of what a git check's value names. A value never reaches git as an option or a revision expression: these
//! parsers refuse everything but a plain name.

use crate::Invalid;

/// An object name: 4 to 64 hexadecimal digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectName(String);

impl ObjectName {
    /// `text` as a name, in lower case.
    pub fn parse(text: &str) -> Result<ObjectName, Invalid> {
        if !(4..=64).contains(&text.len()) {
            return Err(Invalid::new("an object name has 4 to 64 hex digits"));
        }
        if let Some(at) = text.find(|ch: char| !ch.is_ascii_hexdigit()) {
            return Err(Invalid::char_at("this is not a hex digit", text, at));
        }
        Ok(ObjectName(text.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A ref's name, by the rules of `git check-ref-format`.
pub struct RefName;

impl RefName {
    pub fn check(name: &str) -> Result<(), Invalid> {
        let whole = |reason: &str| Err(Invalid::new(reason));
        if name.is_empty() || name == "@" {
            return whole("this names no ref");
        }
        if name.starts_with('-') {
            return Err(Invalid::at("a ref does not start with `-`", 0..1));
        }
        if name.starts_with('/') || name.ends_with('/') || name.contains("//") {
            return whole("a ref's `/` separates non-empty parts");
        }
        if name.ends_with('.') {
            return Err(Invalid::at(
                "a ref does not end with `.`",
                name.len() - 1..name.len(),
            ));
        }
        for needle in ["..", "@{"] {
            if let Some(at) = name.find(needle) {
                return Err(Invalid::at(
                    format!("a ref holds no `{needle}`"),
                    at..at + 2,
                ));
            }
        }
        if let Some((at, ch)) = name.char_indices().find(|(_, ch)| {
            ch.is_ascii_control() || matches!(ch, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
        }) {
            return Err(Invalid::at(
                format!("a ref holds no {ch:?}"),
                at..at + ch.len_utf8(),
            ));
        }
        let mut at = 0;
        for part in name.split('/') {
            if part.starts_with('.') {
                return Err(Invalid::at(
                    "a ref's part does not start with `.`",
                    at..at + 1,
                ));
            }
            if part.ends_with(".lock") {
                return Err(Invalid::at(
                    "a ref's part does not end with `.lock`",
                    at..at + part.len(),
                ));
            }
            at += part.len() + 1;
        }
        Ok(())
    }
}

/// `rev:path`: a file at a revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAt<'a> {
    pub rev: &'a str,
    /// From the repository's top.
    pub path: &'a str,
}

impl<'a> FileAt<'a> {
    pub fn parse(text: &'a str) -> Result<FileAt<'a>, Invalid> {
        let Some((rev, path)) = text.split_once(':') else {
            return Err(Invalid::new("this reads `<revision>:<path>`"));
        };
        if ObjectName::parse(rev).is_err() {
            RefName::check(rev).map_err(|invalid| Invalid {
                at: Some(0..rev.len()),
                ..invalid
            })?;
        }
        let from = rev.len() + 1;
        if path.is_empty() || path.starts_with('/') {
            return Err(Invalid::at(
                "the path is from the repository's top",
                from..text.len(),
            ));
        }
        if path.split('/').any(|part| part == ".." || part == ".") {
            return Err(Invalid::at(
                "the path holds no `.` or `..`",
                from..text.len(),
            ));
        }
        Ok(FileAt { rev, path })
    }
}
