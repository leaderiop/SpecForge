//! A collector's facts (ADR 0327): the store a collect writes, the key it files it under, and what the host admits;
//! then the stores a session serves (D5): each one `specforge-harness` records, read as one file of read-only
//! entities, W191 for one it does not record and W192 for one that does not read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, SourceRoot, SourceSpan, Sym, codes};
use specforge_graph::field_form::spell;
use specforge_graph::{FieldForm, FieldForms, Reading};
use specforge_parser::{
    CURRENT_FORMAT_VERSION, Entity, EntityId, EntityKind, FieldEntry, FieldMap, FieldValue,
    ListItem, SpecFile, Written,
};
use specforge_protocol_types::{Fact, FactStore};
use specforge_registry::{Disk, EntitySnapshot, KindRegistry};
pub use specforge_registry::{FACTS_DIR, filed, is_fact_store};

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
    pub facts: Vec<Fact>,
}

impl Store {
    pub const VERSION: u32 = 1;

    /// The store of `facts`, sorted by kind and id.
    pub fn new(collector: &str, extension: &str, key: &str, mut facts: Vec<Fact>) -> Self {
        facts.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        Store {
            store: Self::VERSION,
            collector: collector.to_string(),
            extension: extension.to_string(),
            key: key.to_string(),
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
        Ok(status) if status.dirty => (format!("{}-dirty", sha.as_str()), Some(KeyReason::Dirty)),
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

/// Why the host refuses `facts` (E116, ADR 0327 Decision 6), each a reason: a kind that is no loaded collected kind,
/// an id that is empty or names none of its kind's schemes, an id the answer repeats. None: every fact is admitted.
pub fn admit(facts: &[Fact], kinds: &KindRegistry) -> Result<(), Vec<String>> {
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

/// Read the fact store `key` of `sources` through `disk` (ADR 0327 D5) against the harness ledger `recorded` (read
/// through `disk` when `None`): its text when a `facts:` line records exactly its collector, key and digest; W191
/// when no line does; W192 when it is not text; `Gone` when it is not there. Its facts are parsed once, by
/// [`served`].
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
    let Some((collector, filed_under)) = filed(key) else {
        return Read::Unreadable(unreadable(
            key,
            "it is not named .specforge/facts/<key>/<collector>.json",
        ));
    };
    let read_here;
    let recorded = match recorded {
        Some(recorded) => recorded,
        None => {
            read_here = self::recorded(disk, sources.base());
            &read_here
        }
    };
    if !recorded.contains(&Line::facts(collector, filed_under, &bytes)) {
        return Read::Unreadable(
            Diagnostic::new(
                codes::W191,
                format!(
                    "fact store '{key}' is not the one specforge-harness records for collector '{collector}' \
                     (collected on another commit or branch, or changed since); its facts are not served"
                ),
            )
            .with_suggestion("run `specforge collect`"),
        );
    }
    match String::from_utf8(bytes) {
        Ok(text) => Read::Text(text),
        Err(e) => Read::Unreadable(unreadable(key, e)),
    }
}

/// The store `text` of the fact store `key` as one file of read-only entities (D5): each fact an entity of its kind,
/// id, title and fields, spanned at the file's first character. A field's value is read from its JSON by the field's
/// form in `forms`, as a `.spec` file writes it (ADR 0234): a string a word field holds bare is the bare word (an
/// enum's `passed`, a reference's id), any other string text; an integer, a bool; an array of strings a list whose
/// items are written as the field writes each. W192 when the text is no store this specforge reads.
pub(crate) fn served(key: &str, text: &str, forms: &FieldForms) -> Result<SpecFile, Diagnostic> {
    let store = serde_json::from_str::<Store>(text).map_err(|e| unreadable(key, e))?;
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
    let file = Sym::new(key);
    let span = SourceSpan {
        file,
        start_line: 1,
        start_col: 1,
        end_line: 1,
        end_col: 1,
    };
    Ok(SpecFile {
        path: file,
        imports: Vec::new(),
        entities: store
            .facts
            .into_iter()
            .map(|fact| entity(fact, &span, forms))
            .collect(),
        errors: Vec::new(),
        format_version: CURRENT_FORMAT_VERSION,
        format_diagnostics: Vec::new(),
    })
}

/// One fact as an entity spanned at `span`, its values read by `forms`: titled by its id when it gives no title.
fn entity(fact: Fact, span: &SourceSpan, forms: &FieldForms) -> Entity {
    let mut fields = FieldMap::new();
    for (name, value) in fact.fields {
        let form = match forms.reading(&fact.kind, &name) {
            Reading::Declared(form) => Some(form),
            Reading::Member | Reading::Undeclared => None,
        };
        if let Some(value) = field_value(value, form, span) {
            // Spanned as its list items are, so a single reference (`test_run.test`) is a written token too.
            fields.push_entry(FieldEntry {
                key: Sym::new(&name),
                value,
                annotations: Vec::new(),
                value_span: Some(span.clone()),
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
    }
}

/// A fact's JSON value as a field value of `form` (none for an undeclared field: text, items quoted); `None` for a
/// form no field holds (an object, a fraction, a null).
fn field_value(
    value: serde_json::Value,
    form: Option<&FieldForm>,
    span: &SourceSpan,
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
                        text,
                        span: span.clone(),
                    }),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Value::Null | Value::Object(_) => return None,
    })
}

/// The fact stores `entities` holds facts of, by store file (D6): each one's collector and key, the schemes its
/// facts' ids are in, and how many facts it holds. What a pass is sent as `fact_stores`.
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
        if let Some(scheme) = specforge_common::scheme::of(&record.id)
            && !store.schemes.iter().any(|s| s == scheme)
        {
            store.schemes.push(scheme.to_string());
            store.schemes.sort();
        }
    }
    stores.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_is_written_canonically_whatever_order_its_facts_came_in() {
        let a = Fact::new("test", "js:b#2").field("z", 1).field("a", "x");
        let b = Fact::new("test", "js:a#1").title("one");
        let one = Store::new("node", "@t/node", "k", vec![a.clone(), b.clone()]);
        let two = Store::new("node", "@t/node", "k", vec![b, a]);

        assert_eq!(one.bytes(), two.bytes());
        let text = String::from_utf8(one.bytes()).unwrap();
        assert!(
            text.ends_with("}\n") && text.contains("\n  \"store\": 1,"),
            "{text}"
        );
        assert!(text.find("js:a#1").unwrap() < text.find("js:b#2").unwrap());
        assert!(text.find("\"a\": \"x\"").unwrap() < text.find("\"z\": 1").unwrap());
        assert_eq!(Store::path("node", "k"), ".specforge/facts/k/node.json");
    }

    use specforge_common::Exclude;
    use specforge_registry::MemoryDisk;
    use std::collections::HashMap;

    const KEY: &str = ".specforge/facts/abc/node.json";

    fn root() -> SourceRoot {
        SourceRoot::new("/p".into(), "/p/spec".into(), Exclude::default())
    }

    fn two() -> Vec<u8> {
        let facts = vec![
            Fact::new("test", "js:a.test.js#adds")
                .title("adds")
                .field("file", "a.test.js"),
            Fact::new("test", "js:a.test.js#subtracts"),
        ];
        Store::new("node", "@t/node", "abc", facts).bytes()
    }

    /// The ledger recording `bytes` as node's store under `abc`.
    fn recording(bytes: &[u8]) -> Vec<u8> {
        let mut ledger = Ledger::default();
        ledger.insert(Line::facts("node", "abc", bytes));
        ledger.canonical().into_bytes()
    }

    fn disk(store: &[u8], ledger: Option<Vec<u8>>) -> MemoryDisk {
        let disk = MemoryDisk::new().with_file(format!("/p/{KEY}"), store);
        match ledger {
            Some(ledger) => disk.with_file("/p/specforge-harness", &ledger),
            None => disk,
        }
    }

    fn diagnostic(read: Read) -> Diagnostic {
        match read {
            Read::Unreadable(diagnostic) => diagnostic,
            Read::Text(_) => panic!("served"),
            Read::Gone => panic!("gone"),
        }
    }

    #[specforge_test_macros::test(
        behavior = "build_in_memory_graph",
        verify = "a fact store specforge-harness records is served as read-only entities; one it does not is W191, one that does not read W192"
    )]
    fn a_recorded_fact_store_is_served_as_one_spec_file() {
        let bytes = two();
        let Read::Text(text) = read(&disk(&bytes, Some(recording(&bytes))), &root(), KEY, None)
        else {
            panic!("a recorded store is served");
        };

        let file = served(KEY, &text, &FieldForms::default()).unwrap();

        assert_eq!(file.path.as_str(), KEY);
        let ids: Vec<&str> = file.entities.iter().map(|e| e.id.raw.as_str()).collect();
        assert_eq!(ids, ["js:a.test.js#adds", "js:a.test.js#subtracts"]);
        let adds = &file.entities[0];
        assert_eq!(
            (adds.kind.raw.as_str(), adds.title.as_deref()),
            ("test", Some("adds"))
        );
        assert_eq!((adds.span.start_line, adds.name_span.is_none()), (1, true));
        assert!(file.errors.is_empty());
    }

    #[test]
    fn an_unrecorded_fact_store_is_w191_and_not_served() {
        let w191 = diagnostic(read(&disk(&two(), None), &root(), KEY, None));

        assert_eq!(w191.code, "W191");
        assert_eq!(
            w191.message,
            "fact store '.specforge/facts/abc/node.json' is not the one specforge-harness records for collector \
             'node' (collected on another commit or branch, or changed since); its facts are not served"
        );
        assert_eq!(w191.suggestion.as_deref(), Some("run `specforge collect`"));
    }

    #[test]
    fn a_fact_store_whose_digest_differs_is_w191() {
        let other = Store::new("node", "@t/node", "abc", Vec::new()).bytes();

        let w191 = diagnostic(read(
            &disk(&two(), Some(recording(&other))),
            &root(),
            KEY,
            None,
        ));

        assert_eq!(w191.code, "W191");
    }

    #[test]
    fn an_unreadable_fact_store_is_w192() {
        // A recorded store is read as it is, and parsed once, when it is served: no form of it is dropped silently.
        let not_json = served(KEY, "{ not json", &FieldForms::default()).unwrap_err();
        let newer = r#"{"store": 2, "collector": "node", "extension": "@t/node", "key": "abc", "facts": []}"#;
        let newer = served(KEY, newer, &FieldForms::default()).unwrap_err();
        let Read::Text(_) = read(
            &disk(b"{ not json", Some(recording(b"{ not json"))),
            &root(),
            KEY,
            None,
        ) else {
            panic!("a recorded store reads as its text");
        };

        for w192 in [&not_json, &newer] {
            assert_eq!(w192.code, "W192");
            assert!(
                w192.message
                    .starts_with("fact store '.specforge/facts/abc/node.json' cannot be read: "),
                "{}",
                w192.message
            );
        }
        assert!(
            newer
                .message
                .ends_with("it is store version 2, and this specforge reads version 1")
        );
        assert!(matches!(
            read(&MemoryDisk::new(), &root(), KEY, None),
            Read::Gone
        ));
    }

    /// `test`'s `status` declared an enum and its `names` a reference list.
    fn declared() -> FieldForms {
        use specforge_protocol_types::FieldType;
        let field = |name: &str| ("test".to_string(), name.to_string());
        let status = FieldForm::new(FieldType::Enum, vec!["passed".into(), "failed".into()]);
        let names = FieldForm::new(FieldType::ReferenceList, Vec::new());
        FieldForms::new(
            HashMap::from([(field("status"), status), (field("names"), names)]),
            BTreeSet::new(),
            BTreeSet::from(["test".to_string()]),
        )
    }

    #[specforge_test_macros::test(
        behavior = "build_in_memory_graph",
        verify = "a fact's value is read by its field's form, as a spec file writes it: a word field's word is bare"
    )]
    fn a_facts_strings_are_read_by_their_fields_form() {
        let fact = Fact::new("test", "js:a#b")
            .field("status", "passed")
            .field("file", "a.test.js")
            .field("names", serde_json::json!(["v1/TR1", "a b"]));
        let text =
            String::from_utf8(Store::new("node", "@t/node", "abc", vec![fact]).bytes()).unwrap();

        let file = served(KEY, &text, &declared()).unwrap();

        let fields = &file.entities[0].fields;
        assert_eq!(
            fields.get("status"),
            Some(&FieldValue::Identifier("passed".into()))
        );
        assert_eq!(
            fields.get("file"),
            Some(&FieldValue::String("a.test.js".into())),
            "an undeclared field's string stays text"
        );
        let Some(FieldValue::List(names)) = fields.get("names") else {
            panic!("an array of strings is a list");
        };
        let names: Vec<(&str, Written)> =
            names.iter().map(|n| (n.text.as_str(), n.written)).collect();
        assert_eq!(names, [("v1/TR1", Written::Bare), ("a b", Written::Quoted)]);
    }

    #[test]
    fn a_facts_field_values_become_field_values_by_their_form() {
        let fact = Fact::new("test", "js:a#b")
            .field("file", "a.test.js")
            .field("runs", 3)
            .field("flaky", true)
            .field("tags", serde_json::json!(["slow", "js:a#c"]))
            .field("meta", serde_json::json!({"x": 1}))
            .field("ratio", 0.5);
        let text =
            String::from_utf8(Store::new("node", "@t/node", "abc", vec![fact]).bytes()).unwrap();

        let file = served(KEY, &text, &FieldForms::default()).unwrap();

        let fields = &file.entities[0].fields;
        assert_eq!(
            fields.get("file"),
            Some(&FieldValue::String("a.test.js".into()))
        );
        assert_eq!(fields.get("runs"), Some(&FieldValue::Integer(3)));
        assert_eq!(fields.get("flaky"), Some(&FieldValue::Boolean(true)));
        let Some(FieldValue::List(tags)) = fields.get("tags") else {
            panic!("an array of strings is a list");
        };
        let tags: Vec<(&str, Written)> =
            tags.iter().map(|t| (t.text.as_str(), t.written)).collect();
        assert_eq!(
            tags,
            [("slow", Written::Quoted), ("js:a#c", Written::Quoted)]
        );
        assert_eq!((fields.get("meta"), fields.get("ratio")), (None, None));
    }
}
