//! Source documents (ADR 0349 D7, D8): a JavaScript test file where the collector `node` placed the test fact
//! `js:a.test.js#adds` (lines 3-5) whose `covers` names `truth T1` at line 1 (`// covers T1`, the name at characters
//! 10-12), the store recording the file's digest; `truths.spec` cites the fact in `proof`. The server holds the
//! source buffer and never compiles it, places spans in it while it is the text collected, and answers definition,
//! implementation, references, hover, code lenses and rename on the names facts hold there.

use std::fs;

use serde_json::json;
use specforge_lsp::changes::Change;
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range};

use crate::recorder::{codes, publications};
use crate::served::Served;

/// The source file as collected.
pub const SOURCE: &str = "// covers T1\n\ntest('adds', () => {\n  add(1, 2);\n});\n";

/// The project, `covers` naming `covered`, opened with `truths.spec` open.
pub fn citing(covered: &str) -> Served {
    use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, stored_with};
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(
        dir.path().join("truths.spec"),
        "truth T1 \"One\" {\n  proof [\"js:a.test.js#adds\"]\n}\n",
    )
    .unwrap();
    fs::write(dir.path().join("a.test.js"), SOURCE).unwrap();
    let place = |line: usize, col: usize, end_line: usize, end_col: usize| json!({"file": "a.test.js", "start_line": line, "start_col": col, "end_line": end_line, "end_col": end_col});
    stored_with(
        dir.path(),
        "node",
        "0123456789abcdef0123456789abcdef01234567",
        json!([{"kind": "test", "id": "js:a.test.js#adds", "at": [place(3, 1, 5, 4)],
                "fields": {"file": "a.test.js", "covers": [covered]},
                "named_at": {"covers": {covered: [place(1, 11, 1, 13)]}}}]),
        [(
            "a.test.js".to_string(),
            specforge_ops::revision::of_bytes(SOURCE.as_bytes()),
        )]
        .into(),
    );
    Served::served_by(dir, Collected::new().runtime(), &COLLECTED_EXTENSIONS).open(&["truths.spec"])
}

#[spec(
    behavior = "document_open_close",
    verify = "a source file opened or typed in is held and rebuilds nothing"
)]
fn a_source_document_edit_rebuilds_nothing() {
    let mut served = citing("T1");
    let uri = served.uri("a.test.js");
    let runs = served.check_runs();

    assert!(
        served.open_document(&uri, SOURCE).is_none(),
        "opening applies nothing"
    );
    assert!(
        served.state().files().document("a.test.js").is_some(),
        "the buffer is held"
    );
    served.type_into(&uri, "// typed\n");
    assert!(served.apply(Change::Edited(vec![uri.clone()])).is_none());
    assert!(served.close_uri(&uri).is_none());
    assert_eq!(served.check_runs(), runs, "no check ran");
    assert!(
        publications(&served.sent(), &uri).is_empty(),
        "{:?}",
        served.sent()
    );
}

/// A Rust project loading `@code/core`: `src/lib.rs`'s item `rust:p::run` names `named` in `names`, written at
/// line 1 (`// names ghost`, characters 9-14 when `named` is `ghost`), the store recording the file's digest.
fn rust_naming(named: &str) -> Served {
    use specforge_project::testing::{WORKTREE, item, stored_with};
    let source = format!("// names {named}\npub fn run() {{}}\n");
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"p","extensions":["@specforge/software","@code/core"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.spec"),
        "behavior alpha \"A\" {\n  contract \"c\"\n}\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), &source).unwrap();
    let mut fact = item("rust:p::run", "src/lib.rs", 2, "function", "public", "run");
    fact["fields"]["names"] = json!([named]);
    fact["named_at"] = json!({"names": {named: [{"file": "src/lib.rs", "start_line": 1, "start_col": 10,
                                                  "end_line": 1, "end_col": 10 + named.len()}]}});
    stored_with(
        dir.path(),
        "rust-source",
        WORKTREE,
        json!([fact]),
        [(
            "src/lib.rs".to_string(),
            specforge_ops::revision::of_bytes(source.as_bytes()),
        )]
        .into(),
    );
    Served::at(dir).open(&["main.spec"])
}

#[spec(
    behavior = "emit_live_diagnostics",
    verify = "a diagnostic at a source span is published on the source file"
)]
fn a_dangling_fact_reference_is_published_on_its_source_file() {
    let served = rust_naming("ghost");
    let uri = served.uri("src/lib.rs");
    let published = publications(served.opening(), &uri);
    let diagnostics = published.last().expect("src/lib.rs is published");
    assert_eq!(codes(diagnostics), ["code/I402"], "{diagnostics:?}");
    assert_eq!(
        diagnostics[0].range,
        Range::new(Position::new(0, 9), Position::new(0, 14))
    );
}

/// The test fact's place in `a.test.js`: lines 3-5.
fn fact_place() -> Range {
    Range::new(Position::new(2, 0), Position::new(4, 3))
}

#[spec(
    behavior = "go_to_implementation",
    verify = "implementation on an entity lists its anchors' source ranges"
)]
fn implementation_lists_an_entitys_anchors() {
    let served = citing("T1");
    let truths = served.uri("truths.spec");
    // `T1` is anchored to the test twice (its `proof` cites it, its `covers` names T1): one place.
    let found =
        specforge_lsp::answers::implementation(&served.state(), &truths, Position::new(0, 6));
    assert_eq!(
        found,
        Some(GotoDefinitionResponse::Array(vec![Location {
            uri: served.uri("a.test.js"),
            range: fact_place(),
        }]))
    );
}

#[spec(
    behavior = "go_to_definition",
    verify = "go-to-def navigates to entity declaration"
)]
fn definition_on_a_name_a_fact_holds_in_a_source_file_opens_the_entity() {
    let served = citing("T1");
    let source = served.uri("a.test.js");
    let definition =
        |position| specforge_lsp::answers::definition(&served.state(), &source, position);
    let Some(GotoDefinitionResponse::Scalar(location)) = definition(Position::new(0, 11)) else {
        panic!("the name has a definition");
    };
    assert_eq!(location.uri, served.uri("truths.spec"));
    assert_eq!(location.range.start, Position::new(0, 6));
    // Anywhere else in the source file the language's own server answers.
    assert_eq!(definition(Position::new(2, 2)), None);
    assert_eq!(definition(Position::new(0, 3)), None);
}

/// The range of `T1` in line 1 of [`SOURCE`].
fn named() -> Range {
    Range::new(Position::new(0, 10), Position::new(0, 12))
}

/// `(file name, range)` of each location.
fn placed(found: Option<Vec<Location>>) -> Vec<(String, Range)> {
    found
        .unwrap_or_default()
        .into_iter()
        .map(|l| {
            (
                l.uri.path().rsplit('/').next().unwrap().to_string(),
                l.range,
            )
        })
        .collect()
}

#[spec(
    behavior = "find_all_references",
    verify = "references to an entity include where a source fact names it"
)]
fn references_include_the_source_token_naming_the_entity() {
    let served = citing("T1");
    let truths = served.uri("truths.spec");
    let found =
        specforge_lsp::answers::references(&served.state(), &truths, Position::new(0, 6), false);
    assert_eq!(placed(found), [("a.test.js".to_string(), named())]);
    // From the name in the source file, the same.
    let source = served.uri("a.test.js");
    let found =
        specforge_lsp::answers::references(&served.state(), &source, Position::new(0, 11), false);
    assert_eq!(placed(found), [("a.test.js".to_string(), named())]);
}

#[spec(
    behavior = "find_all_references",
    verify = "references from a located fact list its citations and its entities"
)]
fn references_from_a_source_fact_list_citations_and_entities() {
    let served = citing("T1");
    let source = served.uri("a.test.js");
    let found =
        specforge_lsp::answers::references(&served.state(), &source, Position::new(3, 2), false);
    assert_eq!(
        placed(found),
        [
            // `T1`'s name, the entity the fact is anchored to.
            (
                "truths.spec".to_string(),
                Range::new(Position::new(0, 6), Position::new(0, 8))
            ),
            // `proof ["js:a.test.js#adds"]`, the citation.
            (
                "truths.spec".to_string(),
                Range::new(Position::new(1, 10), Position::new(1, 27))
            ),
        ]
    );
    // Outside every fact of the file: nothing.
    let found =
        specforge_lsp::answers::references(&served.state(), &source, Position::new(1, 0), false);
    assert_eq!(found, None);
}

#[spec(
    behavior = "entity_code_lens",
    verify = "a source file has a lens per anchored fact naming its entity, which finds its references"
)]
fn a_source_fact_has_a_lens_per_anchored_entity() {
    let served = citing("T1");
    let source = served.uri("a.test.js");
    let lenses = specforge_lsp::answers::code_lenses(&served.state(), &source).unwrap();
    assert_eq!(lenses.len(), 1, "{lenses:?}");
    let lens = &lenses[0];
    assert_eq!(
        lens.range,
        Range::new(Position::new(2, 0), Position::new(2, 0))
    );
    let command = lens.command.as_ref().unwrap();
    assert_eq!(command.title, "truth T1");
    assert_eq!(command.command, specforge_lsp::answers::FIND_REFERENCES);
    assert_eq!(
        command.arguments,
        Some(vec![json!(source), json!({"line": 2, "character": 0})])
    );
}

#[spec(
    behavior = "entity_code_lens",
    verify = "a source file changed since collect has one stale lens"
)]
fn a_stale_source_file_has_one_lens() {
    let mut served = citing("T1");
    let source = served.uri("a.test.js");
    served.write("a.test.js", "// edited\n");
    let lenses = specforge_lsp::answers::code_lenses(&served.state(), &source).unwrap();
    assert_eq!(lenses.len(), 1, "{lenses:?}");
    let command = lenses[0].command.as_ref().unwrap();
    assert!(
        command.title.starts_with("W198: 'a.test.js' changed since"),
        "{}",
        command.title
    );
    assert!(command.command.is_empty());
    // A buffer typed since the collect is stale too; the text as collected is not.
    served.hold("a.test.js");
    served.type_into(&source, SOURCE);
    let lenses = specforge_lsp::answers::code_lenses(&served.state(), &source).unwrap();
    assert_eq!(lenses[0].command.as_ref().unwrap().title, "truth T1");
}

#[spec(
    behavior = "hover_information",
    verify = "hover on a source token naming an entity shows the entity"
)]
fn hover_on_a_source_token_naming_an_entity_shows_it() {
    let served = citing("T1");
    served.hold("a.test.js");
    let shown = served.hover("a.test.js", 0, 11).expect("the name hovers");
    assert!(shown.contains("T1"), "{shown}");
    // Nowhere else in the source file.
    assert_eq!(served.hover("a.test.js", 2, 2), None);
}
