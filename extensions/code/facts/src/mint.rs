//! The minting of `@code/core`'s facts (ADR 0347): `test` and `test_run` from a runner's cases, `item` from a source
//! pack's definitions, `package` from a manifest.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use specforge_code_junit::Case;
use specforge_extension_sdk::host::SyntaxPoint;
use specforge_extension_sdk::prelude::{Fact, FactLocation, TestStatus};

use crate::{IdError, ItemId, PackageId, TargetKind, TestId};

/// The entities a fact names, each with the places it is written (none when its pack read no place): what
/// `Fact::name` records, so a rename of the entity rewrites them (ADR 0346 D7).
pub type Names = BTreeMap<String, Vec<FactLocation>>;

/// The place `file` holds between the reader's points `start` and `end` (1-based lines and byte columns, end
/// exclusive): the one conversion every source pack uses (ADR 0347 D6).
pub fn location(file: &str, start: SyntaxPoint, end: SyntaxPoint) -> FactLocation {
    FactLocation {
        file: file.to_string(),
        start_line: start.line as usize,
        start_col: start.byte_col as usize,
        end_line: end.line as usize,
        end_col: end.byte_col as usize,
    }
}

/// One run (or skip) of a test, as a runner pack read it.
#[derive(Debug, Clone, PartialEq)]
pub struct TestCase {
    pub id: TestId,
    pub status: TestStatus,
    pub seconds: Option<f64>,
    /// The test's file, project-relative with `/`, as the pack computed it (ADR 0347 D8).
    pub file: Option<String>,
    pub names: Names,
    pub at: Vec<FactLocation>,
    /// Whether it ran: a test the runner listed but did not run (ignored) has no `test_run`.
    pub ran: bool,
}

impl TestCase {
    /// The JUnit `case` named `id`, whose project-relative file is `file`: it ran.
    pub fn of(id: TestId, case: &Case, file: Option<String>) -> TestCase {
        TestCase {
            id,
            status: case.status,
            seconds: case.seconds,
            file,
            names: Names::new(),
            at: Vec::new(),
            ran: true,
        }
    }

    /// The test `id`, known to the pack but not run (an ignored test): a `test` of `status`, no `test_run`.
    pub fn unrun(id: TestId, status: TestStatus) -> TestCase {
        TestCase {
            id,
            status,
            seconds: None,
            file: None,
            names: Names::new(),
            at: Vec::new(),
            ran: false,
        }
    }
}

/// The `test` and `test_run` facts of `cases`, by id (ADR 0329 D3, ADR 0347):
///
/// - `test` (titled by its path): `package`, `file` (when the pack gave one), `suite`, `name`, `binary` (a Rust
///   test's nextest binary), `status`, `names`, and its places;
/// - `test_run` (`run:<test id>`), when it ran: `test`, `status` and `duration_ms`, when timed.
///
/// Both statuses come from the same cases. Cases of one id merge: the worst status wins (failed, then skipped, then
/// passed), durations add up, names and places join, and it ran if any did.
pub fn test_facts(cases: impl IntoIterator<Item = TestCase>) -> Vec<Fact> {
    let mut merged: BTreeMap<String, TestCase> = BTreeMap::new();
    for case in cases {
        match merged.get_mut(&case.id.to_string()) {
            Some(kept) => {
                if rank(case.status) > rank(kept.status) {
                    kept.status = case.status;
                }
                kept.seconds = match (kept.seconds, case.seconds) {
                    (Some(a), Some(b)) => Some(a + b),
                    (a, b) => a.or(b),
                };
                kept.file = kept.file.take().or(case.file);
                join_names(&mut kept.names, case.names);
                kept.at.extend(case.at);
                kept.ran |= case.ran;
            }
            None => {
                merged.insert(case.id.to_string(), case);
            }
        }
    }
    merged
        .into_iter()
        .flat_map(|(id, case)| {
            let status = case.status.as_str();
            let mut test = Fact::new("test", id.as_str())
                .title(case.id.path())
                .field("package", case.id.package())
                .field("suite", case.id.suite().to_vec())
                .field("name", case.id.name())
                .field("status", status);
            if let Some(file) = &case.file {
                test = test.field("file", file.as_str());
            }
            if let Some(binary) = case.id.binary() {
                test = test.field("binary", binary);
            }
            test = placed(named(test, case.names), case.at);
            let run = case.ran.then(|| {
                let run = Fact::new("test_run", case.id.run())
                    .field("test", id.as_str())
                    .field("status", status);
                match case.seconds {
                    // shortcut: a fact's number is read as an integer (ADR 0327 D5), so a run under 1 ms is 0.
                    Some(seconds) => run.field("duration_ms", (seconds * 1000.0).round() as u64),
                    None => run,
                }
            });
            std::iter::once(test).chain(run)
        })
        .collect()
}

/// How bad a status is: a merge keeps the worst.
fn rank(status: TestStatus) -> u8 {
    match status {
        TestStatus::Passed => 0,
        TestStatus::Skipped => 1,
        TestStatus::Failed => 2,
    }
}

/// What a definition is, in one vocabulary for Rust and JavaScript: `@code/core`'s `item.kind` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemKind {
    Module,
    Function,
    Method,
    Field,
    Struct,
    Enum,
    Union,
    Trait,
    Type,
    Const,
    Static,
    Macro,
    Class,
    Interface,
    Variable,
    Namespace,
}

/// Each kind and its name, in vocabulary order: the one table [`ItemKind::ALL`], [`ItemKind::NAMES`] and
/// [`ItemKind::as_str`] read.
const ITEM_KINDS: [(ItemKind, &str); 16] = [
    (ItemKind::Module, "module"),
    (ItemKind::Function, "function"),
    (ItemKind::Method, "method"),
    (ItemKind::Field, "field"),
    (ItemKind::Struct, "struct"),
    (ItemKind::Enum, "enum"),
    (ItemKind::Union, "union"),
    (ItemKind::Trait, "trait"),
    (ItemKind::Type, "type"),
    (ItemKind::Const, "const"),
    (ItemKind::Static, "static"),
    (ItemKind::Macro, "macro"),
    (ItemKind::Class, "class"),
    (ItemKind::Interface, "interface"),
    (ItemKind::Variable, "variable"),
    (ItemKind::Namespace, "namespace"),
];

impl ItemKind {
    /// Every kind, in vocabulary order.
    pub const ALL: [ItemKind; 16] = {
        let mut all = [ItemKind::Module; 16];
        let mut at = 0;
        while at < all.len() {
            all[at] = ITEM_KINDS[at].0;
            at += 1;
        }
        all
    };
    /// Every kind's name, in vocabulary order: `@code/core`'s vocabulary of `item.kind`.
    pub const NAMES: [&'static str; 16] = {
        let mut names = [""; 16];
        let mut at = 0;
        while at < names.len() {
            names[at] = ITEM_KINDS[at].1;
            at += 1;
        }
        names
    };

    pub fn as_str(self) -> &'static str {
        ITEM_KINDS
            .iter()
            .find(|(kind, _)| *kind == self)
            .map_or("", |(_, name)| name)
    }
}

/// Who may name a definition: `@code/core`'s `item.visibility` values, narrowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Visibility {
    Private,
    Restricted,
    Crate,
    Public,
}

/// Each visibility and its name, widest first: the one table [`Visibility::NAMES`] and [`Visibility::as_str`] read.
const VISIBILITIES: [(Visibility, &str); 4] = [
    (Visibility::Public, "public"),
    (Visibility::Crate, "crate"),
    (Visibility::Restricted, "restricted"),
    (Visibility::Private, "private"),
];

impl Visibility {
    /// Every visibility's name, widest first: `@code/core`'s vocabulary of `item.visibility`.
    pub const NAMES: [&'static str; 4] = {
        let mut names = [""; 4];
        let mut at = 0;
        while at < names.len() {
            names[at] = VISIBILITIES[at].1;
            at += 1;
        }
        names
    };

    pub fn as_str(self) -> &'static str {
        VISIBILITIES
            .iter()
            .find(|(visibility, _)| *visibility == self)
            .map_or("", |(_, name)| name)
    }

    /// From the `visibility_modifier` text a pack captured (`None`: none written), whitespace ignored (D10): `pub`
    /// public, `pub(crate)` and `crate` crate, `pub(self)` private, `pub(super)` and `pub(in …)` restricted.
    pub fn rust(modifier: Option<&str>) -> Visibility {
        let Some(modifier) = modifier else {
            return Visibility::Private;
        };
        let compact: String = modifier.chars().filter(|c| !c.is_whitespace()).collect();
        match compact.as_str() {
            "pub" => Visibility::Public,
            "pub(crate)" | "crate" => Visibility::Crate,
            "pub(self)" | "" => Visibility::Private,
            _ => Visibility::Restricted,
        }
    }

    /// Exported from its file (named, default or re-exported): public; otherwise private.
    pub fn js(exported: bool) -> Visibility {
        if exported {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }
}

/// One definition a source pack read: what [`item_facts`] merges by id.
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub id: ItemId,
    pub package: PackageId,
    pub kind: ItemKind,
    pub visibility: Visibility,
    /// Its file, project-relative with `/`.
    pub file: String,
    pub at: FactLocation,
    /// Its signature's text, digested with its twins' (never stored as written).
    pub signature: Option<String>,
    pub doc: Option<String>,
    /// `test`, `ignore`, `cfg(<predicate>)`, `export`, …
    pub attrs: Vec<String>,
    pub names: Names,
    /// The id the runner gives it, when it is a test ([`ItemId::test_id`]).
    pub test_id: Option<TestId>,
}

/// One `item` fact per distinct id of `definitions`, by id (ADR 0347 D3): titled by its own name, with `package`, `file`
/// (the first definition's, by file then line), `kind` (each definition's, in vocabulary order), `visibility` (the
/// widest), `signature` (`sha256:<hex>` over the definitions' signature texts, each whitespace-collapsed, sorted and
/// deduplicated, joined by a newline; absent when none has one), `doc` (the first that has one), `attrs` (sorted),
/// `names`, `test_id` (the first) and every place, sorted.
pub fn item_facts(definitions: impl IntoIterator<Item = Definition>) -> Vec<Fact> {
    let mut by_id: BTreeMap<String, Vec<Definition>> = BTreeMap::new();
    for definition in definitions {
        by_id
            .entry(definition.id.to_string())
            .or_default()
            .push(definition);
    }
    by_id
        .into_iter()
        .map(|(id, mut definitions)| {
            definitions.sort_by(|a, b| (&a.file, &a.at).cmp(&(&b.file, &b.at)));
            let first = &definitions[0];
            let mut kinds: Vec<ItemKind> = definitions.iter().map(|d| d.kind).collect();
            kinds.sort();
            kinds.dedup();
            let visibility = definitions.iter().map(|d| d.visibility).max();
            let mut fact = Fact::new("item", id.as_str())
                .title(first.id.name())
                .field("package", first.package.to_string())
                .field("file", first.file.as_str())
                .field("kind", kinds.iter().map(|k| k.as_str()).collect::<Vec<_>>())
                .field(
                    "visibility",
                    visibility.unwrap_or(Visibility::Private).as_str(),
                );
            if let Some(digest) = signature(&definitions) {
                fact = fact.field("signature", digest);
            }
            if let Some(doc) = definitions.iter().find_map(|d| d.doc.as_deref()) {
                fact = fact.field("doc", doc);
            }
            let mut attrs: Vec<&str> = definitions
                .iter()
                .flat_map(|d| d.attrs.iter().map(String::as_str))
                .collect();
            attrs.sort();
            attrs.dedup();
            if !attrs.is_empty() {
                fact = fact.field("attrs", attrs);
            }
            if let Some(test_id) = definitions.iter().find_map(|d| d.test_id.as_ref()) {
                fact = fact.field("test_id", test_id.to_string());
            }
            let mut names = Names::new();
            let mut at = Vec::new();
            for definition in definitions {
                join_names(&mut names, definition.names);
                at.push(definition.at);
            }
            placed(named(fact, names), at)
        })
        .collect()
}

/// The digest of the definitions' signature texts (see [`item_facts`]).
fn signature(definitions: &[Definition]) -> Option<String> {
    let mut texts: Vec<String> = definitions
        .iter()
        .filter_map(|d| d.signature.as_deref())
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    if texts.is_empty() {
        return None;
    }
    texts.sort();
    texts.dedup();
    let digest = Sha256::digest(texts.join("\n").as_bytes());
    Some(format!("sha256:{}", hex(&digest)))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A target of a cargo package: its kind, its name and its root source file, project-relative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub kind: TargetKind,
    pub name: String,
    pub root: String,
}

impl Target {
    /// As `package.targets` writes it: `<kind>:<name>:<root>`.
    pub fn written(&self) -> String {
        format!("{}:{}:{}", self.kind.word(), self.name, self.root)
    }

    /// The target `package.targets` writes as `text` ([`Target::written`]); its root may hold `:`, its name not.
    pub fn read(text: &str) -> Result<Target, IdError> {
        match text.splitn(3, ':').collect::<Vec<_>>()[..] {
            [kind, name, root] if !name.is_empty() && !root.is_empty() => Ok(Target {
                kind: TargetKind::cargo(kind)?,
                name: name.to_string(),
                root: root.to_string(),
            }),
            _ => Err(IdError(format!(
                "'{text}' is no target: <kind>:<name>:<root>"
            ))),
        }
    }
}

/// A cargo or npm package, as a manifest pack read it; its name is its id's.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub id: PackageId,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub rust_version: Option<String>,
    /// `<engine> <range>`, from a `package.json`'s `engines`.
    pub engines: Vec<String>,
    pub features: Vec<String>,
    pub targets: Vec<Target>,
    /// Its directory, project-relative, `""` at the root.
    pub path: String,
    /// The project's own packages it depends on.
    pub depends_on: Vec<PackageId>,
    pub at: Option<FactLocation>,
}

impl Package {
    /// Its `package` fact: titled by its name, a field per value it has, `depends_on` by purl (ADR 0347).
    pub fn fact(&self) -> Fact {
        let mut fact = Fact::new("package", self.id.to_string())
            .title(self.id.name())
            .field("name", self.id.name())
            .field("path", self.path.as_str());
        for (field, value) in [
            ("version", &self.version),
            ("edition", &self.edition),
            ("rust_version", &self.rust_version),
        ] {
            if let Some(value) = value {
                fact = fact.field(field, value.as_str());
            }
        }
        let targets: Vec<String> = self.targets.iter().map(Target::written).collect();
        for (field, values) in [
            ("engines", &self.engines),
            ("features", &self.features),
            ("targets", &targets),
        ] {
            if !values.is_empty() {
                fact = fact.field(field, values.clone());
            }
        }
        if !self.depends_on.is_empty() {
            let mut depends: Vec<String> = self.depends_on.iter().map(|d| d.to_string()).collect();
            depends.sort();
            depends.dedup();
            fact = fact.field("depends_on", depends);
        }
        match &self.at {
            Some(at) => fact.at(at.clone()),
            None => fact,
        }
    }
}

/// `into` joined with `names`: each name's places, sorted and deduplicated.
fn join_names(into: &mut Names, names: Names) {
    for (name, places) in names {
        let kept = into.entry(name).or_default();
        kept.extend(places);
        kept.sort();
        kept.dedup();
    }
}

/// `fact` naming each of `names` in its field `names`, at each place it is written.
fn named(mut fact: Fact, names: Names) -> Fact {
    for (name, places) in names {
        if places.is_empty() {
            fact = fact.unplaced_name("names", &name);
        } else {
            for place in places {
                fact = fact.name("names", &name, place);
            }
        }
    }
    fact
}

/// `fact` at each of `places`, sorted and deduplicated.
fn placed(mut fact: Fact, mut places: Vec<FactLocation>) -> Fact {
    places.sort();
    places.dedup();
    for place in places {
        fact = fact.at(place);
    }
    fact
}
