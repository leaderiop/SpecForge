//! Completion as production runs it: the cursor's site
//! (`Document::at(..).completion()`) and its items
//! (`completion::items`) over a project view.

use crate::registries::registries;
use specforge_common::{SourceSpan, Sym};
use specforge_extension_sdk::prelude::{FieldType, Vocabulary};
use specforge_graph::{Graph, Node};
use specforge_lsp::completion::items;
use specforge_lsp::{CompletionSite, Document};
use specforge_ops::authoring::format::Place;
use specforge_ops::view::ProjectView;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_project::CompiledProject;
use specforge_project::testing::Assembled;
use specforge_project::testing::TestProject;
use specforge_registry::RegistryBuild;
use specforge_test_macros::test as spec;
use std::sync::Arc;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, MarkupKind, Position,
};

/// A project of `files` with `@specforge/software`, compiled with its
/// extension loaded (and the directory that holds it).
pub fn compiled(files: &[(&str, &str)]) -> (TestProject, CompiledProject) {
    let dir = TestProject::new()
        .config(|c| c["name"] = "completion".into())
        .enabling(&["@specforge/software", "@specforge/governance"])
        .files(files);
    let project = dir.loaded();
    (dir, project)
}

/// The site and items at (`line`, `character`) of `text`, over `view`, for a buffer outside any project.
pub fn complete<'v>(
    text: &str,
    line: u32,
    character: u32,
    view: &ProjectView<'v>,
) -> (String, Vec<CompletionItem>) {
    complete_at(text, line, character, view, Place::Detached)
}

/// [`complete`] for a document at `place`.
pub fn complete_at<'v>(
    text: &str,
    line: u32,
    character: u32,
    view: &ProjectView<'v>,
    place: Place<'_>,
) -> (String, Vec<CompletionItem>) {
    let doc = Document::new(text.into());
    let cursor = doc.at(Position::new(line, character)).expect("a position");
    let found = items(&cursor, false, view, place);
    (format!("{:?}", cursor.completion()), found)
}

/// The items at a position of `text`, in a project made of `text` alone.
fn complete_in_project(text: &str, line: u32, character: u32) -> Vec<CompletionItem> {
    let (_dir, project) = compiled(&[("test.spec", text)]);
    complete(text, line, character, &ProjectView::of(&project)).1
}

fn labels(items: &[CompletionItem]) -> Vec<&str> {
    items.iter().map(|i| i.label.as_str()).collect()
}

fn all_of_kind(items: &[CompletionItem], kind: CompletionItemKind) -> bool {
    items.iter().all(|i| i.kind == Some(kind))
}

const TWO_KINDS: &str = "behavior login \"Login\" {\n  \n}\n\ntype token \"Token\" {\n  \n}\n";

#[spec(
    behavior = "complete_field_names",
    verify = "field name completion uses FieldRegistry for entity kind"
)]
fn fields_of_the_enclosing_kind() {
    let found = complete_in_project(TWO_KINDS, 1, 2);
    let names = labels(&found);
    assert!(names.contains(&"contract"), "{names:?}");
    assert!(names.contains(&"invariants"), "{names:?}");
    assert!(all_of_kind(&found, CompletionItemKind::FIELD));
}

#[spec(
    behavior = "complete_field_names",
    verify = "suggestions are filtered by entity kind"
)]
fn fields_differ_by_kind() {
    let (_dir, project) = compiled(&[("test.spec", TWO_KINDS)]);
    let view = ProjectView::of(&project);
    let behavior = complete(TWO_KINDS, 1, 2, &view).1;
    let token = complete(TWO_KINDS, 5, 2, &view).1;
    assert_ne!(labels(&behavior), labels(&token));
    assert!(labels(&token).contains(&"extends"), "{:?}", labels(&token));
}

#[spec(
    behavior = "complete_field_names",
    verify = "no field name suggestions outside entity blocks"
)]
fn no_fields_outside_entity_blocks() {
    let found = complete_in_project(TWO_KINDS, 3, 0);
    assert!(
        found
            .iter()
            .all(|i| i.kind != Some(CompletionItemKind::FIELD)),
        "{:?}",
        labels(&found)
    );
}

#[spec(
    behavior = "complete_keywords",
    verify = "keyword completion includes all registered kinds"
)]
fn registered_kinds_at_top_level() {
    let (_dir, project) = compiled(&[("test.spec", TWO_KINDS)]);
    let view = ProjectView::of(&project);
    let (site, found) = complete(TWO_KINDS, 3, 0, &view);
    assert_eq!(site, "Keywords { prefix: \"\" }");
    let names = labels(&found);
    for kind in view.registries().kinds().keywords() {
        assert!(names.contains(&kind.as_str()), "{kind} in {names:?}");
    }
    let behavior = found.iter().find(|i| i.label == "behavior").unwrap();
    assert_eq!(behavior.detail.as_deref(), Some("@specforge/software"));
}

#[spec(
    behavior = "complete_keywords",
    verify = "use is always suggested and define never is"
)]
fn use_is_suggested_and_define_never_is() {
    // With and without extensions.
    let empty = Assembled::over(RegistryBuild::default());
    let bare = ProjectView::of(empty.detached());
    let found = complete("\n", 0, 0, &bare).1;
    assert_eq!(labels(&found), ["defaults", "use"]);
    let found = complete_in_project("\n", 0, 0);
    let names = labels(&found);
    assert!(names.contains(&"use"), "{names:?}");
    assert!(!names.contains(&"define"), "{names:?}");
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(names, sorted, "sorted, no duplicates");
}

#[test]
fn no_keywords_inside_a_block() {
    let found = complete_in_project(TWO_KINDS, 1, 2);
    assert!(
        found
            .iter()
            .all(|i| i.kind != Some(CompletionItemKind::KEYWORD)),
        "{:?}",
        labels(&found)
    );
}

#[spec(
    behavior = "complete_field_names",
    verify = "a bracket inside a string opens no reference list"
)]
fn a_bracket_in_a_string_opens_no_reference_list() {
    let text = "behavior alpha \"Alpha\" {\n  contract \"see [docs\"\n  \n}\n";
    let (site, found) = complete(
        text,
        2,
        2,
        &ProjectView::of(&compiled(&[("test.spec", text)]).1),
    );
    assert_eq!(
        site, "Fields { kind: \"behavior\", prefix: \"\" }",
        "the string's `[` opens nothing"
    );
    assert!(labels(&found).contains(&"refs"), "{:?}", labels(&found));
}

#[spec(
    behavior = "complete_field_names",
    verify = "nothing is suggested inside a string, a comment or a nested block"
)]
fn nothing_completes_in_strings_comments_or_nested_blocks() {
    let text = concat!(
        "behavior eps \"Eps\" {\n",
        "  requires {\n",
        "    \n",
        "  }\n",
        "  description \"the \"\n",
        "  // a note \n",
        "}\n",
        "define widget {\n",
        "  \n",
        "}\n",
    );
    let (_dir, project) = compiled(&[("test.spec", text)]);
    let view = ProjectView::of(&project);
    // An open block's body is a block the cursor is in (a typed one completes its members); no field declares any.
    let open_block = "Members { kind: \"behavior\", path: \"requires\", prefix: \"\" }";
    for (line, character) in [(2, 4), (4, 19), (5, 7), (8, 2), (0, 10)] {
        let (site, found) = complete(text, line, character, &view);
        let want = if (line, character) == (2, 4) {
            open_block
        } else {
            "Nothing"
        };
        assert_eq!(site, want, "{line}:{character}");
        assert!(found.is_empty(), "{line}:{character}: {:?}", labels(&found));
    }
}

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "suggestions filtered by target_kind when FieldRegistry has constraint"
)]
fn list_items_complete_ids_of_the_target_kind() {
    let text = concat!(
        "type token \"Token\" {}\n",
        "invariant tidy \"Tidy\" {\n  guarantee \"x\"\n}\n",
        "behavior login \"Login\" {\n  types []\n}\n",
    );
    let found = complete_in_project(text, 5, 9);
    assert_eq!(labels(&found), ["token"]);
    assert!(all_of_kind(&found, CompletionItemKind::REFERENCE));
}

#[test]
fn the_prefix_is_the_word_before_the_cursor() {
    let text = "behavior login \"Login\" {\n  invariants []\n}\n";
    // Inside `inv|ariants`: the prefix is `inv`, not the whole word.
    let doc = Document::new(text.into());
    let cursor = doc.at(Position::new(1, 5)).unwrap();
    assert_eq!(
        cursor.completion(),
        CompletionSite::Fields {
            kind: "behavior",
            prefix: "inv"
        }
    );
    let edit = cursor.word_edit();
    assert_eq!(
        (edit.insert.start, edit.insert.end),
        (Position::new(1, 2), Position::new(1, 5))
    );
    assert_eq!(
        (edit.replace.start, edit.replace.end),
        (Position::new(1, 2), Position::new(1, 12))
    );
    let found = complete_in_project(text, 1, 5);
    assert_eq!(labels(&found), ["invariants"]);
    let Some(CompletionTextEdit::Edit(edit)) = &found[0].text_edit else {
        panic!("a plain edit: {:?}", found[0].text_edit);
    };
    assert_eq!(
        (edit.range.start, edit.range.end),
        (Position::new(1, 2), Position::new(1, 5))
    );
    assert_eq!(edit.new_text, "invariants [$1]");
    assert_eq!(found[0].filter_text.as_deref(), Some("invariants"));
}

/// A registry of one kind `task` (verify kinds `unit`, `contract`) with
/// an enum field `state`, a boolean `done`, a string list `tags` and a
/// reference `owner` (to kind `task`); a graph of `alpha`, a task, and
/// `gh.issue:7`, a ref.
struct Fixture {
    project: Assembled,
}

impl Fixture {
    fn new() -> Fixture {
        let registries = registries("@test/ext", |c| {
            c.kind("task", |k| {
                k.testable(true)
                    .supports_verify(true)
                    .verify_kinds(&["unit", "contract"]);
                let mut field = |name: &str,
                                 field_type: FieldType,
                                 target: Option<&str>,
                                 values: &'static [&'static str]| {
                    k.field(name, |f| {
                        if values.is_empty() {
                            f.field_type(field_type);
                        } else {
                            f.vocabulary(&Vocabulary::new(values));
                        }
                        f.description(&format!("the {name}"));
                        if let Some(target) = target {
                            f.target_kind(target);
                        }
                    });
                };
                field("state", FieldType::Enum, None, &["draft", "active", "done"]);
                field("done", FieldType::Bool, None, &[]);
                field("tags", FieldType::StringList, None, &[]);
                field("owner", FieldType::Reference, Some("task"), &[]);
            });
        });
        let mut graph = Graph::new();
        for (id, kind) in [("alpha", "task"), ("gh.issue:7", "ref")] {
            graph.add_node(Node {
                id: EntityId { raw: Sym::new(id) },
                kind: EntityKind {
                    raw: Sym::new(kind),
                },
                title: None,
                fields: FieldMap::new(),
                source_span: SourceSpan {
                    file: Sym::new("f.spec"),
                    start_line: 1,
                    start_col: 1,
                    end_line: 1,
                    end_col: 1,
                },
                methods: Vec::new(),
                obligations: Vec::new(),
                verdicts: Vec::new(),
                scope: None,
            });
        }
        Fixture {
            project: Assembled::over(registries).graph(graph),
        }
    }

    fn view(&self) -> ProjectView<'_> {
        ProjectView::of(self.project.detached())
    }
}

#[spec(
    behavior = "complete_field_names",
    verify = "an enum field's value suggests its declared values"
)]
fn an_enum_value_suggests_its_declared_values() {
    let fixture = Fixture::new();
    let text = "task t \"T\" {\n  state \n  state d\n}\n";
    let (site, found) = complete(text, 1, 8, &fixture.view());
    assert_eq!(
        site,
        "Value { kind: \"task\", field: \"state\", prefix: \"\" }"
    );
    assert_eq!(labels(&found), ["draft", "active", "done"]);
    assert!(all_of_kind(&found, CompletionItemKind::ENUM_MEMBER));
    assert_eq!(found[0].detail.as_deref(), Some("the state"));
    let found = complete(text, 2, 9, &fixture.view()).1;
    assert_eq!(labels(&found), ["draft", "done"]);
}

#[spec(
    behavior = "complete_field_names",
    verify = "a builtin vocabulary's field completes its values"
)]
fn a_builtin_vocabulary_completes_its_values() {
    let text = "feature f \"F\" {\n  status \n}\n";
    let dir = TestProject::new()
        .config(|c| c["name"] = "vocab".into())
        .enabling(&["@specforge/product"])
        .file("test.spec", text);
    let runtime = specforge_component::ComponentRuntime::new();
    specforge_component::builtins::load_builtins_for(&runtime, &["@specforge/product".to_string()])
        .unwrap();
    let project = dir.compile_in(Arc::new(runtime));
    let found = complete(text, 1, 9, &ProjectView::of(&project)).1;
    assert_eq!(
        labels(&found),
        [
            "proposed",
            "accepted",
            "in_progress",
            "done",
            "deferred",
            "deprecated"
        ]
    );
    assert!(all_of_kind(&found, CompletionItemKind::ENUM_MEMBER));
}

#[spec(
    behavior = "complete_field_names",
    verify = "a boolean field's value suggests true and false"
)]
fn a_bool_value_suggests_true_and_false() {
    let fixture = Fixture::new();
    let text = "task t \"T\" {\n  done \n}\n";
    let found = complete(text, 1, 7, &fixture.view()).1;
    assert_eq!(labels(&found), ["true", "false"]);
    assert!(all_of_kind(&found, CompletionItemKind::KEYWORD));
}

#[spec(
    behavior = "autocomplete_entity_ids",
    verify = "a string list's items suggest no entity IDs but scheme ref IDs"
)]
fn a_string_list_suggests_no_ids() {
    let fixture = Fixture::new();
    let text = "task t \"T\" {\n  tags []\n}\n";
    let (site, found) = complete(text, 1, 8, &fixture.view());
    assert_eq!(
        site,
        "ListItem { kind: \"task\", field: \"tags\", prefix: \"\", written: [] }"
    );
    // No entity's ID; only a ref's scheme ref ID, which the core links to
    // its ref from any list.
    assert_eq!(labels(&found), ["gh.issue:7"]);
}

#[spec(
    behavior = "complete_keywords",
    verify = "verify suggests the kinds the entity's kind allows"
)]
fn verify_suggests_the_kinds_the_entity_allows() {
    let fixture = Fixture::new();
    let text = "task t \"T\" {\n  verify \n  verify c\n}\nother o \"O\" {\n  verify \n}\n";
    let (site, found) = complete(text, 1, 9, &fixture.view());
    assert_eq!(site, "VerifyKind { kind: \"task\", prefix: \"\" }");
    assert_eq!(labels(&found), ["unit", "contract"]);
    assert!(all_of_kind(&found, CompletionItemKind::ENUM_MEMBER));
    assert_eq!(
        labels(&complete(text, 2, 10, &fixture.view()).1),
        ["contract"]
    );
    // A kind the registry does not know takes no verify statements here.
    assert!(complete(text, 5, 9, &fixture.view()).1.is_empty());
}

#[spec(
    behavior = "complete_keywords",
    verify = "nothing is suggested after the kind of a verify statement being typed"
)]
fn nothing_is_suggested_after_a_verify_kind_being_typed() {
    let fixture = Fixture::new();
    let text = "task t \"T\" {\n  verify unit \n}\n";
    let (site, found) = complete(text, 1, 14, &fixture.view());
    assert_eq!(site, "Nothing");
    assert!(found.is_empty());
}

#[test]
fn a_single_reference_value_completes_its_target_kinds_ids() {
    let fixture = Fixture::new();
    let text = "task t \"T\" {\n  owner \n}\n";
    let found = complete(text, 1, 8, &fixture.view()).1;
    assert_eq!(labels(&found), ["alpha"]);
}

#[spec(
    behavior = "complete_keywords",
    verify = "a kind keyword's completion documents the kind with its description and inference guide"
)]
fn a_kind_keyword_is_documented_by_its_guide() {
    let (_dir, project) = compiled(&[("test.spec", TWO_KINDS)]);
    let view = ProjectView::of(&project);
    let (_, found) = complete(TWO_KINDS, 3, 0, &view);

    let behavior = found.iter().find(|i| i.label == "behavior").unwrap();
    let Some(Documentation::MarkupContent(markup)) = &behavior.documentation else {
        panic!("behavior has no markdown documentation: {behavior:?}");
    };
    assert_eq!(markup.kind, MarkupKind::Markdown);
    assert!(
        markup
            .value
            .starts_with("A testable unit of system functionality with a defined contract"),
        "{}",
        markup.value
    );
    assert!(markup.value.contains("**Inferring it from code**"));
    assert!(markup.value.contains("Look for public functions"));

    let use_keyword = found.iter().find(|i| i.label == "use").unwrap();
    assert!(use_keyword.documentation.is_none());
}

#[spec(
    behavior = "complete_field_names",
    verify = "a field the block already writes is not suggested, but the one under the cursor is"
)]
fn a_field_the_block_writes_is_not_suggested_again() {
    let text = "behavior login \"Login\" {\n  category command\n  \n}\n";
    let names = labels(&complete_in_project(text, 2, 2))
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    assert!(!names.contains(&"category".to_string()), "{names:?}");
    assert!(names.contains(&"contract".to_string()), "{names:?}");
    // On the written key itself, completing rewrites it: it is offered.
    assert_eq!(labels(&complete_in_project(text, 1, 5)), ["category"]);
    // A defaults block's body too (a repeat there is E078).
    let defaults = "defaults behavior {\n  category command\n  \n}\n";
    assert!(!labels(&complete_in_project(defaults, 2, 2)).contains(&"category"));
}
