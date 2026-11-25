//! IMP-02, the measure of ADR 0391 D13: each of taskboard, ledger and vault, imported from its `.planning/` alone by the
//! guest as built, against its hand-written twin. The entity sets are compared by [`crate::compare`], every remaining
//! difference listed with its reason in [`DIFFERENCES`] and held both ways. Run with `GSD_IMPORT_DIFF=1` (and
//! `--no-capture`) to print every difference instead of failing.

use serde_json::{Value, json};

use crate::compare::Expect::{Absent, Is};
use crate::compare::{self, Excluded, Expect, ID_PAIRS};
use crate::harness::{Project, judged, ledger, taskboard, unjudged, vault};
use crate::imported;

/// The profiles every pin holds under: `PROFILES` of `gsd/core/tests/cli/interface_gate.rs`.
const PROFILES: [&str; 3] = ["gsd-compat", "gsd-strict", "gsd-plus"];

/// What the imported vault carries under every profile, sorted: the twin's `VAULT_CARRIES`
/// (`gsd/core/tests/cli/rules_silent.rs`) with two `gsd.security/W302` instead of five, because the twin's closed
/// `01-01/T-01-02` and both `T-01-SC` threats have no status in `.planning/` and the import invents none (ADR 0391
/// D13). The two `gsd.security/E101` (threat `T-01-02` claimed in plans 01-01 and 01-02) land, an import being judged
/// for host errors only (ADR 0392).
pub const IMPORTED_VAULT_CARRIES: [&str; 7] = [
    "W019",
    "gsd.security/E101",
    "gsd.security/E101",
    "gsd.security/W302",
    "gsd.security/W302",
    "gsd.security/W303",
    "gsd/W103",
];

/// The security workflow's verdicts the imported vault owes, `(code, entity)`, which `@gsd/security`'s book judges and
/// core's harness `BOOKS` does not hold: W302 on each threat `01-SECURITY.md` closes (`T-01-01` of plan 01-01 and
/// `T-01-03` of plan 01-02), W303 on the audit.
pub const VAULT_SECURITY_OWED: [(&str, &str); 3] = [
    ("gsd.security/W302", "v1.0/01/01-01/T-01-01"),
    ("gsd.security/W302", "v1.0/01/01-02/T-01-03"),
    ("gsd.security/W303", "v1.0/01/01-SECURITY"),
];

/// `(fixture, kind, id, field, import, reason)`: each difference of an imported fixture from its twin that is not a
/// guest defect, what the import's value is (folded as [`compare::differences`] folds it), and where it comes from.
/// `field` is a field name, `title`, [`compare::IMPORT_ONLY`] or [`compare::TWIN_ONLY`]. A listed field is checked on
/// both sides: the twin's value differs, and the import's is the one the reason describes.
pub const DIFFERENCES: &[(&str, &str, &str, &str, Expect, &str)] = &[
    // taskboard
    (
        "taskboard",
        "deliverable",
        "v1.0/01/01-01/01-01-SUMMARY/D1",
        "title",
        Is(r#""a task is added with the next integer id and keeps its title""#),
        "twin shortens: its title cuts the coverage description `01-01-SUMMARY.md:27` (spec/plans.spec:264), which the import keeps whole",
    ),
    (
        "taskboard",
        "deliverable",
        "v1.0/01/01-02/01-02-SUMMARY/D1",
        "title",
        Is(r#""a task is completed by its id, and an unknown id is refused by name""#),
        "twin shortens: its title cuts the coverage description `01-02-SUMMARY.md:30`, which the import keeps whole",
    ),
    (
        "taskboard",
        "discussion",
        "v1.0/02/02-DISCUSSION-LOG",
        "entries",
        Is(
            r#"[{"area":"storage format","chosen":"one JSON file","mode":"interactive","notes":"the project ships as one script with no dependencies, so SQLite is out","options":["one JSON file","sQLite"]},{"area":"titles","chosen":"stored as typed","mode":"interactive","notes":"a title is the user's own words","options":["stored as typed","trimmed"]}]"#,
        ),
        "twin gap (RESEARCH D9): the twin drops each entry's `**Notes:**` line (`02-DISCUSSION-LOG.md:20,32`), which the import maps to `notes`",
    ),
    (
        "taskboard",
        "finding",
        "v1.0/02/02-RESEARCH/F1",
        "sources",
        Absent,
        "no source (RESEARCH D7): the recommendation line `02-RESEARCH.md:85` names no source; the twin invents `[SRC1]`",
    ),
    (
        "taskboard",
        "finding",
        "v1.0/02/02-RESEARCH/F1",
        "topic",
        Absent,
        "no source (RESEARCH D7): the recommendation line `02-RESEARCH.md:85` names no topic; the twin invents `stack`",
    ),
    (
        "taskboard",
        "plan",
        "v1.0/01/01-01",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "taskboard",
        "plan",
        "v1.0/01/01-02",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "taskboard",
        "plan",
        "v1.0/02/02-01",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "taskboard",
        "project",
        "taskboard",
        "context",
        Is(
            r#""built to prove the GSD model in SpecForge. Small enough that one person reads all of it""#,
        ),
        "twin shortens (RESEARCH D2): the twin keeps the first sentence of `PROJECT.md:31`, the import the whole section",
    ),
    (
        "taskboard",
        "project",
        "taskboard",
        "out_of_scope",
        Is(
            r#"[{"item":"sharing a board by link","why":"needs accounts, which v1 does not have"}]"#,
        ),
        "twin gap (RESEARCH D3): `PROJECT.md:25-27` is mapped to `out_of_scope`; the twin carries it as requirement SHARE-01 instead",
    ),
    (
        "taskboard",
        "project",
        "taskboard",
        "what",
        Is(
            r#""a small board for tracking tasks from the command line. A user adds a task with a title, marks it complete and lists what is left. It is a Node 20 script with no dependencies, built to prove the GSD model in SpecForge""#,
        ),
        "twin shortens (RESEARCH D1): the twin keeps the first sentence of `PROJECT.md:5-6`, the import the whole section",
    ),
    (
        "taskboard",
        "requirement",
        "SHARE-01",
        "text",
        Is(r#""share a board by link""#),
        "twin invents (RESEARCH D4): an out-of-scope row has no requirement text; the import takes the Feature cell `REQUIREMENTS.md:39`",
    ),
    (
        "taskboard",
        "research",
        "v1.0/02/02-RESEARCH",
        "summary",
        Is(
            r#""a board of a few hundred tasks fits in one JSON file. Node's own fs module reads and writes it, and a write to a temporary name followed by a rename keeps the file whole if the process dies mid-write""#,
        ),
        "twin paraphrases (RESEARCH D8): the import keeps the paragraph `02-RESEARCH.md:34` as written",
    ),
    (
        "taskboard",
        "summary",
        "v1.0/01/01-02/01-02-SUMMARY",
        "requires",
        Is(r#"[{"phase":"01","provides":"createBoard and addTask"}]"#),
        "twin gap (RESEARCH D6): the block-form `requires` of `01-02-SUMMARY.md:7-9` is mapped; the twin has none",
    ),
    (
        "taskboard",
        "uat_session",
        "v1.0/01/01-UAT",
        "source",
        Is(r#"["01-01/01-01-SUMMARY","01-02/01-02-SUMMARY"]"#),
        "reference spelling: the import writes each summary scope-relative (`01-01/01-01-SUMMARY`), the twin the full path (spec/verify.spec:3); both resolve to the same entity",
    ),
    // ledger
    (
        "ledger",
        "plan",
        "v1.0/01/01-01",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "ledger",
        "plan",
        "v1.0/01/01-02",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "ledger",
        "plan",
        "v1.0/02/02-01",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "ledger",
        "project",
        "ledger",
        "out_of_scope",
        Is(r#"[{"item":"currencies other than one","why":"a ledger is one currency"}]"#),
        "twin gap (RESEARCH D3): `PROJECT.md:25-27` is mapped to `out_of_scope`; the twin carries it as requirement CURR-01 instead",
    ),
    (
        "ledger",
        "project",
        "ledger",
        "what",
        Is(
            r#""a ledger of money on the command line: a user adds an entry in integer cents with a memo, and asks for the total. It is a Rust binary with no dependencies, built to give the GSD A/B a project whose CI is red""#,
        ),
        "twin shortens (RESEARCH D1): the twin keeps the first sentence of `PROJECT.md:5-6`, the import the whole section",
    ),
    (
        "ledger",
        "requirement",
        "CURR-01",
        "text",
        Is(r#""several currencies""#),
        "twin invents (RESEARCH D4): an out-of-scope row has no requirement text; the import takes the Feature cell `REQUIREMENTS.md:37`",
    ),
    (
        "ledger",
        "summary",
        "261105-tcl/261105-tcl-SUMMARY",
        "subsystem",
        Is(r#""quick""#),
        "no source (RESEARCH D14): the quick summary has no `subsystem`; the import fills `quick` and reports it, the twin invents `entries`",
    ),
    (
        "ledger",
        "task",
        "v1.0/01/01-01/T1",
        "action",
        Is(
            r#""create the ledger package (no dependencies) and src/lib.rs with Entry { cents: i64, memo: String } and add(entries, cents, memo), which refuses a memo holding a tab or a line break; line and parse write and read the store's <cents>\\t<memo> lines""#,
        ),
        "twin escape: the twin writes `\\t` inside a .spec string, which the parser reads as a tab (spec/plans.spec:18); the import keeps the two characters of `01-01-PLAN.md:42`",
    ),
    // vault
    (
        "vault",
        "accepted_risk",
        "v1.0/01/01-SECURITY/AR1",
        "threat",
        Is(r#""01-02/T-01-03""#),
        "reference spelling: the import writes the threat scope-relative (`01-02/T-01-03`), the twin the full path (spec/security.spec:7); both resolve to the same entity",
    ),
    (
        "vault",
        "audit_run",
        "v1.0/01/01-SECURITY/AU1",
        "at",
        Is(r#""2026-10-29T00:00:00Z""#),
        "no source (RESEARCH D12): `01-SECURITY.md:52` gives a date only; the import fills `T00:00:00Z` and reports it, the twin invents 10:00",
    ),
    (
        "vault",
        "audit_run",
        "v1.0/01/01-SECURITY/AU1",
        "run_by",
        Absent,
        "no source (RESEARCH D12): the audit trail names no runner; the twin invents `gsd-security-auditor` (spec/security.spec:19)",
    ),
    (
        "vault",
        "plan",
        "v1.0/01/01-01",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "vault",
        "plan",
        "v1.0/01/01-02",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "vault",
        "plan",
        "v1.0/02/02-01",
        "context_refs",
        Absent,
        "twin's own paths (RESEARCH D5): the twin names its spec files; the source's `.planning/*` refs name files the import does not write",
    ),
    (
        "vault",
        "project",
        "vault",
        "context",
        Is(
            r#""built to prove the GSD security capability in SpecForge. Small enough that one person reads all of it""#,
        ),
        "twin shortens (RESEARCH D2): the twin keeps the first sentence of `PROJECT.md:21`, the import the whole section",
    ),
    (
        "vault",
        "project",
        "vault",
        "what",
        Is(
            r#""a small encrypted store for secrets, driven from the command line. A user sets a named secret, reads it back and lists the names. It is a Node 20 script with no dependencies, built to prove the GSD security model in SpecForge""#,
        ),
        "twin shortens (RESEARCH D1): the twin keeps the first sentence of `PROJECT.md:5-6`, the import the whole section",
    ),
    (
        "vault",
        "security_audit",
        "v1.0/01/01-SECURITY",
        "asvs_level",
        Is(r#"1"#),
        "twin gap (RESEARCH D12): the frontmatter `asvs_level` of `01-SECURITY.md:7` is mapped (harness, as a peer); the twin has none",
    ),
    (
        "vault",
        "security_audit",
        "v1.0/01/01-SECURITY",
        "created",
        Is(r#""2026-10-30T00:00:00Z""#),
        "twin gap (RESEARCH D12): the frontmatter `created` of `01-SECURITY.md:8` is mapped, filled to midnight; the twin has none",
    ),
    (
        "vault",
        "summary",
        "v1.0/01/01-02/01-02-SUMMARY",
        "threat_flags",
        Is(
            r#"["threat_flag: information_disclosure | src/cli.js | the secret is an argv entry, visible in ps and in shell history"]"#,
        ),
        "twin gap (RESEARCH D15): the `## Threat Flags` table of `01-02-SUMMARY.md:45-49` is mapped; the twin has none",
    ),
    (
        "vault",
        "task",
        "v1.0/01/01-01/T1",
        "action",
        Is(
            r#""create package.json (ES modules, npm test runs node --test) and src/vault.js: seal(vault, key) encrypts the secrets with AES-256-GCM under a fresh 12-byte IV and returns IV, auth tag and data; open(blob, key) reverses it and throws when the tag does not verify""#,
        ),
        "twin shortens (RESEARCH D10): the import keeps `01-01-PLAN.md:41` verbatim",
    ),
    (
        "vault",
        "task",
        "v1.0/01/01-02/T1",
        "verify_text",
        Is(
            r#""vAULT_KEY=$(node -e \"console.log('07'.repeat(32))\") node src/cli.js set db hunter2 && VAULT_KEY=$(node -e \"console.log('07'.repeat(32))\") node src/cli.js get db""#,
        ),
        "twin shortens (RESEARCH D10): the import keeps the `<verify>` of `01-02-PLAN.md:42` verbatim",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-01",
        "evidence",
        Absent,
        "no source (RESEARCH D11): `01-SECURITY.md` records no evidence; the twin holds what an audit would write",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-01",
        "files",
        Absent,
        "authored after import (RESEARCH D11): `files` is SpecForge-only, with no `.planning/` counterpart (vault README)",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-02",
        "evidence",
        Absent,
        "no source (RESEARCH D11): `01-SECURITY.md` records no evidence; the twin holds what an audit would write",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-02",
        "files",
        Absent,
        "authored after import (RESEARCH D11): `files` is SpecForge-only, with no `.planning/` counterpart (vault README)",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-02",
        "status",
        Absent,
        "no source (RESEARCH D11, D13): `01-SECURITY.md` has no row for plan 01-01's T-01-02 (its one T-01-02 row, line 31, names plan 01-02's component); the import invents no status",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-SC",
        "evidence",
        Absent,
        "no source (RESEARCH D11): `01-SECURITY.md` records no evidence; the twin holds what an audit would write",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-SC",
        "files",
        Absent,
        "authored after import (RESEARCH D11): `files` is SpecForge-only, with no `.planning/` counterpart (vault README)",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-01/T-01-SC",
        "status",
        Absent,
        "no source (RESEARCH D11, D13): `01-SECURITY.md` has no T-01-SC row; the import invents no status",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-02/T-01-03",
        "files",
        Absent,
        "authored after import (RESEARCH D11): `files` is SpecForge-only, with no `.planning/` counterpart (vault README)",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-02/T-01-SC",
        "evidence",
        Absent,
        "no source (RESEARCH D11): `01-SECURITY.md` records no evidence; the twin holds what an audit would write",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-02/T-01-SC",
        "files",
        Absent,
        "authored after import (RESEARCH D11): `files` is SpecForge-only, with no `.planning/` counterpart (vault README)",
    ),
    (
        "vault",
        "threat",
        "v1.0/01/01-02/T-01-SC",
        "status",
        Absent,
        "no source (RESEARCH D11, D13): `01-SECURITY.md` has no T-01-SC row; the import invents no status",
    ),
    (
        "vault",
        "trust_boundary",
        "v1.0/01/01-02/TB1",
        "data_crossing",
        Is(r#""secrets and the key, in the clear""#),
        "twin gap (RESEARCH D13): the Data Crossing cell of `01-SECURITY.md:22` is mapped; the twin has none",
    ),
];

/// A fixture's name, its twin, and the same fixture before its import.
type Fixture = (&'static str, fn() -> Project, fn() -> Project);

/// The vault twin as its parity suite loads it, with `@gsd/security`.
fn secured_vault() -> Project {
    vault().with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

fn imported_taskboard() -> Project {
    imported(taskboard())
}

fn imported_ledger() -> Project {
    imported(ledger())
}

fn imported_vault() -> Project {
    imported(vault()).with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

pub const FIXTURES: [Fixture; 3] = [
    ("taskboard", taskboard, imported_taskboard),
    ("ledger", ledger, imported_ledger),
    ("vault", secured_vault, imported_vault),
];

/// `project` once `specforge gsd import` has run on it, which must succeed.
pub fn run_import(project: Project) -> Project {
    let run = project.run(&["gsd", "import"]);
    assert_eq!(run.code, 0, "{}", run.all());
    project
}

/// Every node of `project`'s graph, as `specforge export --format graph` writes them.
fn graph(project: &Project) -> Vec<Value> {
    let run = project.run(&["export", "--format", "graph"]);
    let graph: Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("export prints JSON ({e}): {}", run.all()));
    graph["nodes"].as_array().expect("nodes").clone()
}

#[test]
fn imported_entities_equal_the_twins() {
    let printing = std::env::var_os("GSD_IMPORT_DIFF").is_some();
    let excluded = Excluded::gsd();
    for (name, twin, before) in FIXTURES {
        let twin = graph(&twin());
        let import = graph(&run_import(before()));
        let pairs: Vec<(&str, &str, &str)> = ID_PAIRS
            .iter()
            .filter(|(fixture, ..)| *fixture == name)
            .map(|(_, kind, from, to)| (*kind, *from, *to))
            .collect();
        let listed: Vec<(&str, &str, &str, Expect)> = DIFFERENCES
            .iter()
            .filter(|(fixture, ..)| *fixture == name)
            .map(|(_, kind, id, field, import, _)| (*kind, *id, *field, *import))
            .collect();
        if printing {
            for d in compare::differences(&import, &twin, &pairs, &excluded) {
                eprintln!(
                    "{name}\t{}\t{}\t{}\ttwin={}\timport={}",
                    d.kind, d.id, d.field, d.twin, d.import
                );
            }
            continue;
        }
        let compared = compare::compare(&import, &twin, &pairs, &listed, &excluded);
        assert_eq!(compared, compare::Compared::default(), "{name}");
    }
}

/// The classes a difference's reason opens with (13-RESEARCH "Per-Fixture Mapping").
const CLASSES: [&str; 9] = [
    "twin shortens",
    "twin gap",
    "twin invents",
    "twin paraphrases",
    "twin escape",
    "twin's own paths",
    "no source",
    "authored after import",
    "reference spelling",
];

/// The lines of the fixture file named `name` (`PROJECT.md`, `spec/plans.spec`) under `root`, if one is there.
fn fixture_lines(root: &std::path::Path, name: &str) -> Option<Vec<String>> {
    let base = name.rsplit('/').next().unwrap_or(name);
    for entry in std::fs::read_dir(root).ok()? {
        let path = entry.ok()?.path();
        let found = if path.is_dir() {
            fixture_lines(&path, name)
        } else if path.file_name().is_some_and(|f| f == base) && path.ends_with(name) {
            std::fs::read_to_string(&path)
                .ok()
                .map(|t| t.lines().map(str::to_string).collect())
        } else {
            None
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Each reason names its class and where the difference comes from: a 13-RESEARCH D-number, or a `file:line` that
/// the fixture holds (its `.planning/` source or its twin's spec) at a line that is not blank.
#[test]
fn every_difference_names_its_class_and_a_live_citation() {
    let cite = regex::Regex::new(r"([\w./-]+\.(?:md|spec)):(\d+)").expect("a pattern");
    let research = regex::Regex::new(r"RESEARCH D\d+").expect("a pattern");
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures");
    for (fixture, kind, id, field, _, reason) in DIFFERENCES {
        let at = format!("{fixture} {kind} {id} {field}");
        assert!(
            CLASSES.iter().any(|c| reason.starts_with(c)),
            "{at}: `{reason}` names none of {CLASSES:?}"
        );
        let cited: Vec<(String, usize)> = cite
            .captures_iter(reason)
            .map(|c| (c[1].to_string(), c[2].parse().unwrap_or(0)))
            .collect();
        assert!(
            !cited.is_empty() || research.is_match(reason),
            "{at}: `{reason}` cites nothing"
        );
        for (file, line) in cited {
            let lines = fixture_lines(&fixtures.join(fixture), &file)
                .unwrap_or_else(|| panic!("{at}: {fixture} holds no {file}"));
            assert!(
                line >= 1 && lines.get(line - 1).is_some_and(|l| !l.trim().is_empty()),
                "{at}: {file}:{line} is past the file or blank"
            );
        }
    }
}

/// The codes a fixture's import is pinned to under every profile: none for taskboard and ledger.
fn pinned(fixture: &str) -> &'static [&'static str] {
    match fixture {
        "vault" => &IMPORTED_VAULT_CARRIES,
        _ => &[],
    }
}

#[test]
fn imported_codes_equal_the_pins() {
    for (name, _, before) in FIXTURES {
        let p = run_import(before());
        for profile in PROFILES {
            p.settings(json!({ "profile": profile }));
            let mut codes = p.codes();
            codes.sort();
            assert_eq!(codes, pinned(name), "{name} {profile}");
        }
    }
}

/// `(code, entity)` of every verdict `specforge gsd import --format json` on `project` reports owed, sorted.
fn owed_by_import(project: &Project) -> Vec<(String, String)> {
    let run = project.run(&["gsd", "import", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let answer: Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("the answer is JSON ({e}): {}", run.all()));
    let mut owed: Vec<(String, String)> = answer["output"]["owed"]
        .as_array()
        .expect("the owed list")
        .iter()
        .map(|o| {
            let text = |key: &str| o[key].as_str().expect("a string").to_string();
            (text("code"), text("entity"))
        })
        .collect();
    owed.sort();
    owed
}

/// `check` on an imported fixture exits 1 (ADR 0391 D5, D13; ADR 0392): the judged rules' `no verdict` on exactly the
/// entities core's books say owe one, nothing else but the fixture's pin, and the import's own owed list names the same
/// pairs, with the vault's security verdicts, which core's books do not hold.
#[test]
fn check_owes_exactly_the_judged_codes() {
    for (name, _, before) in FIXTURES {
        let p = before();
        let owed = owed_by_import(&p);
        let run = p.run(&["check"]);
        assert_eq!(run.code, 1, "{name}: {}", run.all());
        let report = p.report_all();
        let expected = p.expected_judged(&[]);
        assert_eq!(judged(&report), expected, "{name}: {report:#?}");
        assert!(
            report
                .iter()
                .filter(|(code, _)| crate::harness::is_judged(code))
                .all(|(_, message)| message.ends_with(": no verdict")),
            "{name}: {report:#?}"
        );
        let rest = unjudged(&report);
        let mut codes: Vec<&str> = rest.iter().map(|(code, _)| code.as_str()).collect();
        codes.sort();
        assert_eq!(codes, pinned(name), "{name}: {report:#?}");
        let mut owes = expected;
        if name == "vault" {
            let security: Vec<(String, String)> = rest
                .iter()
                .filter(|(code, _)| {
                    matches!(code.as_str(), "gsd.security/W302" | "gsd.security/W303")
                })
                .map(|(code, message)| {
                    let id = message.split('\'').nth(1).unwrap_or_else(|| {
                        panic!("a security verdict names its entity: {message}")
                    });
                    (code.clone(), id.to_string())
                })
                .collect();
            let pinned: Vec<(String, String)> = VAULT_SECURITY_OWED
                .iter()
                .map(|(code, entity)| (code.to_string(), entity.to_string()))
                .collect();
            let mut found = security;
            found.sort();
            assert_eq!(found, pinned, "{report:#?}");
            owes.extend(pinned);
            owes.sort();
        }
        assert_eq!(owed, owes, "{name}: the import's owed list");
    }
}
