use gsd_kit::dsl::{Field, Kind, Ty, Writer};
use serde_json::json;

use super::*;

fn declared(kind: &str) -> &'static Kind {
    crate::kinds::KINDS
        .iter()
        .find(|k| k.name == kind)
        .unwrap_or_else(|| panic!("{kind} is declared"))
}

/// A prose mark, where it sits, whether the table reads it (every field on its way is authored), and the names of
/// the record that holds it.
type Mark = (String, bool, Vec<&'static str>);

/// Every prose mark of `fields`, at any depth, through computed and harness blocks too.
fn marks(path: &str, fields: &'static [Field], read: bool, out: &mut Vec<Mark>) {
    let record: Vec<_> = fields.iter().map(|f| f.name).collect();
    for field in fields {
        let path = format!("{path}.{}", field.name);
        let read = read && field.writer == Writer::Author;
        if field.prose {
            assert!(
                matches!(field.ty, Ty::Str | Ty::Strs),
                "{path}: prose is a str or strs"
            );
            out.push((path.clone(), read, record.clone()));
        }
        if let Ty::Block(members) | Ty::Blocks(members) = field.ty {
            marks(&path, members, read, out);
        }
    }
}

/// Every prose mark, at any depth, sits on a text or a list of texts the table reads: a mark the table cannot read
/// fails here rather than leave the vocabulary reading nothing. A record two kinds share (a `resolved` block) may
/// sit under another writer's block in one of them, as long as the table reads it in the other.
#[test]
fn every_prose_mark_is_on_a_text_the_table_reads() {
    let mut all = Vec::new();
    for kind in crate::kinds::KINDS {
        marks(kind.name, kind.fields, true, &mut all);
    }
    let read: Vec<_> = all.iter().filter(|m| m.1).map(|m| &m.2).collect();
    let unread: Vec<_> = all
        .iter()
        .filter(|(_, is_read, record)| !is_read && !read.contains(&record))
        .map(|m| &m.0)
        .collect();
    assert!(
        unread.is_empty(),
        "prose marks the table never reads: {unread:?}"
    );
    // The blocker's `resolved` is authored and read; the revision conflict's is the planner's record and is not.
    assert!(fields("blocker").iter().any(|p| p.name == "how"));
    assert!(!fields("revision_conflict").iter().any(|p| p.name == "how"));
}

/// A computed block is the engine's text, never the author's: no member of one, at any depth, is marked prose.
#[test]
fn a_computed_block_carries_no_prose_mark() {
    fn computed(path: &str, fields: &'static [Field], out: &mut Vec<Mark>) {
        for field in fields {
            let path = format!("{path}.{}", field.name);
            if let Ty::Block(members) | Ty::Blocks(members) = field.ty {
                if field.writer == Writer::Computed {
                    marks(&path, members, false, out);
                } else {
                    computed(&path, members, out);
                }
            }
        }
    }
    let mut found = Vec::new();
    for kind in crate::kinds::KINDS {
        computed(kind.name, kind.fields, &mut found);
    }
    assert!(found.is_empty(), "prose in a computed block: {found:?}");
}

/// The table is what the kinds mark: a kind's prose fields, a block list's prose member, and nothing of a term (the
/// glossary) or of `text_en` (another language).
#[test]
fn the_table_reads_the_kinds_marks() {
    let field = |name, list| Prose {
        blocks: Vec::new(),
        name,
        list,
        alone: false,
        fixed: false,
    };
    assert_eq!(
        fields("requirement"),
        [field("text", false), field("reason", false)]
    );
    assert_eq!(
        fields("journey"),
        [Prose {
            blocks: vec![("steps", true)],
            name: "action",
            list: false,
            alone: true,
            fixed: false,
        }]
    );
    let decision = fields("decision");
    assert!(decision.contains(&field("consequences", true)));
    assert!(decision.contains(&Prose {
        blocks: vec![("alternatives", true)],
        name: "why_not",
        list: false,
        alone: false,
        fixed: false,
    }));
    assert!(fields("discussion").contains(&Prose {
        blocks: vec![("checkpoint", false), ("decisions", true)],
        name: "question",
        list: false,
        alone: true,
        fixed: false,
    }));
    assert!(fields("term").is_empty());
    assert!(fields("commit").is_empty(), "a harness-written kind");
    for (kind, prose) in PROSE.iter() {
        assert!(!prose.iter().any(|p| p.name == "text_en"), "{kind}");
    }
}

/// `term.used_in` lists entities of exactly the kinds that write prose, in the kinds' order.
#[test]
fn the_used_in_union_is_the_prose_kinds() {
    let used_in = declared("term")
        .fields
        .iter()
        .find(|f| f.name == "used_in")
        .expect("term.used_in");
    let Ty::RefsAny(kinds) = used_in.ty else {
        panic!("term.used_in is a union list");
    };
    let prose: Vec<&str> = PROSE.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(kinds, prose.as_slice());
}

#[test]
fn a_journey_step_action_is_prose() {
    let journey: PassEntity = serde_json::from_value(json!({
        "id": "add_and_finish", "kind": "journey",
        "fields": { "persona": "owner" },
        "values": { "steps": [
            { "action": "adds a task", "requirement": "TASK-01" },
            { "action": "marks it complete" },
            { "action": "  " },
        ] },
    }))
    .expect("a journey");
    assert_eq!(
        prose(&journey),
        [
            ("step 1".to_string(), "adds a task".to_string()),
            ("step 2".to_string(), "marks it complete".to_string()),
        ]
    );
    let places = prose(&journey);
    assert_eq!(uses(&places, "Complete"), Some("step 2"));
    assert_eq!(uses(&places, "owner"), None);
}

/// A context's code insights and a user setup's `local_dev` are prose: GSD writes each item as code named, then said
/// in the project's language, so the code is masked and the explanation read.
#[test]
fn code_insights_and_local_dev_read_what_they_say_of_the_code() {
    let context: PassEntity = serde_json::from_value(json!({
        "id": "03-CONTEXT", "kind": "context",
        "fields": { "boundary": "Checkout" },
        "values": {
            "reusable_assets": ["`CartStore`: holds the basket"],
            "patterns": ["src/cart/store.ts keeps one state per basket"],
            "integration_points": ["the checkout page posts each order"],
        },
    }))
    .expect("a context");
    let places = prose(&context);
    assert_eq!(uses(&places, "basket"), Some("reusable_assets"));
    assert_eq!(uses(&places, "order"), Some("integration_points"));
    assert_eq!(uses(&places, "CartStore"), None, "a code span is no word");
    assert_eq!(uses(&places, "cart"), None, "a path is no word");
    let setup: PassEntity = serde_json::from_value(json!({
        "id": "stripe", "kind": "user_setup",
        "fields": { "service": "stripe", "why": "payments" },
        "values": { "local_dev": ["Run: stripe listen --forward-to localhost:3000/api/webhooks",
                                  "Use the webhook secret from the CLI output"] },
    }))
    .expect("a user setup");
    assert_eq!(uses(&prose(&setup), "webhook secret"), Some("local_dev"));
}

/// A decision's prose, in its fields' order: its text and lists of texts, and its alternatives' prose members, each
/// named with its row; its `area` is no prose.
#[test]
fn a_decision_reads_its_prose_fields_and_its_alternatives() {
    let decision: PassEntity = serde_json::from_value(json!({
        "id": "v1.0/01/D1", "kind": "decision",
        "fields": { "text": "Store tasks", "area": "storage", "context": "a board",
                    "rationale": "a board needs them" },
        "values": {
            "consequences": ["a file per board", " "],
            "alternatives": [ { "name": "SQLite", "why_not": "a server to run", "pros": ["fast"] } ],
        },
    }))
    .expect("a decision");
    assert_eq!(
        prose(&decision),
        [
            ("text", "Store tasks"),
            ("context", "a board"),
            ("alternative 1 pros", "fast"),
            ("alternative 1 why_not", "a server to run"),
            ("rationale", "a board needs them"),
            ("consequences", "a file per board"),
        ]
        .map(|(place, said)| (place.to_string(), said.to_string()))
    );
    assert_eq!(uses(&prose(&decision), "board"), Some("context"));
    assert_eq!(
        uses(&prose(&decision), "server"),
        Some("alternative 1 why_not")
    );
    assert_eq!(uses(&prose(&decision), "storage"), None);
}

/// A pass input of `entities` (JSON).
fn input(entities: serde_json::Value) -> PassInput {
    serde_json::from_value(json!({ "entities": entities, "edges": [] })).expect("a pass input")
}

/// `(code, entity, message, suggestion)` of each diagnostic.
fn found(input: &PassInput) -> Vec<(String, String, String, String)> {
    check(input)
        .into_iter()
        .map(|d| {
            (
                d.code,
                d.entity.unwrap_or_default(),
                d.message,
                d.suggestion.unwrap_or_default(),
            )
        })
        .collect()
}

fn term(id: &str, name: &str, avoid: &[&str]) -> serde_json::Value {
    json!({ "id": id, "kind": "term", "fields": { "name": name, "definition": "a word" },
            "values": { "avoid": avoid } })
}

#[test]
fn an_avoided_synonym_in_a_requirement_is_w534() {
    let found = found(&input(json!([
        term("work_item", "task", &["ticket"]),
        { "id": "TASK-01", "kind": "requirement", "fields": { "text": "A user can add a Ticket" } },
    ])));
    assert_eq!(
        found,
        [(
            "W534".to_string(),
            "TASK-01".to_string(),
            "requirement 'TASK-01' writes \"Ticket\" in text, \"ticket\" avoided by term 'work_item'"
                .to_string(),
            "write \"task\" (term 'work_item') instead".to_string(),
        )]
    );
}

/// French: an accent and a capital match ("l'Élève" holds "élève"), a longer word does not ("élèves").
#[test]
fn an_avoided_french_word_is_w534_and_a_longer_word_is_not() {
    let found = found(&input(json!([
        term("apprenant", "apprenant", &["élève"]),
        { "id": "R1", "kind": "requirement", "fields": { "text": "Gérer l'Élève" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "Lister les élèves" } },
    ])));
    let on: Vec<&str> = found
        .iter()
        .map(|(_, entity, _, _)| entity.as_str())
        .collect();
    assert_eq!(on, ["R1"], "{found:#?}");
}

/// Two terms avoid "client", which a decision writes twice in two fields: one W534, naming both terms and the first
/// field that holds it, quoting what the field writes ("client") and the avoid list's spelling ("Client").
#[test]
fn one_w534_per_entity_and_word_naming_every_term() {
    let found = found(&input(json!([
        term("customer", "customer", &["client"]),
        term("buyer", "buyer", &["Client"]),
        { "id": "D1", "kind": "decision",
          "fields": { "text": "Bill the client", "rationale": "a client pays" } },
    ])));
    assert_eq!(found.len(), 1, "{found:#?}");
    let (code, entity, message, suggestion) = &found[0];
    assert_eq!((code.as_str(), entity.as_str()), ("W534", "D1"));
    assert_eq!(
        message,
        "decision 'D1' writes \"client\" in text, \"Client\" avoided by term 'buyer' and term 'customer'"
    );
    assert_eq!(
        suggestion,
        "write \"buyer\" (term 'buyer') or \"customer\" (term 'customer') instead"
    );
}

/// A term that avoids another term's name contradicts the glossary: that is gsd/W538 on the term, and no prose that
/// writes the accepted word is W534 for it. A term is no prose.
#[test]
fn a_term_that_avoids_a_terms_name_is_w538_and_not_w534() {
    let found = found(&input(json!([
        term("work_item", "task", &["board"]),
        term("board", "board", &[]),
        { "id": "SHARE-01", "kind": "requirement", "fields": { "text": "A user can share a board" } },
    ])));
    assert_eq!(found, []);
}

#[test]
fn prose_without_an_avoided_word_is_not_w534() {
    let found = found(&input(json!([
        term("work_item", "task", &["ticket"]),
        { "id": "TASK-01", "kind": "requirement", "fields": { "text": "A user can add a task; tickets are gone" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "A user can add a task", "area": "ticket" } },
    ])));
    assert_eq!(found, []);
}

/// An avoided word the prose writes inside an accepted name is that name: "the ticket system" names term 'helpdesk',
/// so only a "ticket" outside it is W534.
#[test]
fn an_avoided_word_inside_an_accepted_name_is_not_w534() {
    let found = found(&input(json!([
        term("work_item", "task", &["ticket"]),
        term("helpdesk", "ticket system", &[]),
        { "id": "TASK-01", "kind": "requirement",
          "fields": { "text": "Tasks are imported from the ticket system" } },
        { "id": "TASK-02", "kind": "requirement",
          "fields": { "text": "A ticket in the Ticket\n System becomes a task" } },
    ])));
    let on: Vec<&str> = found
        .iter()
        .map(|(_, entity, _, _)| entity.as_str())
        .collect();
    assert_eq!(on, ["TASK-02"], "{found:#?}");
}

/// An avoided phrase that holds an accepted word is still the avoided phrase; an avoided word inside an alias the
/// prose writes is that alias.
#[test]
fn an_avoided_phrase_that_holds_an_accepted_word_is_w534() {
    let mut helpdesk = term("helpdesk", "support desk", &["ticket system"]);
    helpdesk["values"]["aliases"] = json!(["help desk"]);
    let found = found(&input(json!([
        helpdesk,
        term("ticket", "ticket", &["desk"]),
        { "id": "R1", "kind": "requirement", "fields": { "text": "The ticket system routes a ticket" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "The help desk answers a ticket" } },
    ])));
    let on: Vec<(&str, &str)> = found
        .iter()
        .map(|(_, entity, message, _)| (entity.as_str(), message.as_str()))
        .collect();
    assert_eq!(
        on,
        [(
            "R1",
            "requirement 'R1' writes \"ticket system\" in text, \"ticket system\" avoided by term 'helpdesk'"
        )],
        "{found:#?}"
    );
}

/// A path or a code span in prose names a file or code, not the domain: "todo" in `src/todo-list.js` asks for no
/// rewrite, while the same word in plain prose does.
#[test]
fn w534_reads_no_word_in_a_path_or_a_code_span() {
    let found = found(&input(json!([
        term("task", "task", &["todo"]),
        { "id": "R1", "kind": "requirement", "fields": { "text": "Write src/todo-list.js" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "Call `todo.add()` once" } },
        { "id": "R3", "kind": "requirement", "fields": { "text": "Show the todo list" } },
    ])));
    let on: Vec<&str> = found
        .iter()
        .map(|(_, entity, _, _)| entity.as_str())
        .collect();
    assert_eq!(on, ["R3"], "{found:#?}");
}

/// The message quotes the prose as it writes the word, case and line break included.
#[test]
fn w534_quotes_the_prose_as_written() {
    let found = found(&input(json!([
        term("work_item", "task", &["support ticket"]),
        { "id": "R1", "kind": "requirement", "fields": { "text": "Close a Support\n  Ticket" } },
    ])));
    let messages: Vec<&str> = found.iter().map(|(_, _, m, _)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "requirement 'R1' writes \"Support\n  Ticket\" in text, \"support ticket\" avoided by term 'work_item'"
        ]
    );
}

/// Every field a kind marks prose is read, beyond the five of a requirement, a decision, a truth, a journey and an
/// invariant: a task's action, a decision's consequences, an alternative's `why_not`.
#[test]
fn w534_reads_every_field_the_kinds_mark_prose() {
    let found = found(&input(json!([
        term("work_item", "task", &["ticket"]),
        { "id": "T1", "kind": "task", "fields": { "action": "close the ticket" } },
        { "id": "D1", "kind": "decision", "fields": { "text": "Keep a board" },
          "values": { "consequences": ["no ticket is lost"],
                      "alternatives": [ { "name": "Jira", "why_not": "a ticket tracker" } ] } },
    ])));
    let on: Vec<(&str, &str)> = found
        .iter()
        .map(|(_, entity, message, _)| (entity.as_str(), message.as_str()))
        .collect();
    assert_eq!(
        on,
        [
            (
                "D1",
                "decision 'D1' writes \"ticket\" in alternative 1 why_not, \"ticket\" avoided by term 'work_item'"
            ),
            (
                "T1",
                "task 'T1' writes \"ticket\" in action, \"ticket\" avoided by term 'work_item'"
            ),
        ],
        "{found:#?}"
    );
}

/// A prose member two blocks down reads with both places: a discussion checkpoint's decision question.
#[test]
fn a_nested_block_member_reads_with_its_places() {
    let discussion: PassEntity = serde_json::from_value(json!({
        "id": "v1.0/01/01-DISCUSSION-LOG", "kind": "discussion",
        "values": {
            "entries": [ { "area": "storage", "question": "Where do tasks live?" } ],
            "checkpoint": { "decisions": [ { "area": "ui", "question": "One board or many?" } ],
                            "deferred_ideas": ["sharing"] },
        },
    }))
    .expect("a discussion");
    assert_eq!(
        prose(&discussion),
        [
            ("entry 1", "Where do tasks live?"),
            ("checkpoint decision 1", "One board or many?"),
            ("checkpoint deferred_ideas", "sharing"),
        ]
        .map(|(place, said)| (place.to_string(), said.to_string()))
    );
}

/// A deviation is a log: its text cannot change once committed (rewriting it is E084), so W534 does not ask for it,
/// while a requirement beside it still gets one. The table says which prose is fixed: an immutable field, an
/// append-only kind's, and a row of an append-only list (a discussion's entries).
#[test]
fn w534_skips_prose_the_author_cannot_change() {
    let found = found(&input(json!([
        term("customer", "customer", &["client"]),
        { "id": "v1.0/01/01-01/01-01-SUMMARY/DV1", "kind": "deviation",
          "fields": { "kind": "deviation", "text": "list() returns the client tasks" } },
        { "id": "R1", "kind": "requirement", "fields": { "text": "A client signs in" } },
    ])));
    let on: Vec<&str> = found
        .iter()
        .map(|(_, entity, _, _)| entity.as_str())
        .collect();
    assert_eq!(on, ["R1"], "{found:#?}");
    assert!(fields("deviation").iter().all(|p| p.fixed));
    let discussion = fields("discussion");
    let fixed = |block: &str| {
        discussion
            .iter()
            .find(|p| p.blocks[0].0 == block && p.name == "question")
            .map(|p| p.fixed)
    };
    // A row of an append-only list is fixed; the checkpoint, set and cleared while the discussion runs, is not.
    assert_eq!(
        (fixed("entries"), fixed("checkpoint")),
        (Some(true), Some(false))
    );
    assert!(fields("requirement").iter().all(|p| !p.fixed));
}
