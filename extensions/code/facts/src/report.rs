//! The shared pieces of the report facts (ADR 0388): the five kinds a standard report mints, what a report leaves
//! unstated, and the id parts and repeat suffixes every report reader builds an id from.

use std::collections::BTreeMap;

/// A kind a report reader mints, and the scheme its ids start with (`<scheme>:…`). These schemes are not
/// [`Scheme`](crate::Scheme)'s, which names test and item ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportKind {
    pub kind: &'static str,
    pub scheme: &'static str,
}

/// A coverage report's entry for one file (LCOV).
pub const COVERAGE: ReportKind = ReportKind {
    kind: "coverage",
    scheme: "cov",
};
/// One entry of an API snapshot.
pub const API_ITEM: ReportKind = ReportKind {
    kind: "api_item",
    scheme: "api",
};
/// One result of a SARIF run; never `finding`, which `@gsd/core` declares (F1).
pub const STATIC_FINDING: ReportKind = ReportKind {
    kind: "static_finding",
    scheme: "sarif",
};
/// One component of a CycloneDX bill of materials.
pub const DEPENDENCY: ReportKind = ReportKind {
    kind: "dependency",
    scheme: "dep",
};
/// One OSV record.
pub const VULNERABILITY: ReportKind = ReportKind {
    kind: "vulnerability",
    scheme: "vuln",
};

/// The value a field holds when the report does not state it (F5).
pub const UNKNOWN: &str = "unknown";

/// What [`id_part`] percent-encodes besides whitespace and control characters: `%` itself and every id delimiter.
const ID_ESCAPES: &[char] = &['%', '/', '#', ':', '~', '@', '?'];

/// Report text made safe to place in a fact id: `%`, `/`, `#`, `:`, `~`, `@`, `?`, whitespace and control characters
/// percent-encoded as `%XX` (uppercase hex of each UTF-8 byte), every other character kept. Report text can then forge
/// no scheme, separator or repeat suffix, nor carry a terminal escape into a diagnostic naming the id.
pub fn id_part(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if ID_ESCAPES.contains(&c) || c.is_whitespace() || c.is_control() {
            let mut bytes = [0; 4];
            for byte in c.encode_utf8(&mut bytes).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The ids a reader has answered so far, so one answered twice gets a suffix: the host refuses a collect that answers
/// one id twice (F4).
#[derive(Debug, Default)]
pub struct Repeats {
    seen: BTreeMap<String, usize>,
}

impl Repeats {
    /// `base` the first time it is asked, then `base~2`, `base~3`, …, in the order asked (document order).
    pub fn id(&mut self, base: String) -> String {
        let count = self.seen.entry(base.clone()).or_insert(0);
        *count += 1;
        match *count {
            1 => base,
            n => format!("{base}~{n}"),
        }
    }
}
