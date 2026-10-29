//! What the host read of the files a scan names (K11). A path the host did not serve is never "missing": a
//! `budget`, `too_large`, `denied` or `unreadable` error, a truncated read, or no fact at all means "not checked".

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

/// The fact of one path.
pub enum Served<'a> {
    /// The host looked and the file is not there.
    Absent,
    /// The host did not serve it: nothing can be said.
    Unserved,
    File(File<'a>),
}

/// A file the host read.
#[derive(Clone, Copy)]
pub struct File<'a> {
    pub lines: Option<u64>,
    /// Absent when the file is past the host's text cap or the read asked for facts only.
    pub text: Option<&'a str>,
}

/// The facts of one read, by path.
pub struct Reads<'a> {
    facts: BTreeMap<String, File<'a>>,
    absent: BTreeSet<String>,
    unserved: BTreeSet<String>,
    truncated: bool,
}

/// A path as the host keys it: project-relative and lexically normalised (`./`, `//` and `a/../` removed). A `..` that
/// leaves the project is kept, so it stays a path the host will not serve.
pub fn key(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.trim().split('/') {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|p| *p != "..") => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

pub fn reads<'a>(input: &'a PassInput, name: &str) -> Reads<'a> {
    let mut reads = Reads {
        facts: BTreeMap::new(),
        absent: BTreeSet::new(),
        unserved: BTreeSet::new(),
        truncated: false,
    };
    let Some(files) = input.files.get(name) else {
        return reads;
    };
    reads.truncated = files.truncated;
    for fact in &files.facts {
        let path = key(&fact.path);
        if fact.error.is_some() {
            reads.unserved.insert(path);
        } else if !fact.exists {
            reads.absent.insert(path);
        } else {
            let file = File {
                lines: fact.lines,
                text: fact.text.as_deref(),
            };
            reads.facts.insert(path, file);
        }
    }
    reads
}

impl<'a> Reads<'a> {
    pub fn get(&self, path: &str) -> Served<'a> {
        let path = key(path);
        if let Some(file) = self.facts.get(&path) {
            Served::File(*file)
        } else if self.absent.contains(&path) && !self.truncated {
            Served::Absent
        } else {
            Served::Unserved
        }
    }
}
