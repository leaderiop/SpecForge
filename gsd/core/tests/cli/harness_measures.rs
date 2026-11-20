//! The measures ADR 0379 defines, computed from what `specforge` prints and never typed: verdict freshness at the ship
//! gate and the re-verification scope of single-file edits. A child of `harness.rs`, so every crate that includes the
//! harness (core's tests and `@gsd/security`'s) shares them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use super::Project;

/// A verdict as the measures key it: `(rule, entity)`, the rule qualified (`gsd/E501`, `gsd.security/W302`).
pub type Verdict = (String, String);

/// Verdict freshness at the ship gate of a phase: the judgements the gate reads, the fresh ones among them, and what
/// its `gsd/E516` refuses.
#[derive(Debug)]
pub struct Freshness {
    /// Every judgement the graph lists, except a `missing` one outside the phase (E516's scope for a phase subject).
    pub required: BTreeSet<Verdict>,
    /// The required judgements whose state is `fresh`.
    pub fresh: BTreeSet<Verdict>,
    /// Each judgement a `gsd/E516` diagnostic of the refused ship names, with what the diagnostic says of it after
    /// the entity (`is missing`, `is stale: … changed`).
    pub refused: BTreeMap<Verdict, String>,
    /// Whether the ship gate's `gsd/E516` entry passes; a ship that renders passes every gate.
    pub gate_passes: bool,
}

impl Freshness {
    /// The judgements the gate reads, counted. A measure over nothing is not a measure: an empty set panics.
    fn denominator(&self) -> usize {
        assert!(
            !self.required.is_empty(),
            "the ship gate reads no judgement: {self:#?}"
        );
        self.required.len()
    }

    /// Fresh over required.
    pub fn ratio(&self) -> f64 {
        self.fresh.len() as f64 / self.denominator() as f64
    }

    /// The share in whole percent, rounded down: 100 only when every judgement is fresh.
    pub fn percent(&self) -> usize {
        self.fresh.len() * 100 / self.denominator()
    }

    /// The judgements the gate reads that are not fresh.
    pub fn not_fresh(&self) -> BTreeSet<Verdict> {
        self.required.difference(&self.fresh).cloned().collect()
    }

    /// The refused judgements, without what the diagnostics say of them.
    pub fn refused_set(&self) -> BTreeSet<Verdict> {
        self.refused.keys().cloned().collect()
    }

    /// The printed value: `verdict freshness at ship on the <twin> twin: <fresh> of <required> fresh (<percent>%)`.
    pub fn line(&self, twin: &str) -> String {
        format!(
            "verdict freshness at ship on the {twin} twin: {} of {} fresh ({}%)",
            self.fresh.len(),
            self.required.len(),
            self.percent()
        )
    }
}

/// `(rule, entity)` and what follows the entity, from a `gsd/E516` message: `the <rule> verdict on '<entity>' <rest>`.
fn e516_verdict(message: &str) -> (Verdict, String) {
    let parsed = message.strip_prefix("the ").and_then(|rest| {
        let (rule, rest) = rest.split_once(" verdict on '")?;
        let (entity, rest) = rest.split_once("' ")?;
        Some(((rule.to_string(), entity.to_string()), rest.to_string()))
    });
    parsed.unwrap_or_else(|| panic!("not a gsd/E516 verdict message: {message}"))
}

impl Project {
    /// Verdict freshness at the ship gate of `phase` (`v1.0/01`), read from `specforge export --format graph` (the
    /// judgements on its nodes) and `specforge prompt gsd.ship --format json -- <phase>` (exit 0: it renders; exit 1:
    /// the `gate_failed` answer, its `gsd/E516` diagnostics and gate entry). A refusal by another gate (gsd/E515 on a
    /// twin that records no commit) leaves the measure to E516's own entry and diagnostics.
    pub fn ship_freshness(&self, phase: &str) -> Freshness {
        let run = self.run(&["export", "--format", "graph"]);
        let graph: Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("export prints JSON ({e}): {}", run.all()));
        let scoped = format!("{phase}/");
        let mut required = BTreeSet::new();
        let mut fresh = BTreeSet::new();
        for node in graph["nodes"].as_array().expect("nodes") {
            let id = node["id"].as_str().expect("an id");
            for judgement in node["judgements"].as_array().into_iter().flatten() {
                let state = judgement["state"].as_str().expect("a state");
                if state == "missing" && id != phase && !id.starts_with(&scoped) {
                    continue;
                }
                let verdict = (
                    judgement["rule"].as_str().expect("a rule").to_string(),
                    id.to_string(),
                );
                if state == "fresh" {
                    fresh.insert(verdict.clone());
                }
                required.insert(verdict);
            }
        }
        let run = self.run(&[
            "prompt",
            "gsd.ship",
            "--host",
            "claude-code",
            "--format",
            "json",
            "--",
            phase,
        ]);
        let (refused, gate_passes) = match run.code {
            0 => (BTreeMap::new(), true),
            1 => refused_by_e516(&run.all()),
            _ => panic!("ship renders or refuses: {}", run.all()),
        };
        Freshness {
            required,
            fresh,
            refused,
            gate_passes,
        }
    }
}

/// The `gsd/E516` part of a refused prompt's `gate_failed` answer: each judgement a diagnostic names (one diagnostic
/// each: a second on the same judgement panics), and whether the gate's entry passes.
fn refused_by_e516(all: &str) -> (BTreeMap<Verdict, String>, bool) {
    let start = all
        .find("{\"code\"")
        .unwrap_or_else(|| panic!("no JSON in: {all}"));
    let answer: Value =
        serde_json::from_str(&all[start..]).unwrap_or_else(|e| panic!("{e}: {all}"));
    assert_eq!(answer["code"], "gate_failed", "{all}");
    let mut refused = BTreeMap::new();
    for diagnostic in answer["data"]["failing"].as_array().expect("failing") {
        if diagnostic["code"] != "gsd/E516" {
            continue;
        }
        let (verdict, says) = e516_verdict(diagnostic["message"].as_str().expect("a message"));
        assert!(
            refused.insert(verdict.clone(), says).is_none(),
            "two gsd/E516 diagnostics on {verdict:?}: {all}"
        );
    }
    let gate = answer["data"]["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|g| g["code"] == "gsd/E516")
        .unwrap_or_else(|| panic!("ship has no gsd/E516 gate (is the verdict gate on?): {all}"));
    (refused, gate["passes"].as_bool().expect("passes"))
}

/// `(file, rule, entity)`: a verdict of a sweep, by the edited file it belongs to.
pub type EditVerdict = (String, String, String);

/// The re-verification scope of a sweep of single-file edits: per edit, what a listing names against what is not
/// fresh, summed.
#[derive(Debug, Default)]
pub struct Scope {
    /// The edits made, one per file.
    pub edits: usize,
    /// The edits that staled nothing.
    pub quiet: usize,
    /// The edits that staled a proper, non-empty subset of the universe.
    pub partial: usize,
    /// The verdicts listed, summed over the edits.
    pub listed: usize,
    /// The verdicts not fresh, summed over the edits.
    pub stale: usize,
    /// Listed after an edit though fresh.
    pub over: BTreeSet<EditVerdict>,
    /// Not fresh after an edit though not listed.
    pub under: BTreeSet<EditVerdict>,
    /// Still listed once the edited file is restored byte for byte.
    pub restored: BTreeSet<EditVerdict>,
    /// `(listed, stale)` per rule, summed over the edits: every rule of the universe, and any other a listing names.
    pub by_rule: BTreeMap<String, (usize, usize)>,
    /// The stale set of each edited file.
    pub per_edit: BTreeMap<String, BTreeSet<Verdict>>,
}

impl Scope {
    /// Listed over stale. A sweep that stales and lists nothing measures nothing: it panics.
    pub fn ratio(&self) -> f64 {
        assert!(
            self.listed + self.stale > 0,
            "the sweep staled and listed nothing: {self:#?}"
        );
        self.listed as f64 / self.stale as f64
    }

    /// Every edit listed exactly its stale verdicts, and nothing once restored.
    pub fn exact(&self) -> bool {
        self.over.is_empty()
            && self.under.is_empty()
            && self.restored.is_empty()
            && self.listed == self.stale
    }

    /// The printed value: `re-verification scope on the <twin> twin: <L> re-judged of <S> stale over <E> single-file
    /// edits (<O> over-judged, <U> under-judged): <ratio>`, with the split by rule inside the parentheses when the
    /// sweep has more than one rule.
    pub fn line(&self, twin: &str) -> String {
        let split = if self.by_rule.len() > 1 {
            let rules: Vec<String> = self
                .by_rule
                .iter()
                .map(|(rule, (listed, stale))| format!("{rule} {listed} of {stale}"))
                .collect();
            format!("; {}", rules.join(", "))
        } else {
            String::new()
        };
        format!(
            "re-verification scope on the {twin} twin: {} re-judged of {} stale over {} single-file edits ({} \
             over-judged, {} under-judged{split}): {:?}",
            self.listed,
            self.stale,
            self.edits,
            self.over.len(),
            self.under.len(),
            self.ratio()
        )
    }
}

/// The paths under `dir`, relative to `base` and joined by `/`, a `.specforge` directory (the derived store) and the
/// verdict ledger at the top left out.
fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let entry = entry.expect("a directory entry");
        let path = entry.path();
        if entry.file_type().expect("a file type").is_dir() {
            if entry.file_name() != ".specforge" {
                walk(base, &path, out);
            }
        } else if !(dir == base && entry.file_name() == "specforge-harness") {
            let relative = path.strip_prefix(base).expect("under the fixture");
            let parts: Vec<&str> = relative
                .components()
                .map(|c| c.as_os_str().to_str().expect("a UTF-8 path"))
                .collect();
            out.push(parts.join("/"));
        }
    }
}

/// The JSON strings of `value`: one string, or each of a list.
fn strings(value: &Value) -> BTreeSet<String> {
    match value {
        Value::String(one) => [one.clone()].into(),
        Value::Array(all) => all
            .iter()
            .map(|v| v.as_str().expect("a string").to_string())
            .collect(),
        other => panic!("not a string or a list of strings: {other}"),
    }
}

impl Project {
    /// Every file of the fixture directory the project was copied from, relative and sorted, but the verdict ledger
    /// (`specforge-harness`: its edit is an edit of the verdicts, not of their inputs) and any `.specforge` directory.
    pub fn fixture_files(&self) -> Vec<String> {
        let source = self
            .source
            .as_deref()
            .expect("a project copied from a fixture directory");
        let mut files = Vec::new();
        walk(source, source, &mut files);
        files.sort();
        assert!(!files.is_empty(), "{} holds no file", source.display());
        files
    }

    /// The project files the verdict of `rule` on `entity` reads, according to the spec alone (no re-verification, no
    /// `query --inputs`): a truth's verifier verdict reads what it cites ([`Project::cites`]), an artifact's its
    /// `path`, a key link's its `from` and `to`; a threat's mitigation verdict reads its `files`; the audit's mapping
    /// verdict no project file. Any other rule or kind panics, so a new one gets its read set on purpose.
    pub fn reads(&self, rule: &str, entity: &str) -> BTreeSet<String> {
        let node = self.node(entity);
        let fields = &node["fields"];
        match (rule, node["kind"].as_str().unwrap_or_default()) {
            ("gsd/E501", "truth") => {
                let cited: Value = serde_json::from_str(&self.cites(entity)).expect("a JSON list");
                cited
                    .as_array()
                    .expect("a list")
                    .iter()
                    .map(|c| c["file"].as_str().expect("a cited file").to_string())
                    .collect()
            }
            ("gsd/E501", "artifact") => strings(&fields["path"]),
            ("gsd/E501", "key_link") => &strings(&fields["from"]) | &strings(&fields["to"]),
            ("gsd.security/W302", "threat") => strings(&fields["files"]),
            ("gsd.security/W303", "security_audit") => BTreeSet::new(),
            (rule, kind) => panic!("no read set for {rule} on a {kind} ('{entity}')"),
        }
    }

    /// The members of `universe` whose verdict `query <entity> --inputs <rule>` says is not fresh.
    fn not_fresh(&self, universe: &BTreeSet<Verdict>) -> BTreeSet<Verdict> {
        universe
            .iter()
            .filter(|(rule, entity)| self.inputs(entity, rule)["verdict"]["state"] != "fresh")
            .cloned()
            .collect()
    }

    /// The re-verification scope of `files`, edited one at a time: each gets a line break appended, `listing` names
    /// what it would re-judge and `query --inputs` says which members of `universe` are not fresh, then the file is
    /// restored byte for byte and `listing` asked again. An edit that stales nothing must list nothing, and counts as
    /// exact only because both sets are empty. The universe must be fresh before the sweep.
    pub fn rejudge_scope(
        &self,
        files: &[String],
        universe: &BTreeSet<Verdict>,
        listing: &dyn Fn(&Project) -> BTreeSet<Verdict>,
    ) -> Scope {
        assert!(!universe.is_empty(), "a sweep over no verdict");
        assert_eq!(
            self.not_fresh(universe),
            BTreeSet::new(),
            "stale before the sweep"
        );
        let mut scope = Scope::default();
        for (rule, _) in universe {
            scope.by_rule.entry(rule.clone()).or_default();
        }
        let keyed = |file: &String, set: BTreeSet<Verdict>| {
            set.into_iter()
                .map(|(rule, entity)| (file.clone(), rule, entity))
                .collect::<Vec<_>>()
        };
        for file in files {
            let path = self.root.join(file);
            let kept = std::fs::read(&path).unwrap_or_else(|e| panic!("read {file}: {e}"));
            let mut edited = kept.clone();
            edited.push(b'\n');
            std::fs::write(&path, &edited).unwrap_or_else(|e| panic!("edit {file}: {e}"));
            let listed = listing(self);
            let stale = self.not_fresh(universe);
            scope.edits += 1;
            scope.listed += listed.len();
            scope.stale += stale.len();
            for (rule, _) in &listed {
                scope.by_rule.entry(rule.clone()).or_default().0 += 1;
            }
            for (rule, _) in &stale {
                scope.by_rule.entry(rule.clone()).or_default().1 += 1;
            }
            scope.over.extend(keyed(file, &listed - &stale));
            scope.under.extend(keyed(file, &stale - &listed));
            if stale.is_empty() {
                scope.quiet += 1;
            } else if stale.len() < universe.len() {
                scope.partial += 1;
            }
            scope.per_edit.insert(file.clone(), stale);
            std::fs::write(&path, &kept).unwrap_or_else(|e| panic!("restore {file}: {e}"));
            scope.restored.extend(keyed(file, listing(self)));
        }
        scope
    }
}
