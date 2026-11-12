//! The span map of the port (ADR 0332): every `@gsd/core` and `@gsd/security` asset aligned with its upstream copy
//! by a patience diff. Kept spans are upstream's text byte for byte, the quotes; own spans are the rest, each with a
//! group that says why it differs. `tests/port.lock` records every span, and [`check_lock`] recomputes it.
//!
//! It includes nothing: it uses its includer's root modules `command_word`, `files` and `specforge_only`.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use std::collections::{BTreeMap, VecDeque};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use sha2::{Digest, Sha256};
use similar::{Algorithm, DiffOp, capture_diff_slices};

use super::command_word;
use super::specforge_only::SHELL_WORDS;

/// The lock key prefix of `@gsd/security`'s assets, from `gsd/core`.
const SECURITY: &str = "../security/assets/";

/// The upstream names a port replaced: a span whose upstream side names one is `ported-name`.
pub const PORTED: &str = r"\.planning|gsd-tools|gsd_run|gsd-sdk";

/// The closed group vocabulary (ADR 0332 D3), each with its meaning.
pub const GROUPS: &[(&str, &str)] = &[
    // computed, in precedence order
    (
        "ported-name",
        "upstream's side names `.planning`, `gsd-tools`, `gsd_run` or `gsd-sdk`",
    ),
    (
        "served",
        "the span runs `specforge` where upstream's ran a program it no longer needs",
    ),
    (
        "markdown",
        "the span differs only in blank lines, indentation or fence width",
    ),
    // reviewed
    (
        "relic",
        "a guard, branch or read SpecForge never reaches, deleted or reworded (ADR 0343 D4)",
    ),
    ("answer", "logic that moved into a `specforge gsd` answer"),
    ("args", "a typed argument read through `{{arg:}}`"),
    ("files", "a `specforge files` rewrite and its prose"),
    ("git", "a `specforge git` verb that reads differently"),
    ("jq", "JSON read with `--pick`"),
    ("prose", "ADR 0343 D2 and D7–D9"),
    (
        "quick",
        "a quick plan's context, decisions and research are entities",
    ),
    ("review", "a line that told the agent something false"),
    (
        "thread",
        "decisions, items and the `Gsd-Task` trailer (ADR 0351, ADR 0352)",
    ),
    ("wait", "a `specforge wait` call"),
    (
        "workflow",
        "a link that left the tree became the tag's URL, or a false line was fixed",
    ),
    ("agent", "an agent body's key list, probe, write or ratio"),
    // sentinels
    (
        "unreviewed",
        "a bless could not carry the span's group: set one (it always fails)",
    ),
];

/// The groups the test recomputes, in precedence order.
pub const COMPUTED: &[&str] = &["ported-name", "served", "markdown"];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// `gsd/core/tests/port.lock`.
pub fn lock_path() -> PathBuf {
    root().join("tests/port.lock")
}

/// The lock key of every asset, sorted: each `.md` under `gsd/core/assets` as its path there, and the five under
/// `gsd/security/assets/{agents,commands,gsd-core}` as `../security/assets/<path>`.
pub fn assets() -> Vec<String> {
    let mut out = Vec::new();
    for (dir, prefix) in [("assets", ""), ("../security/assets", SECURITY)] {
        let base = root().join(dir);
        for path in super::files::under(&base) {
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let relative = path
                .strip_prefix(&base)
                .expect("under its base")
                .to_string_lossy()
                .replace('\\', "/");
            let upstream = ["agents/", "commands/", "gsd-core/"];
            if prefix.is_empty() || upstream.iter().any(|d| relative.starts_with(d)) {
                out.push(format!("{prefix}{relative}"));
            }
        }
    }
    out.sort();
    out
}

/// The upstream path of the asset `asset`: its lock key without `../security/assets/`.
pub fn upstream_path(asset: &str) -> &str {
    asset.strip_prefix(SECURITY).unwrap_or(asset)
}

/// The body of the upstream file `path` in the vendored snapshot (`<path>`, or `<path>.txt` when its links leave the
/// tree), and the number of upstream lines before it: an agent or a command loses its frontmatter.
pub fn upstream_body(path: &str) -> (Vec<String>, usize) {
    let base = Path::new(gsd_kit::snapshot!());
    let plain = base.join(path);
    let file = if plain.is_file() {
        plain
    } else {
        base.join(format!("{path}.txt"))
    };
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    let offset = if path.starts_with("agents/") || path.starts_with("commands/") {
        assert_eq!(lines[0], "---", "{path} has no frontmatter");
        lines[1..]
            .iter()
            .position(|l| l == "---")
            .expect("a closing ---")
            + 2
    } else {
        0
    };
    lines.drain(..offset);
    (lines, offset)
}

/// The asset `asset` (a lock key) after its attribution header, which must name the tracked release and the upstream
/// path.
pub fn asset_body(asset: &str) -> Vec<String> {
    let file = if asset.starts_with(SECURITY) {
        root().join(asset)
    } else {
        root().join("assets").join(asset)
    };
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let mut lines = text.split('\n').map(String::from);
    let header = lines.next().unwrap_or_default();
    let want = format!(
        "<!-- Ported from gsd-core v{} {}, MIT",
        gsd_kit::tracked_version!(),
        upstream_path(asset)
    );
    assert!(header.starts_with(&want), "{asset}: header {header}");
    lines.collect()
}

/// A span of an asset's body against its upstream body. Both ranges are 0-based body lines; a deletion has an empty
/// `asset` range and an insertion an empty `up` range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    /// Upstream's text, byte for byte.
    Kept {
        asset: Range<usize>,
        up: Range<usize>,
        sha: String,
    },
    /// Text of ours, with the group that says why.
    Own {
        asset: Range<usize>,
        up: Range<usize>,
        sha_up: String,
        sha_own: String,
        group: String,
    },
}

/// The words of `lines`: no blank line, each line trimmed, a leading run of three or more backticks or tildes
/// shortened to three, joined by `\n`. A layout edit keeps a span's words.
pub fn words(lines: &[impl AsRef<str>]) -> String {
    lines
        .iter()
        .map(|l| l.as_ref().trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mark = l.chars().next().filter(|c| *c == '`' || *c == '~');
            let run = mark.map_or(0, |m| l.chars().take_while(|c| *c == m).count());
            match mark {
                Some(m) if run >= 3 => format!("{}{}", m.to_string().repeat(3), &l[run..]),
                _ => l.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first 12 hex digits of the sha256 of `text`.
fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .take(6)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The hash of `lines` joined by `\n`.
fn sha_lines(lines: &[impl AsRef<str>]) -> String {
    sha(&lines
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Whether `theirs` (upstream's lines) stand served by `mine` as a block that calls only `specforge`: mine runs no
/// program theirs does not, besides `specforge` and the shell's own words, and leaves out one that theirs runs.
pub fn served_by_specforge(theirs: &[&str], mine: &[&str]) -> bool {
    let programs = |lines: &[&str]| {
        lines
            .iter()
            .flat_map(|line| command_word::commands(line))
            .filter(|tool| !SHELL_WORDS.contains(&tool.as_str()))
            .collect::<Vec<_>>()
    };
    let (before, after) = (programs(theirs), programs(mine));
    after
        .iter()
        .all(|tool| tool == "specforge" || before.contains(tool))
        && before.iter().any(|tool| !after.contains(tool))
}

/// The first computed group the own span `up` → `own` qualifies for.
pub fn computed(up: &[&str], own: &[&str]) -> Option<&'static str> {
    static PORTED_RX: OnceLock<Regex> = OnceLock::new();
    let ported = PORTED_RX.get_or_init(|| Regex::new(PORTED).expect("a regex"));
    if up.iter().any(|l| ported.is_match(l)) {
        Some("ported-name")
    } else if served_by_specforge(up, own) {
        Some("served")
    } else if words(up) == words(own) {
        Some("markdown")
    } else {
        None
    }
}

/// Whether `line` is blank or a fence mark alone.
fn trivial(line: &str) -> bool {
    line.trim().chars().all(|c| c == '`' || c == '~')
}

/// The tiles of `own` against `up`, as `(kept, up range, own range)`: contiguous changes join into one own tile, and a
/// kept run of only blank and fence-mark lines between two own tiles folds into them.
fn tiles(up: &[&str], own: &[&str]) -> Vec<(bool, Range<usize>, Range<usize>)> {
    let mut out: Vec<(bool, Range<usize>, Range<usize>)> = Vec::new();
    for op in capture_diff_slices(Algorithm::Patience, up, own) {
        let (kept, u, a) = match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => (true, old_index..old_index + len, new_index..new_index + len),
            DiffOp::Delete {
                old_index,
                old_len,
                new_index,
            } => (false, old_index..old_index + old_len, new_index..new_index),
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => (false, old_index..old_index, new_index..new_index + new_len),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => (
                false,
                old_index..old_index + old_len,
                new_index..new_index + new_len,
            ),
        };
        match out.last_mut() {
            Some((false, lu, la)) if !kept => {
                lu.end = u.end;
                la.end = a.end;
            }
            _ => out.push((kept, u, a)),
        }
    }
    let mut folded: Vec<(bool, Range<usize>, Range<usize>)> = Vec::new();
    let mut i = 0;
    while i < out.len() {
        let (kept, _, a) = &out[i];
        let next_own = out.get(i + 1).filter(|(k, _, _)| !k).cloned();
        match (folded.last_mut(), next_own) {
            (Some((false, lu, la)), Some((_, nu, na)))
                if *kept && own[a.clone()].iter().all(|l| trivial(l)) =>
            {
                lu.end = nu.end;
                la.end = na.end;
                i += 2;
            }
            _ => {
                folded.push(out[i].clone());
                i += 1;
            }
        }
    }
    folded
}

/// The spans of `own` against `up`; an own span's group is its computed group, or empty.
pub fn spans(up: &[impl AsRef<str>], own: &[impl AsRef<str>]) -> Vec<Span> {
    let up: Vec<&str> = up.iter().map(AsRef::as_ref).collect();
    let own: Vec<&str> = own.iter().map(AsRef::as_ref).collect();
    tiles(&up, &own)
        .into_iter()
        .map(|(kept, u, a)| {
            if kept {
                Span::Kept {
                    sha: sha_lines(&up[u.clone()]),
                    asset: a,
                    up: u,
                }
            } else {
                let (theirs, mine) = (&up[u.clone()], &own[a.clone()]);
                Span::Own {
                    sha_up: sha_lines(theirs),
                    sha_own: sha(&words(mine)),
                    group: computed(theirs, mine).unwrap_or_default().to_string(),
                    asset: a,
                    up: u,
                }
            }
        })
        .collect()
}

/// A row of the lock. `up` is the 1-based upstream file line (frontmatter included; an insertion's is the line it
/// precedes), `n` the upstream length and `m` the own length. A kept row has `m == n`, the kept text's hash in
/// `sha_up`, and no `sha_own` or group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub asset: String,
    pub kept: bool,
    pub up: usize,
    pub n: usize,
    pub m: usize,
    pub sha_up: String,
    pub sha_own: String,
    pub group: String,
}

impl Row {
    /// The row without its group cell.
    fn key(&self) -> String {
        let mut row = self.clone();
        row.group.clear();
        render_row(&row)
    }
}

/// The rows of the asset `asset`, whose upstream body starts after `offset` upstream lines.
fn rows_of(asset: &str, offset: usize, spans: &[Span]) -> Vec<Row> {
    spans
        .iter()
        .map(|s| match s {
            Span::Kept { up, sha, .. } => Row {
                asset: asset.to_string(),
                kept: true,
                up: offset + up.start + 1,
                n: up.len(),
                m: up.len(),
                sha_up: sha.clone(),
                sha_own: String::new(),
                group: String::new(),
            },
            Span::Own {
                asset: a,
                up,
                sha_up,
                sha_own,
                group,
            } => Row {
                asset: asset.to_string(),
                kept: false,
                up: offset + up.start + 1,
                n: up.len(),
                m: a.len(),
                sha_up: sha_up.clone(),
                sha_own: sha_own.clone(),
                group: group.clone(),
            },
        })
        .collect()
}

/// The lock's first line, up to its format note.
pub fn header() -> String {
    format!(
        "# gsd-core v{} port lock (ADR 0332)",
        gsd_kit::tracked_version!()
    )
}

fn render_row(r: &Row) -> String {
    if r.kept {
        format!("{}\tkept\t{}+{}\t{}", r.asset, r.up, r.n, r.sha_up)
    } else {
        format!(
            "{}\town\t{}+{}\t{}\t{}\t{}\t{}",
            r.asset, r.up, r.n, r.m, r.sha_up, r.sha_own, r.group
        )
    }
}

/// The lock text of `rows`.
pub fn render(rows: &[Row]) -> String {
    let mut out = format!(
        "{}: <asset> kept <u>+<n> <sha> | <asset> own <u>+<n> <m> <sha_up> <sha_own> <group>. \
         GSD_BLESS=1 rewrites it; edit only group cells.\n",
        header()
    );
    for row in rows {
        out.push_str(&render_row(row));
        out.push('\n');
    }
    out
}

/// The rows of the lock text `lock`.
pub fn rows(lock: &str) -> Vec<Row> {
    let number = |s: &str, at: usize| -> usize {
        s.parse()
            .unwrap_or_else(|_| panic!("port.lock:{at}: not a number: {s}"))
    };
    lock.lines()
        .enumerate()
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .map(|(i, l)| {
            let at = i + 1;
            let cells: Vec<&str> = l.split('\t').collect();
            let (u, n) = cells
                .get(2)
                .and_then(|c| c.split_once('+'))
                .unwrap_or_else(|| panic!("port.lock:{at}: no <u>+<n>: {l}"));
            match cells[..] {
                [asset, "kept", _, sha] => Row {
                    asset: asset.to_string(),
                    kept: true,
                    up: number(u, at),
                    n: number(n, at),
                    m: number(n, at),
                    sha_up: sha.to_string(),
                    sha_own: String::new(),
                    group: String::new(),
                },
                [asset, "own", _, m, sha_up, sha_own, group] => Row {
                    asset: asset.to_string(),
                    kept: false,
                    up: number(u, at),
                    n: number(n, at),
                    m: number(m, at),
                    sha_up: sha_up.to_string(),
                    sha_own: sha_own.to_string(),
                    group: group.to_string(),
                },
                _ => panic!("port.lock:{at}: not a kept or own row: {l}"),
            }
        })
        .collect()
}

/// The rows of `gsd/core/tests/port.lock` (none before its first bless).
pub fn lock_rows() -> Vec<Row> {
    rows(&std::fs::read_to_string(lock_path()).unwrap_or_default())
}

/// How a bless writes an own span it cannot carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bless {
    /// `GSD_BLESS=1`: `unreviewed`.
    Review,
}

/// The bless `GSD_BLESS` asks for.
pub fn bless() -> Option<Bless> {
    match std::env::var("GSD_BLESS").as_deref() {
        Ok("1") => Some(Bless::Review),
        _ => None,
    }
}

/// The groups of `new` (rows whose own groups are computed, or empty): a computed group stays, an empty one takes the
/// group of the first unused `old` row with the same asset, upstream hash and words hash, or else what `mode` writes.
pub fn carry(old: &[Row], new: Vec<Row>, mode: Bless) -> Vec<Row> {
    let key = |r: &Row| (r.asset.clone(), r.sha_up.clone(), r.sha_own.clone());
    let mut groups: BTreeMap<(String, String, String), VecDeque<String>> = BTreeMap::new();
    for o in old
        .iter()
        .filter(|o| !o.kept && !COMPUTED.contains(&o.group.as_str()))
    {
        groups.entry(key(o)).or_default().push_back(o.group.clone());
    }
    new.into_iter()
        .map(|mut row| {
            if !row.kept && row.group.is_empty() {
                let found = groups.get_mut(&key(&row)).and_then(VecDeque::pop_front);
                row.group = match (found, mode) {
                    (Some(group), _) => group,
                    (None, Bless::Review) => "unreviewed".to_string(),
                };
            }
            row
        })
        .collect()
}

/// An asset's upstream offset and its spans, own groups computed or empty.
struct Mapped {
    offset: usize,
    spans: Vec<Span>,
}

/// Every asset, mapped once per test binary.
fn mapped() -> &'static BTreeMap<String, Mapped> {
    static MAPPED: OnceLock<BTreeMap<String, Mapped>> = OnceLock::new();
    MAPPED.get_or_init(|| {
        assets()
            .into_iter()
            .map(|asset| {
                let (up, offset) = upstream_body(upstream_path(&asset));
                let spans = spans(&up, &asset_body(&asset));
                (asset, Mapped { offset, spans })
            })
            .collect()
    })
}

/// The rows the assets give today: own groups computed, or empty.
pub fn computed_rows() -> Vec<Row> {
    mapped()
        .iter()
        .flat_map(|(asset, m)| rows_of(asset, m.offset, &m.spans))
        .collect()
}

/// The spans of the asset `path` (a lock key), with their groups: computed, or reviewed in the lock.
pub fn map(path: &str) -> Vec<Span> {
    static GROUPED: OnceLock<BTreeMap<String, Vec<Span>>> = OnceLock::new();
    let grouped = GROUPED.get_or_init(|| {
        let mut groups = carry(&lock_rows(), computed_rows(), Bless::Review).into_iter();
        mapped()
            .iter()
            .map(|(asset, m)| {
                let spans = m
                    .spans
                    .iter()
                    .map(|s| {
                        let group = groups.next().map(|r| r.group).unwrap_or_default();
                        match s.clone() {
                            Span::Own {
                                asset,
                                up,
                                sha_up,
                                sha_own,
                                ..
                            } => Span::Own {
                                asset,
                                up,
                                sha_up,
                                sha_own,
                                group,
                            },
                            kept => kept,
                        }
                    })
                    .collect();
                (asset.clone(), spans)
            })
            .collect()
    });
    grouped
        .get(path)
        .cloned()
        .unwrap_or_else(|| panic!("{path} is not an asset of the port"))
}

/// The lock test: under `GSD_BLESS`, rewrites the lock from the assets first; then the lock must equal the computed
/// map, every own row must hold a group of [`GROUPS`] (its computed group when it qualifies, no computed group when
/// it does not), and none may be `unreviewed`.
pub fn check_lock() {
    let before = lock_rows();
    let computed = computed_rows();
    if let Some(mode) = bless() {
        let blessed = carry(&before, computed.clone(), mode);
        std::fs::write(lock_path(), render(&blessed)).expect("write port.lock");
    }
    let text = std::fs::read_to_string(lock_path()).expect("port.lock (bless it with GSD_BLESS=1)");
    assert!(
        text.starts_with(&header()),
        "port.lock must start with `{}`",
        header()
    );
    let assets = assets();
    assert_eq!(
        assets.iter().filter(|a| a.starts_with(SECURITY)).count(),
        5,
        "@gsd/security's agents, command, workflow and template"
    );
    let lock = rows(&text);
    let by_asset = |rows: &[Row]| {
        let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for r in rows {
            out.entry(r.asset.clone()).or_default().push(r.key());
        }
        out
    };
    let (theirs, ours) = (by_asset(&lock), by_asset(&computed));
    let mut moved = Vec::new();
    for asset in theirs
        .keys()
        .chain(ours.keys().filter(|a| !theirs.contains_key(*a)))
    {
        let (l, c) = (
            theirs.get(asset).cloned().unwrap_or_default(),
            ours.get(asset).cloned().unwrap_or_default(),
        );
        if l != c {
            let gone = l
                .iter()
                .filter(|r| !c.contains(r))
                .take(20)
                .map(|r| format!("-{r}"));
            let new = c
                .iter()
                .filter(|r| !l.contains(r))
                .take(20)
                .map(|r| format!("+{r}"));
            moved.push(format!(
                "{asset}:\n{}",
                gone.chain(new).collect::<Vec<_>>().join("\n")
            ));
        }
    }
    assert!(
        moved.is_empty(),
        "port.lock is not the assets' span map (re-bless with GSD_BLESS=1, then set each `unreviewed` group):\n{}",
        moved.join("\n")
    );
    let names: Vec<&str> = GROUPS.iter().map(|(g, _)| *g).collect();
    let mut unreviewed = Vec::new();
    for (row, now) in lock.iter().zip(&computed).filter(|(r, _)| !r.kept) {
        let at = format!("{}:{}+{}", row.asset, row.up, row.n);
        assert!(
            names.contains(&row.group.as_str()),
            "{at}: `{}` is not a group of GROUPS",
            row.group
        );
        if now.group.is_empty() {
            assert!(
                !COMPUTED.contains(&row.group.as_str()),
                "{at}: holds `{}`, a computed group it does not qualify for",
                row.group
            );
        } else {
            assert_eq!(
                row.group, now.group,
                "{at}: qualifies for its computed group"
            );
        }
        if row.group == "unreviewed" {
            let hint: Vec<&str> = before
                .iter()
                .filter(|o| {
                    !o.kept
                        && o.asset == row.asset
                        && o.up <= row.up + row.n
                        && row.up <= o.up + o.n
                        && o.group != "unreviewed"
                })
                .map(|o| o.group.as_str())
                .collect();
            unreviewed.push(format!("{at} (was {hint:?})"));
        }
    }
    assert!(
        unreviewed.is_empty(),
        "own spans no group was carried to: set each group cell in port.lock:\n{}",
        unreviewed.join("\n")
    );
}
