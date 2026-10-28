//! The ledger file: its grammar, its parse and its digest.
//!
//! ```text
//! ledger  = "# specforge-harness 1" LF *line
//! line    = path SP key SP digest LF        ; sorted bytewise, unique
//! key     = field-name / "moved:" path / namespace ":" rest
//! digest  = 64 lowercase hex = sha256(path NUL key NUL value)
//! ```

use std::collections::BTreeSet;

use specforge_package::Sha256;

/// The ledger's file name, at the project root.
pub const LEDGER_FILE: &str = "specforge-harness";

/// The first line of a ledger.
pub const HEADER: &str = "# specforge-harness 1";

/// The key of a move record: `<new path> moved:<old path> <digest>`.
pub const MOVED: &str = "moved:";

/// The prefix of the key of a verdict's line: `<path> verdict:<rule> <digest>` (ADR 0290).
pub const VERDICT_KEY: &str = "verdict:";

/// `sha256(path NUL key NUL value)`: the line that records `value` as `key` of `path`.
pub fn digest(path: &str, key: &str, value: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(path.len() + key.len() + value.len() + 2);
    bytes.extend_from_slice(path.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(key.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(value);
    Sha256::of(&bytes).to_string()
}

/// One ledger line.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Line {
    pub path: String,
    pub key: String,
    pub digest: String,
}

impl Line {
    /// A move record: the entity `old` is now `new`.
    pub fn moved(new: &str, old: &str) -> Line {
        Line {
            path: new.to_string(),
            key: format!("{MOVED}{old}"),
            digest: digest(new, &format!("{MOVED}{old}"), b""),
        }
    }

    /// The path this line records a move from, when it is a move record.
    pub fn moved_from(&self) -> Option<&str> {
        self.key.strip_prefix(MOVED)
    }

    /// Whether the line records a value of the entity: a harness field, or a verdict (`verdict:<rule>`).
    pub fn is_value(&self) -> bool {
        self.is_field() || self.key.starts_with(VERDICT_KEY)
    }

    /// Whether the key names a harness field (not a move record or another namespace's key).
    pub fn is_field(&self) -> bool {
        !self.key.contains(':')
    }

    fn text(&self) -> String {
        format!("{} {} {}\n", self.path, self.key, self.digest)
    }
}

/// The lines of a ledger, sorted bytewise and unique.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger(BTreeSet<Line>);

/// A ledger read from text, and what was wrong with the text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parsed {
    pub ledger: Ledger,
    /// W163: the 1-based line number of each malformed line and why; a bad or missing header is line 1.
    pub malformed: Vec<(usize, String)>,
    /// The text is a ledger the host would write differently: a repeated header, a blank line, a duplicate, an
    /// order that is not bytewise (I203). The rewrite also drops lines, which only the check can see.
    pub uncanonical: bool,
}

impl Ledger {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn lines(&self) -> impl Iterator<Item = &Line> {
        self.0.iter()
    }

    pub fn insert(&mut self, line: Line) {
        self.0.insert(line);
    }

    pub fn remove(&mut self, line: &Line) {
        self.0.remove(line);
    }

    /// Whether a line records `digest` as `key` of `path`.
    pub fn records(&self, path: &str, key: &str, digest: &str) -> bool {
        self.0
            .iter()
            .any(|l| l.path == path && l.key == key && l.digest == digest)
    }

    /// The text of the ledger: the header and the lines, sorted and unique.
    pub fn canonical(&self) -> String {
        let mut text = format!("{HEADER}\n");
        for line in &self.0 {
            text.push_str(&line.text());
        }
        text
    }

    /// Read `text`. A missing file is `parse("")`: an empty ledger, nothing wrong. Anything else starts with the
    /// header, or is read as empty (W163 for line 1), so a ledger of another version cannot silence E083.
    pub fn parse(text: &str) -> Parsed {
        let mut parsed = Parsed::default();
        if text.is_empty() {
            return parsed;
        }
        let mut lines = text.lines().enumerate();
        match lines.next() {
            Some((_, first)) if first == HEADER => {}
            Some((_, first)) => {
                parsed
                    .malformed
                    .push((1, format!("expected '{HEADER}', found '{first}'")));
                return parsed;
            }
            None => return parsed,
        }
        let mut written = Vec::new();
        for (index, line) in lines {
            let number = index + 1;
            if line == HEADER || line.is_empty() {
                parsed.uncanonical = true;
                continue;
            }
            let tokens: Vec<&str> = line.split(' ').collect();
            let [path, key, digest] = tokens.as_slice() else {
                parsed.malformed.push((
                    number,
                    format!("expected 'path key digest', found '{line}'"),
                ));
                continue;
            };
            if Sha256::parse(digest).is_err() || path.is_empty() || key.is_empty() {
                parsed.malformed.push((
                    number,
                    format!("'{digest}' is not a SHA-256 digest of a line"),
                ));
                continue;
            }
            let line = Line {
                path: path.to_string(),
                key: key.to_string(),
                digest: digest.to_string(),
            };
            written.push(line.clone());
            if !parsed.ledger.0.insert(line) {
                parsed.uncanonical = true;
            }
        }
        if written.windows(2).any(|pair| pair[0] > pair[1]) || !text.ends_with('\n') {
            parsed.uncanonical = true;
        }
        parsed
    }
}
