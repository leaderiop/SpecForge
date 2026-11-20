//! The judged rules in the harness: which diagnostics are a judged rule's, what a project with no verdict reports,
//! and playing a judge (reading a rule's inputs and recording its verdict). A child of `harness.rs`, so it reaches
//! [`Project`]'s fields; `@gsd/security`'s tests compile it through their include of `harness.rs`.

use std::collections::{BTreeMap, BTreeSet};

use gsd_kit::judged::{Judged, Owed};
use serde_json::Value;

use super::cites::cite_flags;
use super::{Project, Run};

/// One extension's judged rules as the harness reads them: its table, the command that records their verdicts, its
/// kinds (for the owner states that say when a verdict is owed) and its codes' severities. Each test crate root lists
/// the books it knows in `crate::BOOKS`.
pub struct Book {
    /// The extension, as `specforge.json` loads it: `@gsd/core`.
    pub extension: &'static str,
    /// The namespace of its codes: `gsd`, so a rule is `gsd/E501`.
    pub namespace: &'static str,
    pub table: &'static [Judged],
    /// The `specforge gsd` command that records a verdict on one of its rules.
    pub command: &'static str,
    /// Its kinds, as `describe("entities")` answers them.
    pub kinds: fn() -> Value,
    /// Whether one of its local codes is an Error.
    pub is_error: fn(&str) -> bool,
}

fn core_kinds() -> Value {
    let host = specforge_extension_sdk::testing::MockHost::new(gsd_core::build());
    let text = host.describe_json("entities").expect("the entities");
    serde_json::from_str(&text).expect("describe answers JSON")
}

fn core_error(code: &str) -> bool {
    gsd_core::codes::CODES
        .iter()
        .any(|k| k.id == code && k.severity == gsd_core::codes::Severity::Error)
}

impl Book {
    pub const CORE: Book = Book {
        extension: "@gsd/core",
        namespace: "gsd",
        table: gsd_core::judged::JUDGED,
        command: "record-verdict",
        kinds: core_kinds,
        is_error: core_error,
    };

    /// The qualified code of a local one: `gsd/E501`.
    fn qualified(&self, local: &str) -> String {
        format!("{}/{local}", self.namespace)
    }
}

/// The book whose table holds `code` and its local code: a qualified code (`gsd/W311`) by its namespace, a local
/// one (`W311`) in the first book that has it.
fn book_of(code: &str) -> Option<(&'static Book, &str)> {
    crate::BOOKS.iter().find_map(|book| {
        let local = match code.rsplit_once('/') {
            Some((namespace, local)) if namespace == book.namespace => local,
            Some(_) => return None,
            None => code,
        };
        book.table
            .iter()
            .any(|j| j.code == local)
            .then_some((book, local))
    })
}

/// The book of a local or qualified code a test names: a test that names no judged rule is wrong.
fn book(code: &str) -> (&'static Book, &str) {
    book_of(code)
        .unwrap_or_else(|| panic!("'{code}' is no judged rule of any book in crate::BOOKS"))
}

/// Whether a diagnostic code (`gsd/W311`) is one of the judged rules' of a book.
pub fn is_judged(code: &str) -> bool {
    book_of(code).is_some()
}

/// The `(code, entity)` pairs of the judged rules' diagnostics in a report from [`Project::report_all`], sorted. The
/// entity is the quoted id every judged message carries.
pub fn judged(report: &[(String, String)]) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = report
        .iter()
        .filter(|(code, _)| is_judged(code))
        .map(|(code, message)| {
            let id = message
                .split('\'')
                .nth(1)
                .unwrap_or_else(|| panic!("a judged message names its entity: {message}"));
            (code.clone(), id.to_string())
        })
        .collect();
    found.sort();
    found
}

/// What a report holds besides the judged rules' diagnostics.
pub fn unjudged(report: &[(String, String)]) -> Vec<(String, String)> {
    report
        .iter()
        .filter(|(code, _)| !is_judged(code))
        .cloned()
        .collect()
}

/// Whether the entity `id` is pending, as the host decides an owner state: its own kind's `complete_when`, else the
/// nearest scope ancestor's (its id with the last segment cut) whose kind declares one, does not hold; with none on
/// the chain, it is not pending. `when` maps a kind to its `complete_when` field and values.
fn pending(
    nodes: &BTreeMap<String, Value>,
    when: &BTreeMap<String, (String, Vec<String>)>,
    id: &str,
) -> bool {
    let mut at = Some(id);
    while let Some(id) = at {
        let declared = nodes
            .get(id)
            .and_then(|node| Some((node, when.get(node["kind"].as_str()?)?)));
        if let Some((node, (field, values))) = declared {
            return !node["fields"][field]
                .as_str()
                .is_some_and(|value| values.iter().any(|v| v == value));
        }
        at = id.rsplit_once('/').map(|(parent, _)| parent);
    }
    false
}

impl Project {
    /// The books of `crate::BOOKS` whose extension the project loads.
    fn books(&self) -> Vec<&'static Book> {
        let loaded = self.extensions.borrow();
        crate::BOOKS
            .iter()
            .filter(|book| loaded.iter().any(|(name, _)| name == book.extension))
            .collect()
    }

    /// The `(code, entity)` pairs every judged rule reports on a project that records no verdict, sorted: one for each
    /// entity of each kind the rule targets that owes a verdict now (the graph and the books' tables and kinds say
    /// which, not the judged diagnostics), except the rules in `off` (a gate that is off), an entity that writes a
    /// field the target says to skip and an entity that already carries a fresh passing verdict of the rule. A target
    /// that waits for its owner state owes one only once its owner is complete (`Owed::Complete`) or while it is
    /// pending (`Owed::Open`).
    pub fn expected_judged(&self, off: &[&str]) -> Vec<(String, String)> {
        let books = self.books();
        let mut when = BTreeMap::new();
        for book in &books {
            let kinds = (book.kinds)();
            for kind in kinds["items"].as_array().expect("the kinds") {
                let complete = &kind["complete_when"];
                let (Some(name), Some(field)) = (kind["name"].as_str(), complete["field"].as_str())
                else {
                    continue;
                };
                let values = complete["values"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| Some(v.as_str()?.to_string()))
                    .collect();
                when.insert(name.to_string(), (field.to_string(), values));
            }
        }
        let run = self.run(&["export", "--format", "graph"]);
        let graph: Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("export prints JSON ({e}): {}", run.all()));
        let nodes: BTreeMap<String, Value> = graph["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(|n| (n["id"].as_str().expect("an id").to_string(), n.clone()))
            .collect();
        let mut out = Vec::new();
        for book in books {
            for rule in book.table.iter().filter(|r| !off.contains(&r.code)) {
                let code = book.qualified(rule.code);
                for target in rule.targets {
                    for (id, node) in nodes.iter().filter(|(_, n)| n["kind"] == target.kind) {
                        let owed = match target.judged_while {
                            None => true,
                            Some(Owed::Complete) => !pending(&nodes, &when, id),
                            Some(Owed::Open) => pending(&nodes, &when, id),
                        };
                        let skipped = target
                            .unless_set
                            .iter()
                            .any(|f| node["fields"].get(f).is_some_and(|v| !v.is_null()));
                        // A fresh passing verdict on the node (the twin carries its verifier's) answers the rule.
                        let judged = node["judgements"].as_array().is_some_and(|all| {
                            all.iter().any(|j| {
                                j["rule"] == code.as_str()
                                    && j["state"] == "fresh"
                                    && j["passed"] == true
                            })
                        });
                        if owed && !skipped && !judged {
                            out.push((code.clone(), id.clone()));
                        }
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// The clean-report invariant: `specforge check` finds nothing but the judged rules' `no verdict` on exactly the
    /// entities they target (see [`Project::expected_judged`]). Failable both ways: a missing judged diagnostic, an
    /// extra one, and any other diagnostic fail it.
    pub fn check_exact(&self, off: &[&str]) {
        let run = self.run(&["check"]);
        let report = self.report_all();
        assert_eq!(unjudged(&report), Vec::new(), "{report:#?}");
        let found = judged(&report);
        let expected = self.expected_judged(off);
        assert_eq!(found, expected, "{report:#?}");
        // `check` exits 1 exactly when a judged rule that is an Error (under gsd-compat) reports.
        let errors = expected.iter().any(|(code, _)| {
            let (book, local) = book(code);
            (book.is_error)(local)
        });
        assert_eq!(run.code, i32::from(errors), "check: {}", run.all());
        assert!(
            report
                .iter()
                .filter(|(code, _)| is_judged(code))
                .all(|(_, message)| message.ends_with(": no verdict")),
            "{report:#?}"
        );
    }

    /// The project as it was before its verifier ran: every `verdict gsd/E501` block of `spec/` and its ledger line
    /// removed. The taskboard twin carries those verdicts; a test that needs phase 01 unverified strips them.
    pub fn strip_verdicts(&self) {
        let dir = self.root.join("spec");
        for entry in std::fs::read_dir(&dir).expect("the spec dir") {
            let path = entry.expect("an entry").path();
            let text = std::fs::read_to_string(&path).expect("a spec file");
            let mut kept = Vec::new();
            let mut skipping: Option<String> = None;
            for line in text.lines() {
                match &skipping {
                    Some(close) if line == close => skipping = None,
                    Some(_) => {}
                    None if line.trim_start().starts_with("verdict gsd/E501 ") => {
                        let indent = line.len() - line.trim_start().len();
                        skipping = Some(format!("{}}}", " ".repeat(indent)));
                    }
                    None => kept.push(line),
                }
            }
            std::fs::write(&path, format!("{}\n", kept.join("\n"))).expect("a spec file");
        }
        let ledger = self.read("specforge-harness");
        let kept: Vec<&str> = ledger
            .lines()
            .filter(|l| !l.contains(" verdict:gsd/E501 "))
            .collect();
        self.write("specforge-harness", &format!("{}\n", kept.join("\n")));
    }

    /// `specforge query <entity> --inputs <namespace>/<code> --format json`: what the judge reads, and the basis to
    /// record. `code` is local (`E501`) or qualified (`gsd.security/W302`), and names its book.
    pub fn inputs(&self, entity: &str, code: &str) -> serde_json::Value {
        let (book, local) = book(code);
        let run = self.run(&[
            "query",
            entity,
            "--inputs",
            &book.qualified(local),
            "--format",
            "json",
        ]);
        serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("inputs are JSON ({e}): {}", run.all()))
    }

    /// Plays the judge: reads the inputs of the rule `code` on `entity`, naming what `--cited` in `extra` cites, then
    /// records `outcome` over their basis with its book's command (`specforge gsd record-verdict`, or
    /// `record-security-verdict` for a `gsd.security/` rule). `extra` are further options (`--findings`, `--cited`,
    /// `--values`).
    pub fn record(&self, entity: &str, code: &str, outcome: &str, extra: &[&str]) -> Run {
        let cited = extra
            .iter()
            .position(|a| *a == "--cited")
            .map(|at| extra.get(at + 1).expect("--cited takes a JSON list"));
        let flags = cited.map(|list| cite_flags(list)).unwrap_or_default();
        let (book, local) = book(code);
        let rule = book.qualified(local);
        let mut args = vec!["query", entity, "--inputs", &rule, "--format", "json"];
        args.extend(flags.iter().map(String::as_str));
        let run = self.run(&args);
        let inputs: serde_json::Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("inputs are JSON ({e}): {}", run.all()));
        self.record_over(entity, code, outcome, &inputs["basis"].to_string(), extra)
    }

    /// Plays the verifier on every item of `phase` (`v1.0/01`): a `verified` verdict on each truth, artifact and key
    /// link that `gsd/E501` reports, a truth's citing what [`Project::cites`] says. Answers how many it recorded.
    pub fn verify_phase(&self, phase: &str) -> usize {
        let items: Vec<String> = judged(&self.report_all())
            .into_iter()
            .filter(|(code, id)| code == "gsd/E501" && id.starts_with(&format!("{phase}/")))
            .map(|(_, id)| id)
            .collect();
        for id in &items {
            let citing = self.citing(id);
            let extra: Vec<&str> = citing.iter().map(String::as_str).collect();
            let run = self.record(id, "E501", "verified", &extra);
            assert_eq!(run.code, 0, "{id}: {}", run.all());
        }
        items.len()
    }

    /// The answer of `specforge gsd re-verify <subject> [--stale-only] --format json`: the verifier's verdicts of a
    /// phase or a plan and whether each is fresh.
    pub fn re_verify(&self, subject: &str, stale_only: bool) -> Value {
        let mut args = vec!["gsd", "re-verify", subject];
        if stale_only {
            args.push("--stale-only");
        }
        args.extend(["--format", "json"]);
        let run = self.run(&args);
        assert_eq!(run.code, 0, "{}", run.all());
        serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("re-verify prints JSON ({e}): {}", run.all()))
    }

    /// The truths of `subject` (a phase or a plan) whose verdict is not fresh, as `re-verify --stale-only` lists them.
    pub fn stale_truths(&self, subject: &str) -> BTreeSet<String> {
        let answer = self.re_verify(subject, true);
        answer["items"]
            .as_array()
            .unwrap_or_else(|| panic!("re-verify lists items: {answer}"))
            .iter()
            .filter(|item| item["kind"] == "truth")
            .map(|item| item["entity"].as_str().expect("an entity").to_string())
            .collect()
    }

    /// Every recorded verdict of the project, byte for byte: each `verdict <rule> …` block of `spec/` by its entity
    /// and rule, each ledger line by its entity and rule, and the ledger's other lines.
    pub fn verdicts(&self) -> Verdicts {
        let mut blocks = BTreeMap::new();
        let dir = self.root.join("spec");
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .expect("the spec dir")
            .map(|entry| entry.expect("an entry").path())
            .collect();
        files.sort();
        for path in files {
            let text = std::fs::read_to_string(&path).expect("a spec file");
            blocks.extend(verdict_blocks(&text));
        }
        let mut ledger = BTreeMap::new();
        let mut other = Vec::new();
        for line in self.read("specforge-harness").lines() {
            let mut words = line.split(' ');
            match (
                words.next(),
                words.next().and_then(|w| w.strip_prefix("verdict:")),
            ) {
                (Some(entity), Some(rule)) => {
                    let key = (entity.to_string(), rule.to_string());
                    assert!(ledger.insert(key, line.to_string()).is_none(), "{line}");
                }
                _ => other.push(line.to_string()),
            }
        }
        Verdicts {
            blocks,
            ledger,
            other,
        }
    }
}

/// The recorded verdicts of a project at one moment (see [`Project::verdicts`]): blocks and ledger lines are keyed by
/// `(entity, rule)`.
#[derive(Debug, PartialEq)]
pub struct Verdicts {
    pub blocks: BTreeMap<(String, String), String>,
    pub ledger: BTreeMap<(String, String), String>,
    pub other: Vec<String>,
}

impl Verdicts {
    /// That `after` differs from `self` in exactly the `(entity, rule)` verdicts of `rejudged`, each of which was
    /// recorded again (its block and its ledger line both changed): every other block and line is byte-identical.
    pub fn assert_only(&self, after: &Verdicts, rejudged: &[(String, String)]) {
        assert_eq!(self.other, after.other, "the ledger's other lines");
        for (name, before, now) in [
            ("block", &self.blocks, &after.blocks),
            ("ledger line", &self.ledger, &after.ledger),
        ] {
            let keys: Vec<_> = before.keys().chain(now.keys()).collect();
            for key in keys {
                let (old, new) = (before.get(key), now.get(key));
                if rejudged.contains(key) {
                    assert!(
                        old.is_some() && new.is_some() && old != new,
                        "{name} {key:?} is recorded again: {old:?} -> {new:?}"
                    );
                } else {
                    assert_eq!(old, new, "{name} {key:?} is untouched");
                }
            }
        }
    }
}

/// The `verdict` blocks of one `.spec` text by `(entity, rule)`. A block's entity is the top-level statement it sits
/// in: its name, under the `plan` or `phase` it names when it names one (`truth TR1` with `plan v1.0/01/01-01` is
/// `v1.0/01/01-01/TR1`), as the graph's ids are scoped.
fn verdict_blocks(text: &str) -> BTreeMap<(String, String), String> {
    let mut out = BTreeMap::new();
    let mut lines = text.lines().peekable();
    while let Some(header) = lines.next() {
        if header.starts_with([' ', '}']) || !header.ends_with('{') {
            continue;
        }
        let name = header.split(' ').nth(1).unwrap_or_default().to_string();
        let mut scope = None;
        while let Some(line) = lines.next_if(|l| *l != "}") {
            let trimmed = line.trim_start();
            if scope.is_none() && line.len() - trimmed.len() == 2 {
                scope = ["plan ", "phase "]
                    .iter()
                    .find_map(|field| Some(trimmed.strip_prefix(field)?.trim().to_string()));
            }
            let Some(rest) = trimmed.strip_prefix("verdict ") else {
                continue;
            };
            let rule = rest.split(' ').next().unwrap_or_default().to_string();
            let close = format!("{}}}", &line[..line.len() - trimmed.len()]);
            let mut block = vec![line];
            for inner in lines.by_ref() {
                block.push(inner);
                if inner == close {
                    break;
                }
            }
            let entity = scope
                .as_ref()
                .map_or(name.clone(), |s| format!("{s}/{name}"));
            let key = (entity, rule);
            assert!(
                out.insert(key.clone(), block.join("\n")).is_none(),
                "two verdicts {key:?}"
            );
        }
    }
    out
}

impl Project {
    /// [`Project::record`] over a `basis` the caller read earlier.
    pub fn record_over(
        &self,
        entity: &str,
        code: &str,
        outcome: &str,
        basis: &str,
        extra: &[&str],
    ) -> Run {
        let (book, local) = book(code);
        let mut args = vec![
            "gsd",
            book.command,
            entity,
            local,
            outcome,
            "test-model",
            basis,
        ];
        args.extend_from_slice(extra);
        self.run(&args)
    }
}
