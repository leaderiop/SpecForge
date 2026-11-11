//! A collector's facts (ADR 0327): the store a collect writes, the key it files it under, and what the host admits;
//! then the stores a session serves (D5): each one `specforge-harness` records, read as one file of read-only
//! entities, W191 for one it does not record and W192 for one that does not read; one a rename marked stale is
//! served with the names the rename wrote and its places shifted by what it wrote, W197 ([`stale`]). [`store`] is
//! the one reader of a store's text; [`Placed`] the one judge of whether a source file is the text a served store's
//! places are in.

mod stale;

pub use stale::Renames;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use specforge_common::facts::{FACTS_DIR, filed, is_fact_store};
use specforge_common::{Diagnostic, SourceRoot, SourceSpan, Sym, codes};
use specforge_graph::field_form::spell;
use specforge_graph::{FieldForm, FieldForms, Reading};
use specforge_parser::{
    CURRENT_FORMAT_VERSION, Entity, EntityId, EntityKind, FieldEntry, FieldMap, FieldValue,
    ListItem, SpecFile, Written,
};
use specforge_protocol_types::{Fact, FactLocation, FactStore};
use specforge_registry::{Disk, EntitySnapshot, KindRegistry};

use crate::Environment;
use crate::harness::{LEDGER_FILE, Ledger, Line};
use crate::sources::Read;
use specforge_effects::git::Located;

/// One store file, `.specforge/facts/<key>/<collector>.json`: the facts one collector answered, filed under one key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Store {
    /// The store format, [`Store::VERSION`].
    pub store: u32,
    pub collector: String,
    pub extension: String,
    pub key: String,
    /// The digest (`sha256:<hex>`) of each source file the facts' places name, by its key from the project root, as
    /// the host read it when it admitted them (ADR 0346 D2): what a rename checks the file against before it rewrites
    /// a name there.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, String>,
    pub facts: Vec<Fact>,
}

impl Store {
    pub const VERSION: u32 = 2;

    /// The store of `facts`, sorted by kind and id, and the digests of the `sources` their places name.
    pub fn new(
        collector: &str,
        extension: &str,
        key: &str,
        mut facts: Vec<Fact>,
        sources: BTreeMap<String, String>,
    ) -> Self {
        facts.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        Store {
            store: Self::VERSION,
            collector: collector.to_string(),
            extension: extension.to_string(),
            key: key.to_string(),
            sources,
            facts,
        }
    }

    /// The canonical bytes (ADR 0327 D2): two-space JSON, fields in key order, a final newline. Equal facts give
    /// equal bytes.
    pub fn bytes(&self) -> Vec<u8> {
        let mut text = serde_json::to_string_pretty(self).expect("a store serializes");
        text.push('\n');
        text.into_bytes()
    }

    /// The store file of `collector` under `key`, from the project root.
    pub fn path(collector: &str, key: &str) -> String {
        format!("{FACTS_DIR}/{key}/{collector}.json")
    }
}

/// Why a collect's key is not a clean commit's sha (I209).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyReason {
    /// The tree holds a tracked change, a conflict or an untracked file: `<sha>-dirty`.
    Dirty,
    /// The project is in no repository: `worktree`.
    NoRepository,
    /// HEAD names no commit yet: `worktree`.
    UnbornHead,
    /// Git can't be asked, and why: `worktree`.
    NoHistory(String),
}

impl KeyReason {
    /// What I209 says of it.
    pub fn says(&self) -> String {
        match self {
            KeyReason::Dirty => "the tree has changes git does not hold at HEAD".to_string(),
            KeyReason::NoRepository => "the project is in no git repository".to_string(),
            KeyReason::UnbornHead => "HEAD names no commit yet".to_string(),
            KeyReason::NoHistory(why) => format!("git can't be asked ({why})"),
        }
    }
}

/// The key a collect files its facts under (ADR 0327 D3), asked through the history port: HEAD's sha when the tree
/// is clean, `<sha>-dirty` when it is not, `worktree` when there is no commit to name; with why, unless it is clean.
pub fn key(env: &Environment) -> (String, Option<KeyReason>) {
    const WORKTREE: &str = "worktree";
    let Some(history) = env.git() else {
        return (
            WORKTREE.into(),
            Some(KeyReason::NoHistory("no git port is given".into())),
        );
    };
    let place = match env.place(history.as_ref()) {
        Located::Repo(place) => place,
        Located::NotARepository => return (WORKTREE.into(), Some(KeyReason::NoRepository)),
        Located::Unavailable(why) => return (WORKTREE.into(), Some(KeyReason::NoHistory(why))),
    };
    let sha = match history.commit(&place, "HEAD") {
        Ok(Some(sha)) => sha,
        Ok(None) => return (WORKTREE.into(), Some(KeyReason::UnbornHead)),
        Err(error) => return (WORKTREE.into(), Some(KeyReason::NoHistory(error.0))),
    };
    match history.status(&place) {
        Ok(status) if status.dirty() => (format!("{}-dirty", sha.as_str()), Some(KeyReason::Dirty)),
        Ok(_) => (sha.as_str().to_string(), None),
        Err(error) => (WORKTREE.into(), Some(KeyReason::NoHistory(error.0))),
    }
}

/// The fact stores under the project root `root` now, as source keys, sorted: every
/// `.specforge/facts/<key>/<collector>.json`, listed through `disk`.
pub fn keys(disk: &dyn Disk, root: &Path) -> Vec<String> {
    let dir = root.join(FACTS_DIR);
    let mut keys = Vec::new();
    for key in disk.list(&dir).unwrap_or_default() {
        for file in disk.list(&dir.join(&key)).unwrap_or_default() {
            let store = format!("{FACTS_DIR}/{key}/{file}");
            if is_fact_store(&store) {
                keys.push(store);
            }
        }
    }
    keys.sort();
    keys
}

/// The stores of `collector` under `root` filed under a key other than `key`, listed through `disk`: what a collect
/// that files its facts under `key` replaces (ADR 0327 D4).
pub fn superseded(disk: &dyn Disk, root: &Path, collector: &str, key: &str) -> Vec<String> {
    keys(disk, root)
        .into_iter()
        .filter(|store| filed(store).is_some_and(|(by, under)| by == collector && under != key))
        .collect()
}

/// Why the host refuses `facts` a collector that `answers` those kinds answered (E116, ADR 0327 D5, ADR 0345
/// D4), each a reason: a kind that is no loaded collected kind, or one the collector does not list in `answers`, an id
/// that is empty or names none of its kind's schemes, an id the answer repeats. None: every fact is admitted.
pub fn admit(facts: &[Fact], kinds: &KindRegistry, answers: &[String]) -> Result<(), Vec<String>> {
    let mut reasons = Vec::new();
    let mut seen = BTreeSet::new();
    for fact in facts {
        let Some(kind) = kinds.collected(&fact.kind) else {
            reasons.push(format!(
                "fact '{}' is of the kind '{}', which no loaded extension declares a collector writes",
                fact.id, fact.kind
            ));
            continue;
        };
        if !answers.contains(&fact.kind) {
            reasons.push(format!(
                "fact '{}' is of the kind '{}', which the collector does not list in its answers",
                fact.id, fact.kind
            ));
            continue;
        }
        if fact.id.is_empty() {
            reasons.push(format!(
                "a fact of the kind '{}' has an empty id",
                fact.kind
            ));
            continue;
        }
        if !specforge_common::scheme::of(&fact.id)
            .is_some_and(|scheme| kind.schemes().iter().any(|s| s == scheme))
        {
            reasons.push(format!(
                "fact '{}' names none of the schemes of the kind '{}' ({})",
                fact.id,
                fact.kind,
                kind.schemes().join(", ")
            ));
            continue;
        }
        if !seen.insert((fact.kind.as_str(), fact.id.as_str())) {
            reasons.push(format!("fact '{}' is answered twice", fact.id));
        }
    }
    if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons)
    }
}

/// W192: the fact store `key` cannot be read as a store, and why.
fn unreadable(key: &str, why: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        codes::W192,
        format!("fact store '{key}' cannot be read: {why}"),
    )
    .with_suggestion("run `specforge collect` to write it again")
}

/// The harness ledger at the project root `root`, read through `disk`: what a store is checked against (empty when
/// there is none). A session reads it once for all its stores.
pub(crate) fn recorded(disk: &dyn Disk, root: &Path) -> Ledger {
    let bytes = disk.read(&root.join(LEDGER_FILE)).unwrap_or_default();
    Ledger::parse(&String::from_utf8_lossy(&bytes)).ledger
}

/// Each path a rename moved and the path it is now, from the ledger's `new moved:old` records (a move carries an
/// earlier record of what it moves, so each is the path now).
pub(crate) fn moves(ledger: &Ledger) -> BTreeMap<String, String> {
    ledger
        .lines()
        .filter_map(|line| Some((line.moved_from()?.to_string(), line.subject.clone())))
        .collect()
}

/// Read the fact store `key` of `sources` through `disk` (ADR 0327 D5) against the harness ledger `recorded` (read
/// through `disk` when `None`): its served text ([`served_text`]); `Gone` when it is not there. Its facts are parsed
/// once, by [`store`].
pub(crate) fn read(
    disk: &dyn Disk,
    sources: &SourceRoot,
    key: &str,
    recorded: Option<&Ledger>,
) -> Read {
    let bytes = match disk.read(&sources.path(key)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Read::Gone,
        Err(e) => return Read::Unreadable(unreadable(key, e)),
    };
    let read_here;
    let recorded = match recorded {
        Some(recorded) => recorded,
        None => {
            read_here = self::recorded(disk, sources.base());
            &read_here
        }
    };
    match served_text(key, bytes, recorded) {
        Ok(text) => Read::Text(text),
        Err(diagnostic) => Read::Unreadable(diagnostic),
    }
}

/// The text a session serves of the fact store `key` whose file holds `bytes`, against the harness ledger
/// `recorded`: the bytes, when a `facts:` line records exactly its collector, key and digest, and the renames its
/// `stale:` lines mark exactly those bytes with after them (so a ledger change re-serves it); W191 when no line
/// records them; W192 when they are not text or the key is no store's.
fn served_text(key: &str, bytes: Vec<u8>, recorded: &Ledger) -> Result<String, Diagnostic> {
    let Some((collector, filed_under)) = filed(key) else {
        return Err(unreadable(
            key,
            "it is not named .specforge/facts/<key>/<collector>.json",
        ));
    };
    if !recorded.contains(&Line::facts(collector, filed_under, &bytes)) {
        return Err(Diagnostic::new(
            codes::W191,
            format!(
                "fact store '{key}' is not the one specforge-harness records for collector '{collector}' \
                 (collected on another commit or branch, or changed since); its facts are not served"
            ),
        )
        .with_suggestion("run `specforge collect`"));
    }
    let renames = stale::renames(recorded, collector, filed_under, &bytes);
    let text = String::from_utf8(bytes).map_err(|e| unreadable(key, e))?;
    Ok(stale::carried(text, &renames))
}

/// The fact stores a session serves under the project root `root`, read through `disk`: each store the harness ledger
/// records exactly as it is on disk, in the ledger's order. A store the ledger does not record as it is (W191), or
/// that is no store this specforge reads (W192), is none of them.
pub fn recorded_stores(disk: &dyn Disk, root: &Path) -> Vec<Store> {
    let ledger = recorded(disk, root);
    ledger
        .lines()
        .filter_map(Line::facts_of)
        .filter_map(|(collector, key)| {
            let path = Store::path(collector, key);
            let bytes = disk.read(&root.join(&path)).ok()?;
            let text = served_text(&path, bytes, &ledger).ok()?;
            store(&path, &text).ok().map(|served| served.store)
        })
        .collect()
}

/// The facts of `kinds` in the [`recorded_stores`] under the project root `root`, sorted by kind and id (ADR 0345
/// D3): what a collector's `facts` input is served of a kind no collector of its collect answers.
pub fn recorded_facts(disk: &dyn Disk, root: &Path, kinds: &[&str]) -> Vec<Fact> {
    if kinds.is_empty() {
        return Vec::new();
    }
    let mut facts: Vec<Fact> = recorded_stores(disk, root)
        .into_iter()
        .flat_map(|store| store.facts)
        .filter(|fact| kinds.contains(&fact.kind.as_str()))
        .collect();
    facts.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
    facts
}

/// A store as a session serves it ([`store`]).
#[derive(Debug)]
pub struct Served<'t> {
    /// The store, each name a rename marked it stale with read as the rename wrote it, and every place in a file a
    /// rename rewrote shifted by what it wrote.
    pub store: Store,
    /// The collector's bytes the store was read from: what a `stale:` line digests.
    pub bytes: &'t str,
    /// W197 when a rename marked it stale.
    pub stale: Option<Diagnostic>,
    /// The renames its stale lines record: field → the name the store holds → the name written now.
    pub renames: Renames,
    /// Each source file its facts are placed in, by its key from the project root: what says whether a text is the
    /// one its places are in.
    pub placed: BTreeMap<String, Placed>,
}

impl Served<'_> {
    /// The name the store holds in `field` that is served as `now`: the name a rename wrote it from, else `now`.
    pub fn collected<'s>(&'s self, field: &str, now: &'s str) -> &'s str {
        self.renames
            .get(field)
            .and_then(|names| names.iter().find(|(_, written)| *written == now))
            .map_or(now, |(held, _)| held.as_str())
    }
}

/// What a served store says of one source file its facts are placed in (ADR 0346 D2, D7): the digest the host
/// recorded of it at the collect, and each place a rename rewrote since, as it is in the text the renames left. A
/// text is the one the served places are in exactly when undoing those rewrites gives the text collected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Placed {
    /// `sha256:<hex>` of the file the collect read.
    pub digest: String,
    /// What the renames wrote, in the order of the text.
    pub respelled: Vec<Respelled>,
}

/// One place a rename rewrote: where it is in the text the renames left, the name the collector read there, and the
/// name written now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Respelled {
    pub at: FactLocation,
    pub written: String,
    pub now: String,
}

impl Placed {
    /// Whether `text` is the text the served places are in: each respelled place spells its name written now, and
    /// putting back the name collected at each gives the bytes whose digest the collect recorded. Exact: nothing
    /// else in the file may have changed.
    pub fn holds(&self, text: &[u8]) -> bool {
        let mut ranges = Vec::with_capacity(self.respelled.len());
        for respelled in &self.respelled {
            let Some(range) = byte_range(text, &respelled.at) else {
                return false;
            };
            if text.get(range.clone()) != Some(respelled.now.as_bytes()) {
                return false;
            }
            ranges.push((range, respelled.written.as_bytes()));
        }
        ranges.sort_by_key(|(range, _)| range.start);
        if ranges
            .windows(2)
            .any(|pair| pair[0].0.end > pair[1].0.start)
        {
            return false;
        }
        let mut collected = text.to_vec();
        for (range, written) in ranges.into_iter().rev() {
            collected.splice(range, written.iter().copied());
        }
        format!("sha256:{}", specforge_package::Sha256::of(&collected)) == self.digest
    }
}

/// The bytes of `text` the span `at` covers (1-based lines, 1-based byte columns, end exclusive); `None` when it lies
/// outside the text or a column passes its line's end.
fn byte_range(text: &[u8], at: &FactLocation) -> Option<std::ops::Range<usize>> {
    let offset = |line: usize, col: usize| -> Option<usize> {
        let start = if line == 1 {
            0
        } else {
            text.iter()
                .enumerate()
                .filter(|(_, b)| **b == b'\n')
                .nth(line.checked_sub(2)?)?
                .0
                + 1
        };
        let end = text[start..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(text.len(), |n| start + n);
        let at = start + col.checked_sub(1)?;
        (at <= end).then_some(at)
    };
    let (start, end) = (
        offset(at.start_line, at.start_col)?,
        offset(at.end_line, at.end_col)?,
    );
    (start <= end).then_some(start..end)
}

/// The served text `text` of the fact store `key` ([`read`]) read: the one reader of a store. Its facts' names a rename
/// marked it stale with are read as the rename wrote them and its places in the files it rewrote shifted by what it
/// wrote, with W197 (ADR 0346 D7). W192 when the text is no store this specforge reads.
pub fn store<'t>(key: &str, text: &'t str) -> Result<Served<'t>, Diagnostic> {
    let (mut store, bytes, renames) = stale::split(text).map_err(|e| unreadable(key, e))?;
    if store.store != Store::VERSION {
        return Err(unreadable(
            key,
            format!(
                "it is store version {}, and this specforge reads version {}",
                store.store,
                Store::VERSION
            ),
        ));
    }
    let (stale, mut respelled) = stale::follow(key, &mut store, &renames);
    let placed = store
        .sources
        .iter()
        .map(|(file, digest)| {
            let placed = Placed {
                digest: digest.clone(),
                respelled: respelled.remove(file).unwrap_or_default(),
            };
            (file.clone(), placed)
        })
        .collect();
    Ok(Served {
        store,
        bytes,
        stale,
        renames,
        placed,
    })
}

/// The served text `text` of the fact store `key` as one file of read-only entities (D5): each fact an entity of its
/// kind, id, title and fields, spanned at the file's first character (what identifies a fact) and `located` where its
/// collector says it is written (ADR 0346 D3). A field's value is read from its JSON by the field's form in `forms`,
/// as a `.spec` file writes it (ADR 0234): a string a word field holds bare is the bare word (an enum's `passed`, a
/// reference's id), any other string text; an integer, a bool; an array of strings a list whose items are written as
/// the field writes each. Each value is spanned where its name is written (`named_at`), else where the fact is, else
/// at the store. W192 when the text is no store this specforge reads; W197, in the file's format diagnostics, when a
/// rename marked it stale. Beside it, what the store says of each source file its places name ([`Placed`]).
pub(crate) fn served(
    key: &str,
    text: &str,
    forms: &FieldForms,
) -> Result<(SpecFile, BTreeMap<String, Placed>), Diagnostic> {
    let Served {
        store,
        stale,
        placed,
        ..
    } = store(key, text)?;
    let file = Sym::new(key);
    let span = SourceSpan {
        file,
        start_line: 1,
        start_col: 1,
        end_line: 1,
        end_col: 1,
    };
    let file = SpecFile {
        path: file,
        imports: Vec::new(),
        entities: store
            .facts
            .into_iter()
            .map(|fact| entity(fact, &span, forms))
            .collect(),
        errors: Vec::new(),
        format_version: CURRENT_FORMAT_VERSION,
        format_diagnostics: stale.into_iter().collect(),
    };
    Ok((file, placed))
}

/// One fact as an entity spanned at `span` (its store's) and located at its places, its values read by `forms` and
/// spanned at their names' places: titled by its id when it gives no title.
fn entity(fact: Fact, span: &SourceSpan, forms: &FieldForms) -> Entity {
    let located: Vec<SourceSpan> = fact.at.iter().map(source_span).collect();
    // Where a value no name place is recorded for is placed: the fact's first place, else its store.
    let fallback = located.first().unwrap_or(span).clone();
    let mut fields = FieldMap::new();
    for (name, value) in fact.fields {
        let form = match forms.reading(&fact.kind, &name) {
            Reading::Declared(form) => Some(form),
            Reading::Member | Reading::Undeclared => None,
        };
        let named = fact.named_at.get(&name);
        let place = |text: &str| {
            named
                .and_then(|ids| ids.get(text))
                .and_then(|places| places.first())
                .map_or_else(|| fallback.clone(), source_span)
        };
        let value_span = match &value {
            serde_json::Value::String(text) => place(text),
            _ => fallback.clone(),
        };
        if let Some(value) = field_value(value, form, &place) {
            // Spanned as its list items are, so a single reference (`test_run.test`) is a written token too.
            fields.push_entry(FieldEntry {
                key: Sym::new(&name),
                value,
                annotations: Vec::new(),
                value_span: Some(value_span),
                value_lexemes: Vec::new(),
            });
        }
    }
    Entity {
        kind: EntityKind {
            raw: Sym::new(&fact.kind),
        },
        id: EntityId {
            raw: Sym::new(&fact.id),
        },
        // A block owes a title (E076); a fact's tool may give none, so its id stands in.
        title: fact.title.or_else(|| Some(fact.id.clone())),
        title_span: None,
        fields,
        span: span.clone(),
        name_span: None,
        methods: Vec::new(),
        obligations: Vec::new(),
        obligation_spans: Vec::new(),
        verdicts: Vec::new(),
        doc_span: None,
        body_comments: Vec::new(),
        trailing_comments: Vec::new(),
        value_comments: Vec::new(),
        located,
    }
}

/// A fact's place as the host spans source: its file a source key, its lines and byte columns as written.
pub fn source_span(place: &FactLocation) -> SourceSpan {
    SourceSpan {
        file: Sym::new(&place.file),
        start_line: place.start_line,
        start_col: place.start_col,
        end_line: place.end_line,
        end_col: place.end_col,
    }
}

/// A fact's JSON value as a field value of `form` (none for an undeclared field: text, items quoted), each list item
/// spanned at `place` of its text; `None` for a form no field holds (an object, a fraction, a null).
fn field_value(
    value: serde_json::Value,
    form: Option<&FieldForm>,
    place: &dyn Fn(&str) -> SourceSpan,
) -> Option<FieldValue> {
    use serde_json::Value;
    let written = |text: &str| form.map_or(Written::Quoted, |form| spell::form(form, text));
    Some(match value {
        Value::String(text) if written(&text) == Written::Bare => FieldValue::Identifier(text),
        Value::String(text) => FieldValue::String(text),
        Value::Bool(b) => FieldValue::Boolean(b),
        Value::Number(n) => FieldValue::Integer(n.as_i64()?),
        Value::Array(items) => FieldValue::List(
            items
                .into_iter()
                .map(|item| match item {
                    Value::String(text) => Some(ListItem {
                        written: written(&text),
                        span: place(&text),
                        text,
                    }),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Value::Null | Value::Object(_) => return None,
    })
}

/// The fact stores `entities` holds facts of, by store file (D6): each one's collector and key, the kinds of its facts
/// by the scheme of their ids (ADR 0347 D9), and how many facts it holds. What a pass is sent as `fact_stores`.
pub fn stores(entities: &EntitySnapshot) -> Vec<FactStore> {
    let mut stores: BTreeMap<&str, FactStore> = BTreeMap::new();
    for (record, _) in entities.iter() {
        let file = record.span.file.as_str();
        let Some((collector, key)) = filed(file) else {
            continue;
        };
        let store = stores.entry(file).or_insert_with(|| FactStore {
            collector: collector.to_string(),
            key: key.to_string(),
            ..FactStore::default()
        });
        store.facts += 1;
        if let Some(scheme) = specforge_common::scheme::of(&record.id) {
            let kinds = store.held.entry(scheme.to_string()).or_default();
            if let Err(at) = kinds.binary_search(&record.kind) {
                kinds.insert(at, record.kind.clone());
            }
        }
    }
    stores.into_values().collect()
}

#[cfg(test)]
mod tests;
