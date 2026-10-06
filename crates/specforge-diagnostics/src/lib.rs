//! Diagnostic code registry: the single canonical catalog of every code the
//! compiler, the CLI, and the first-party extensions emit.
//!
//! `specforge explain <CODE>` and the MCP `specforge.explain` tool print an
//! entry, MCP diagnostics and the LSP carry its title and docs link, doctor
//! quotes its explanation, and `docs/diagnostics.md` is generated from
//! [`CATALOG`]. The catalog is one table (`catalog.rs`) that generates both
//! [`CATALOG`] and a typed constant for every core code ([`codes`]): the
//! host names a core code as `codes::W112`, a [`Code`] whose level is the
//! catalogued one. The tests at the bottom of this file fail when an emitted
//! code is missing from the catalog, is attributed to the wrong owner, or
//! when the catalog lists a code nothing emits.

#[macro_use]
mod catalog;
mod code;

pub use catalog::{CATALOG, codes};
#[doc(hidden)]
pub use code::prefix_states;
pub use code::{Code, GradedCode};

/// One diagnostic code and what it means.
#[derive(Debug, Clone, Copy)]
pub struct CodeEntry {
    /// `E###` (error), `W###` (warning), `I###` (info), `A###` (an
    /// `analyze` pass finding, whose severity the pass sets), or the
    /// registry client's `R###` / `R-<AREA>-###`.
    pub code: &'static str,
    /// Short human-readable name.
    pub title: &'static str,
    /// `"core"` for the compiler/CLI, or the emitting extension
    /// (`"@specforge/formal"`, ...).
    pub owner: &'static str,
    /// The severity a diagnostic of this code has when it is reported. An
    /// `E`/`W`/`I` prefix states it (the table does not compile otherwise);
    /// an `A###` finding's severity is set by its pass.
    pub level: Level,
    /// What triggers the diagnostic and how to fix it.
    pub explanation: &'static str,
}

/// A catalogued code's severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    Error,
    Warning,
    Info,
    /// `A###` analyze findings: the pass picks the severity per finding.
    SetByPass,
}

impl Level {
    /// How `specforge explain` and `docs/diagnostics.md` name the level.
    pub fn describe(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Info => "info",
            Level::SetByPass => "set by the analyze pass",
        }
    }
}

/// Width used when wrapping explanations for the terminal and the docs page.
pub const WRAP_WIDTH: usize = 80;

/// The page on the canonical repository (the workspace's Cargo
/// `repository`, ADR 0004 D6-b) that documents every catalogued code.
pub const DOCS_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/docs/diagnostics.md"
);

/// The docs link for `code`: its section of [`DOCS_URL`], or `None` when
/// the catalog has no entry for it (a third-party code, or anything
/// outside the catalog).
pub fn docs_href(code: &str) -> Option<String> {
    lookup(code).map(|entry| format!("{DOCS_URL}#{}", entry.code.to_lowercase()))
}

/// Look up a code (case-insensitive).
pub fn lookup(code: &str) -> Option<&'static CodeEntry> {
    let normalized = code.to_uppercase();
    CATALOG
        .binary_search_by(|entry| entry.code.cmp(normalized.as_str()))
        .ok()
        .map(|index| &CATALOG[index])
}

/// Greedy word wrap; never splits a word.
pub fn wrap(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut line_len = 0;
    for word in text.split_whitespace() {
        if line_len > 0 && line_len + 1 + word.len() > width {
            out.push('\n');
            line_len = 0;
        } else if line_len > 0 {
            out.push(' ');
            line_len += 1;
        }
        out.push_str(word);
        line_len += word.len();
    }
    out
}

/// Render `docs/diagnostics.md` from [`CATALOG`] (see the `explain_docs_sync` test).
pub fn render_docs() -> String {
    let mut out = String::from(DOCS_HEADER);
    for entry in CATALOG {
        out.push_str(&format!(
            "\n## {code}\n\n```\n{code}: {title}\n\n{body}\n\nOwner: {owner}\nLevel: {level}\n```\n",
            code = entry.code,
            title = entry.title,
            body = wrap(entry.explanation, WRAP_WIDTH),
            owner = entry.owner,
            level = entry.level.describe(),
        ));
    }
    out.push_str(
        "\n## Retired codes\n\nThese codes are no longer emitted, and are never reused for another \
         meaning.\n\n| Code | Replaced by |\n|------|-------------|\n",
    );
    for (old, new) in RETIRED {
        let new = match new {
            Some(code) => format!("[{code}](#{})", code.to_lowercase()),
            None => "(nothing)".to_string(),
        };
        out.push_str(&format!("| {old} | {new} |\n"));
    }
    out
}

const DOCS_HEADER: &str = "# SpecForge Diagnostic Codes

<!-- Generated file: do not edit by hand. -->

This page is generated from the catalog table in `crates/specforge-diagnostics/src/catalog.rs`,
the single registry of diagnostic codes; `specforge explain <CODE>` prints the
same text. Every code emitted by the compiler, the CLI, or a first-party
extension has exactly one entry, and each entry names its owner: `core` for the
compiler and CLI, or the `@specforge/<name>` extension that emits it. The same
table generates a typed constant for every core code
(`specforge_diagnostics::codes`). A test fails when an emitted code is missing
here, is attributed to the wrong owner, or is listed but never emitted.

Codes follow the pattern `E###` (error), `W###` (warning) and `I###` (info);
`A###` codes are `specforge analyze` findings, whose severity the pass sets.
The registry client keeps its own family, `R###` and `R-<AREA>-###`, whose
prefix doesn't state the severity; no other family is accepted. Each entry's
`Level` is the severity a diagnostic of that code has when it is reported;
`specforge check --strict` raises warnings to errors afterwards. The ranges
`E900`-`E998`, `W900`-`W998` and `I900`-`I998` are reserved for third-party
extensions and never appear in this catalog; `I999` is a core code.

Regenerate this page after editing the catalog table:

```sh
SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync
```
";

/// Codes that are no longer emitted, with the code that replaced them (if
/// any). A retired code is never reused for another meaning.
pub const RETIRED: &[(&str, Option<&str>)] = &[
    ("E017", None),
    ("E018", None),
    ("E020", None),
    ("E023", None),
    ("E029", None),
    ("E035", None),
    ("E037", None),
    ("E038", None),
    ("E047", Some("W139")),
    ("E053", None),
    ("I006", None),
    ("W024", None),
    ("W025", None),
    ("W026", None),
    ("W028", None),
    ("W063", None),
    ("W099", None),
    ("W111", None),
    ("W114", None),
    ("W116", None),
    ("W117", None),
    ("W120", None),
    ("W122", None),
];

/// Look up a retired code (case-insensitive): `Some(replacement)`.
pub fn retired(code: &str) -> Option<Option<&'static str>> {
    let normalized = code.to_uppercase();
    RETIRED
        .iter()
        .find(|(old, _)| *old == normalized)
        .map(|(_, new)| *new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Third-party extensions own E900–E998 / W900–W998 / I900–I998.
    /// The registry client's code areas (`R-RES-005`), catalogued as they
    /// are (ADR 0004 D6-c). The grammar is frozen: no new area, no new family.
    const REGISTRY_AREAS: &[&str] = &["AUTH", "LOGIN", "OPS", "RES", "TRUST"];

    /// `E###`/`W###`/`I###`/`A###`, or the registry client's `R###` and
    /// `R-<AREA>-###`.
    fn is_allowed_shape(code: &str) -> bool {
        let digits = |s: &str| s.len() == 3 && s.bytes().all(|c| c.is_ascii_digit());
        if let Some(rest) = code.strip_prefix("R-") {
            return rest
                .split_once('-')
                .is_some_and(|(area, n)| REGISTRY_AREAS.contains(&area) && digits(n));
        }
        code.len() == 4
            && matches!(code.as_bytes()[0], b'E' | b'W' | b'I' | b'A' | b'R')
            && digits(&code[1..])
    }

    fn is_third_party(code: &str) -> bool {
        let b = code.as_bytes();
        if b.len() != 4 || !matches!(b[0], b'E' | b'W' | b'I') {
            return false;
        }
        let n: u32 = code[1..].parse().unwrap_or(0);
        (900..=998).contains(&n)
    }

    /// C4-10: docs/diagnostics.md is generated from [`CATALOG`] and linked
    /// from LSP `codeDescription.href`. Set `SPECFORGE_BLESS=1` to rewrite it.
    #[test]
    fn explain_docs_sync() {
        let path = workspace_root().join("docs/diagnostics.md");
        let rendered = render_docs();
        if std::env::var("SPECFORGE_BLESS").as_deref() == Ok("1") {
            std::fs::write(&path, &rendered).expect("write docs/diagnostics.md");
            return;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            on_disk == rendered,
            "docs/diagnostics.md is out of date with the explain catalog; regenerate it with \
             `SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync`"
        );
    }

    #[test]
    fn catalog_is_sorted_unique_and_well_formed() {
        let mut problems = Vec::new();
        for pair in CATALOG.windows(2) {
            if pair[0].code >= pair[1].code {
                problems.push(format!(
                    "{} must come before {} (CATALOG is sorted by code, without duplicates)",
                    pair[1].code, pair[0].code
                ));
            }
        }
        for entry in CATALOG {
            if !is_allowed_shape(entry.code) {
                problems.push(format!(
                    "{}: not of the form E###/W###/I###/A###; R### and R-<AREA>-### (AREA one \
                     of {}) are kept only for the registry client, so a new code takes the \
                     next free E/W/I/A number",
                    entry.code,
                    REGISTRY_AREAS.join(", ")
                ));
            } else if is_third_party(entry.code) {
                problems.push(format!(
                    "{}: the 900-998 range is reserved for third-party extensions",
                    entry.code
                ));
            }
            if entry.title.is_empty() || entry.explanation.is_empty() {
                problems.push(format!("{}: empty title or explanation", entry.code));
            }
            if entry.owner != "core" && !entry.owner.starts_with("@specforge/") {
                problems.push(format!(
                    "{}: owner `{}` must be `core` or `@specforge/<name>`",
                    entry.code, entry.owner
                ));
            }
        }
        for (old, new) in RETIRED {
            if lookup(old).is_some() {
                problems.push(format!("{old} is retired, so it can't be in CATALOG"));
            }
            if new.is_some_and(|code| lookup(code).is_none()) {
                problems.push(format!(
                    "{old} is retired to {new:?}, which isn't in CATALOG"
                ));
            }
        }
        assert!(
            problems.is_empty(),
            "CATALOG problems:\n  {}",
            problems.join("\n  ")
        );
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(lookup("e001").map(|e| e.code), Some("E001"));
        assert!(lookup("E900").is_none());
    }

    /// The table builds each constant and its catalog entry from the same
    /// tokens, so one constant of every shape stands for all of them.
    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "a core code's constant carries its catalogued level and owner"
    )]
    fn code_constants_are_their_catalog_entries() {
        for (code, level) in [
            (codes::W112, Level::Warning),
            (codes::E001, Level::Error),
            (codes::I999, Level::Info),
            (codes::R003, Level::Warning),
            (codes::R_RES_005, Level::Error),
        ] {
            let entry = lookup(code.id()).expect("a constant names a catalogued code");
            assert_eq!(entry.code, code.id());
            assert_eq!(entry.owner, "core", "{code}");
            assert_eq!(entry.level, code.level(), "{code}");
            assert_eq!(code.level(), level, "{code}");
        }
        assert_eq!(codes::R_RES_005.id(), "R-RES-005");
        let graded = lookup(codes::A010.id()).expect("A010 is catalogued");
        assert_eq!(
            (graded.code, graded.owner, graded.level),
            ("A010", "core", Level::SetByPass)
        );
        assert_eq!(
            CATALOG.iter().filter(|e| e.owner == "core").count(),
            113,
            "every core entry has a constant; extensions' entries have none"
        );
    }

    /// One code literal found in production source.
    struct Site {
        code: String,
        owner: String,
        location: String,
        /// The literal is compared against (`d.code == "E059"`, a
        /// `matches!` pattern, a `const` list of codes to look for), so it
        /// doesn't keep a catalog entry alive.
        consumer: bool,
        /// The severity the emit site most likely uses (see [`site_level`]).
        level: Option<Level>,
    }

    /// Lines searched on each side of a Rust emit site for its severity.
    /// A struct literal sets `severity:` a line or two from `code:`, and a
    /// constructor names it on the code's line or the one before; five
    /// lines covers both without reaching the neighbouring emit site.
    const SEVERITY_WINDOW: usize = 5;

    /// Severity tokens on one line of Rust: `Severity::X`, a `::error(` /
    /// `::warning(` / `::info(` constructor, a `"severity": "x"` JSON key,
    /// or an `error[`/`warning[` prefix printed before a code.
    fn severity_tokens(line: &str) -> Vec<Level> {
        let mut found = Vec::new();
        for (needles, level) in [
            (
                &[
                    "Severity::Error",
                    "::error(",
                    "\"error[",
                    "\"severity\": \"error\"",
                ][..],
                Level::Error,
            ),
            (
                &[
                    "Severity::Warning",
                    "::warning(",
                    "\"warning[",
                    "\"severity\": \"warning\"",
                ][..],
                Level::Warning,
            ),
            (
                &["Severity::Info", "::info(", "\"severity\": \"info\""][..],
                Level::Info,
            ),
        ] {
            if needles.iter().any(|n| line.contains(n)) {
                found.push(level);
            }
        }
        found
    }

    /// The severity an emit site at `lines[index]` most likely uses.
    /// JSON rules: the `"severity"` of the same rule object (up to the next
    /// `"code"`). Rust: the nearest severity token within
    /// [`SEVERITY_WINDOW`] lines; `None` when there is none, or when the
    /// nearest tokens disagree.
    fn site_level(lines: &[&str], index: usize, is_json: bool) -> Option<Level> {
        if is_json {
            for (offset, line) in lines[index..].iter().enumerate() {
                if offset > 0 && line.contains("\"code\"") {
                    return None;
                }
                if let Some(rest) = line.trim().strip_prefix("\"severity\": ") {
                    return match rest.trim_end_matches(',') {
                        "\"error\"" => Some(Level::Error),
                        "\"warning\"" => Some(Level::Warning),
                        "\"info\"" => Some(Level::Info),
                        _ => None,
                    };
                }
            }
            return None;
        }
        for distance in 0..=SEVERITY_WINDOW {
            let mut levels = Vec::new();
            if let Some(before) = index.checked_sub(distance) {
                levels.extend(severity_tokens(lines[before]));
            }
            if distance > 0 && index + distance < lines.len() {
                levels.extend(severity_tokens(lines[index + distance]));
            }
            levels.dedup();
            match levels.as_slice() {
                [] => continue,
                [level] => return Some(*level),
                _ => return None,
            }
        }
        None
    }

    /// Replace test-only items (`#[cfg(test)]` / `#[test]` and the item that
    /// follows, brace-matched) with blank lines so line numbers survive.
    fn strip_test_items(src: &str) -> Vec<&str> {
        let lines: Vec<&str> = src.lines().collect();
        let mut out = Vec::with_capacity(lines.len());
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("#[cfg(test)]") || trimmed == "#[test]" {
                let mut j = i + 1;
                let mut depth: i64 = 0;
                let mut started = false;
                while j < lines.len() {
                    for ch in lines[j].chars() {
                        match ch {
                            '{' => {
                                depth += 1;
                                started = true;
                            }
                            '}' => depth -= 1,
                            _ => {}
                        }
                    }
                    if (started && depth <= 0) || (!started && lines[j].trim_end().ends_with(';')) {
                        break;
                    }
                    j += 1;
                }
                let end = j.min(lines.len() - 1);
                out.extend(std::iter::repeat_n("", end - i + 1));
                i = end + 1;
                continue;
            }
            out.push(lines[i]);
            i += 1;
        }
        out
    }

    /// The end of a code-shaped token starting at `b[i]`: one uppercase
    /// letter, optional `-AREA` segments, then three digits (`E001`, `R013`,
    /// `R-RES-005`, `E-PUB-001`), not followed by another word character.
    /// Any such family is found, so an emitter can't hide a code behind a
    /// new shape; the catalog decides which families are allowed.
    fn code_shape_end(b: &[u8], i: usize) -> Option<usize> {
        if !b.get(i)?.is_ascii_uppercase() {
            return None;
        }
        let mut j = i + 1;
        let mut segments = 0;
        while b.get(j) == Some(&b'-') && b.get(j + 1).is_some_and(u8::is_ascii_uppercase) {
            j += 1;
            while b.get(j).is_some_and(u8::is_ascii_uppercase) {
                j += 1;
            }
            segments += 1;
        }
        if segments > 0 {
            if b.get(j) != Some(&b'-') {
                return None;
            }
            j += 1;
        }
        if !(j..j + 3).all(|k| b.get(k).is_some_and(u8::is_ascii_digit)) {
            return None;
        }
        let end = j + 3;
        let word = |c: &u8| c.is_ascii_alphanumeric() || *c == b'_';
        (!b.get(end).is_some_and(word)).then_some(end)
    }

    /// Find diagnostic codes in string literals on a line, each with the
    /// byte range around it: a whole literal (`"E028"`, `"R-RES-005"`), a
    /// code printed in brackets after a severity (`"error[E048]: ..."`), or
    /// a literal that starts with the code (`"W097: ..."`). Single-letter
    /// families are limited to E, W, I, A, R and F, so strings like
    /// `"X509"` aren't codes.
    fn code_literals(line: &str) -> Vec<(&str, std::ops::Range<usize>)> {
        let b = line.as_bytes();
        let mut found = Vec::new();
        for i in 1..b.len() {
            let Some(end) = code_shape_end(b, i) else {
                continue;
            };
            let code = &line[i..end];
            if code.len() == 4 && !matches!(b[i], b'E' | b'W' | b'I' | b'A' | b'R' | b'F') {
                continue;
            }
            let after = b.get(end).copied();
            let quoted = b[i - 1] == b'"' && matches!(after, Some(b'"') | Some(b':'));
            let bracketed = b[i - 1] == b'['
                && after == Some(b']')
                && ["error", "warning", "info"]
                    .iter()
                    .any(|w| line[..i - 1].ends_with(w));
            if quoted || bracketed {
                found.push((code, i - 1..end + 1));
            }
        }
        found
    }

    /// Whether the code literal on `line` at `range` is only compared
    /// against (a consumer) rather than emitted: an operand of `==`/`!=`,
    /// a match or `matches!` pattern (`"E003" | "E025" =>`), or an item of
    /// a `const` array of codes to look for (`[&str; N]`). A value after
    /// `=>`, in a struct field, a call or a `&str` constant is emitted.
    fn is_consumer(line: &str, range: &std::ops::Range<usize>) -> bool {
        let before = line[..range.start].trim_end();
        let after = line[range.end..].trim_start();
        let alternative = |s: &str| s.starts_with('|') && !s.starts_with("||");
        before.ends_with("==")
            || before.ends_with("!=")
            || after.starts_with("==")
            || after.starts_with("!=")
            || (before.ends_with('|') && !before.ends_with("||"))
            || alternative(after)
            || (after.starts_with("=>") && !before.ends_with("=>"))
            || line.contains("matches!(")
            || (line.contains("const ") && line.contains(": [&str"))
    }

    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        const SKIP: &[&str] = &[
            "tests",
            "target",
            "node_modules",
            ".git",
            "benches",
            "examples",
        ];
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !SKIP.contains(&name.as_str()) {
                    walk(&path, files);
                }
            } else {
                files.push(path);
            }
        }
    }

    /// The catalog itself: the table (every code) and this file (the
    /// retired codes), so the scanner skips them.
    const CATALOG_FILES: &[&str] = &[
        "crates/specforge-diagnostics/src/catalog.rs",
        "crates/specforge-diagnostics/src/lib.rs",
    ];

    /// Every code literal in production (non-test) source, with the owner
    /// implied by where it lives.
    fn emitted_sites() -> Vec<Site> {
        let root = workspace_root();
        let mut files = Vec::new();
        for top in ["crates", "xtask", "integrations", "extensions"] {
            walk(&root.join(top), &mut files);
        }
        let mut sites = Vec::new();
        // A stale CATALOG_FILES would scan the catalog as an emitter, and
        // every entry would look emitted: each skip must match one file.
        let mut catalog_skipped = Vec::new();
        for path in files {
            let rel = path.strip_prefix(&root).unwrap_or(&path);
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let parts: Vec<&str> = rel_str.split('/').collect();
            let file_name = *parts.last().unwrap_or(&"");
            let is_rust = file_name.ends_with(".rs");
            let owner = if parts[0] == "extensions" {
                if parts.len() < 3 || parts[2] != "src" {
                    continue;
                }
                if !is_rust {
                    continue;
                }
                format!("@specforge/{}", parts[1])
            } else {
                if !is_rust || !parts.contains(&"src") {
                    continue;
                }
                // The coverage rule is a shared crate owned by the testing
                // extension (ADR 0004, D2-f); its codes are that extension's.
                if rel_str.starts_with("crates/specforge-coverage/") {
                    "@specforge/testing".to_string()
                } else {
                    "core".to_string()
                }
            };
            if CATALOG_FILES.contains(&rel_str.as_str()) {
                catalog_skipped.push(rel_str);
                continue;
            }
            if file_name == "tests.rs" || file_name.ends_with("_tests.rs") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            let lines = strip_test_items(&src);
            for (index, line) in lines.iter().enumerate() {
                for (code, range) in code_literals(line) {
                    if is_third_party(code) {
                        continue;
                    }
                    sites.push(Site {
                        code: code.to_string(),
                        owner: owner.clone(),
                        location: format!("{rel_str}:{}", index + 1),
                        consumer: is_consumer(line, &range),
                        level: site_level(&lines, index, !is_rust),
                    });
                }
            }
        }
        catalog_skipped.sort();
        assert_eq!(
            catalog_skipped, CATALOG_FILES,
            "the scanner must skip exactly the catalog files"
        );
        sites
    }

    /// Every code-shaped word in `text` (see [`code_shape_end`]; single-letter
    /// families E, W, I, A, R and F), with its line number.
    fn cited_codes(text: &str) -> Vec<(usize, &str)> {
        let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';
        let mut found = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let b = line.as_bytes();
            for i in 0..b.len() {
                if i > 0 && word(b[i - 1]) {
                    continue;
                }
                let Some(end) = code_shape_end(b, i) else {
                    continue;
                };
                let single = end - i == 4;
                if !single || matches!(b[i], b'E' | b'W' | b'I' | b'A' | b'R' | b'F') {
                    found.push((index + 1, &line[i..end]));
                }
            }
        }
        found
    }

    /// C3: hand-written docs cite only catalogued codes. `docs/diagnostics.md`
    /// is generated from the catalog, and ADRs are dated records that quote
    /// the codes of their time; the third-party ranges are never catalogued.
    #[test]
    fn hand_written_docs_cite_only_catalogued_codes() {
        let docs = workspace_root().join("docs");
        let mut files = Vec::new();
        walk(&docs, &mut files);
        files.sort();
        let mut problems = Vec::new();
        for path in files {
            let rel = path.strip_prefix(&docs).unwrap_or(&path);
            let rel = rel.to_string_lossy().replace('\\', "/");
            if !rel.ends_with(".md") || rel == "diagnostics.md" || rel.starts_with("adr/") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for (line, code) in cited_codes(&text) {
                if lookup(code).is_none() && !is_third_party(code) {
                    problems.push(format!("docs/{rel}:{line} cites {code}"));
                }
            }
        }
        assert!(
            problems.is_empty(),
            "docs cite codes the catalog doesn't have; use the catalogued code (see \
             docs/diagnostics.md) or drop the citation:\n  {}",
            problems.join("\n  ")
        );
    }

    /// C2: a code that is only compared against isn't emitted, so a catalog
    /// entry can't outlive its last emitter through a consumer.
    #[test]
    fn consumer_references_do_not_count_as_emitting() {
        let consumers = [
            r#"        Err(e) if e.code == "E059" => err_invalid("#,
            r#"            if d.code != "E001" {"#,
            r#"        if !matches!(diag.code.as_str(), "E003" | "E025") {"#,
            r#"        if matches!(diag.code.as_str(), "E003") {"#,
            r#"            "E003" | "E025" => fix(diag),"#,
            r#"pub const CONFLICT_CODES: [&str; 2] = ["E017", "W018"];"#,
        ];
        for line in consumers {
            for (code, range) in code_literals(line) {
                assert!(
                    is_consumer(line, &range),
                    "{code} in `{line}` is a consumer"
                );
            }
        }
        let emitters = [
            r#"            Diagnostic::warning("W139", format!("claim"))"#,
            r#"                code: "E028".to_string(),"#,
            r#"      "code": "W041","#,
            r#"const BUDGET_TOO_SMALL: &str = "E062";"#,
            r#"            Kind::Missing => "E025","#,
            r#"        fail("E034", "bad")"#,
        ];
        for line in emitters {
            for (code, range) in code_literals(line) {
                assert!(!is_consumer(line, &range), "{code} in `{line}` is emitted");
            }
        }
    }

    /// The catalog is enforced: every emitted code is registered under the
    /// right owner, and every registered code is emitted somewhere.
    #[specforge_test(
        invariant = "diagnostic_code_uniqueness",
        verify = "Diagnostic Code Uniqueness guarantee holds"
    )]
    fn explain_catalog_matches_emitted_codes() {
        let catalog: BTreeMap<&str, &CodeEntry> = CATALOG.iter().map(|e| (e.code, e)).collect();
        let sites = emitted_sites();
        assert!(
            !sites.is_empty(),
            "scanner found no diagnostic codes; is the workspace root right?"
        );

        let mut problems = Vec::new();
        let mut emitted = BTreeSet::new();
        for site in &sites {
            if site.consumer {
                // A consumer may test for another owner's code, but the code
                // it looks for must exist.
                if !catalog.contains_key(site.code.as_str()) {
                    problems.push(format!(
                        "{} compared against at {} is not in the CATALOG; nothing emits it",
                        site.code, site.location
                    ));
                }
                continue;
            }
            emitted.insert(site.code.as_str());
            match catalog.get(site.code.as_str()) {
                None => problems.push(format!(
                    "{} emitted at {} is not in the CATALOG; add a CodeEntry with owner `{}`",
                    site.code, site.location, site.owner
                )),
                Some(entry) if entry.owner != site.owner => problems.push(format!(
                    "{} emitted at {} belongs to `{}`, but CATALOG says owner `{}`; each code has one \
                     owner, so pick a free code for this emitter or fix the entry",
                    site.code, site.location, site.owner, entry.owner
                )),
                Some(_) => {}
            }
        }
        for code in catalog.keys() {
            if !emitted.contains(code) {
                problems.push(format!(
                    "{code} is in CATALOG but nothing emits it any more; remove the entry"
                ));
            }
        }
        assert!(
            problems.is_empty(),
            "diagnostic catalog out of sync with emitters ({} problems):\n  {}\n\nThen regenerate \
             docs with `SPECFORGE_BLESS=1 cargo test -p specforge-diagnostics explain_docs_sync`.",
            problems.len(),
            problems.join("\n  ")
        );
    }

    /// C4: no emit site uses a severity its catalog entry contradicts. The
    /// site's severity is a heuristic (see [`site_level`]): the rule
    /// object's `"severity"` for JSON rules, else the nearest severity
    /// token within [`SEVERITY_WINDOW`] lines. Sites where it finds none
    /// are skipped; the test also requires most sites to be decided, so the
    /// heuristic can't quietly stop working.
    #[test]
    fn emit_sites_use_the_catalogued_level() {
        let sites = emitted_sites();
        let mut problems = Vec::new();
        let (mut decided, mut total) = (0, 0);
        for site in sites.iter().filter(|s| !s.consumer) {
            let Some(entry) = lookup(&site.code) else {
                continue;
            };
            if entry.level == Level::SetByPass {
                continue;
            }
            total += 1;
            let Some(level) = site.level else {
                continue;
            };
            decided += 1;
            if level != entry.level {
                problems.push(format!(
                    "{} at {} is emitted as {:?}, but the catalog level is {:?}",
                    site.code, site.location, level, entry.level
                ));
            }
        }
        assert!(
            decided * 3 >= total * 2,
            "the severity heuristic decided only {decided} of {total} emit sites"
        );
        assert!(
            problems.is_empty(),
            "emit sites contradict the catalog level; emit the catalogued severity or \
             move the code to the prefix of the severity it has:\n  {}",
            problems.join("\n  ")
        );
    }
}
