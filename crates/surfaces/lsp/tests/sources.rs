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
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("a.test.js"), SOURCE).unwrap();
    citing_in(dir, covered, "a.test.js")
}

/// [`citing`] in `dir`, the fact placed in `file` (a key from the root) and the store recording [`SOURCE`]'s digest
/// of it, whatever `dir` holds there.
fn citing_in(dir: tempfile::TempDir, covered: &str, file: &str) -> Served {
    use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, stored_with};
    fs::write(
        dir.path().join("truths.spec"),
        "truth T1 \"One\" {\n  proof [\"js:a.test.js#adds\"]\n}\n",
    )
    .unwrap();
    let place = |line: usize, col: usize, end_line: usize, end_col: usize| json!({"file": file, "start_line": line, "start_col": col, "end_line": end_line, "end_col": end_col});
    stored_with(
        dir.path(),
        "node",
        "0123456789abcdef0123456789abcdef01234567",
        json!([{"kind": "test", "id": "js:a.test.js#adds", "at": [place(3, 1, 5, 4)],
                "fields": {"file": file, "covers": [covered]},
                "named_at": {"covers": {covered: [place(1, 11, 1, 13)]}}}]),
        [(
            file.to_string(),
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

/// The edits of renaming the entity at `position` of `file` to `T2`, as `(file name, range, new text)`, sorted.
fn renamed(
    served: &Served,
    file: &str,
    position: Position,
) -> Result<Vec<(String, Range, String)>, tower_lsp::jsonrpc::Error> {
    let edit = specforge_lsp::answers::rename(&served.state(), &served.uri(file), position, "T2")?
        .expect("the cursor names an entity");
    let mut edits: Vec<(String, Range, String)> = edit
        .changes
        .expect("plain changes")
        .into_iter()
        .flat_map(|(uri, edits)| {
            let name = uri.path().rsplit('/').next().unwrap().to_string();
            edits
                .into_iter()
                .map(move |e| (name.clone(), e.range, e.new_text))
        })
        .collect();
    edits.sort_by_key(|(file, range, _)| (file.clone(), range.start));
    Ok(edits)
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename rewrites the source token of every fact naming the entity"
)]
fn rename_rewrites_the_source_token_naming_the_entity() {
    let served = citing("T1");
    let expected = vec![
        ("a.test.js".to_string(), named(), "T2".to_string()),
        (
            "truths.spec".to_string(),
            Range::new(Position::new(0, 6), Position::new(0, 8)),
            "T2".to_string(),
        ),
    ];
    // From the name in the source file, and from the declaration: the same edits.
    assert_eq!(
        renamed(&served, "a.test.js", Position::new(0, 11)).unwrap(),
        expected
    );
    assert_eq!(
        renamed(&served, "truths.spec", Position::new(0, 6)).unwrap(),
        expected
    );
    // Elsewhere in the source file nothing renames.
    let source = served.uri("a.test.js");
    let none = specforge_lsp::answers::rename(&served.state(), &source, Position::new(2, 2), "T2");
    assert_eq!(none.unwrap(), None);
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename refuses while a source file it edits changed since collect"
)]
fn rename_refuses_a_stale_source_file() {
    let mut served = citing("T1");
    // Typed in since the collect, the disk as collected: refused with W198's sentence.
    served.hold("a.test.js");
    served.type_into(&served.uri("a.test.js"), &format!("// typed\n{SOURCE}"));
    for (file, position) in [
        ("truths.spec", Position::new(0, 6)),
        ("a.test.js", Position::new(1, 11)),
    ] {
        let refused = renamed(&served, file, position).unwrap_err();
        assert_eq!(refused.code, tower_lsp::jsonrpc::ErrorCode::InvalidParams);
        assert!(
            refused
                .message
                .starts_with("'a.test.js' changed since `specforge collect` read it"),
            "{file}: {}",
            refused.message
        );
    }
    // Changed on disk: refused too, from the declaration.
    served.close_uri(&served.uri("a.test.js"));
    served.write("a.test.js", &format!("// edited\n{SOURCE}"));
    let refused = renamed(&served, "truths.spec", Position::new(0, 6)).unwrap_err();
    assert_eq!(refused.code, tower_lsp::jsonrpc::ErrorCode::InvalidParams);
    assert!(
        refused
            .message
            .contains("a.test.js changed since `specforge collect` read it"),
        "{}",
        refused.message
    );
}

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename answers a source token naming an entity"
)]
fn prepare_rename_answers_a_source_token() {
    let served = citing("T1");
    let source = served.uri("a.test.js");
    let prepare = |served: &Served, position| {
        specforge_lsp::answers::prepare_rename(&served.state(), &source, position)
    };
    assert_eq!(
        prepare(&served, Position::new(0, 11)).unwrap(),
        Some(tower_lsp::lsp_types::PrepareRenameResponse::Range(named()))
    );
    assert_eq!(prepare(&served, Position::new(2, 2)).unwrap(), None);
    served.write("a.test.js", "// edited\n");
    let refused = prepare(&served, Position::new(0, 11)).unwrap_err();
    assert!(
        refused.message.starts_with("'a.test.js' changed since"),
        "{}",
        refused.message
    );
}

/// A project inside `outer`, beside `outer/x.js`, which holds [`SOURCE`]: what a source file of the project must
/// never reach.
fn beside_an_outside_file() -> (tempfile::TempDir, tempfile::TempDir) {
    let outer = tempfile::TempDir::new().unwrap();
    fs::write(outer.path().join("x.js"), SOURCE).unwrap();
    let dir = tempfile::TempDir::new_in(outer.path()).unwrap();
    (outer, dir)
}

/// Renaming `T1` is refused from the declaration and, when `from_source`, from the name in `a.test.js`; the
/// outside file keeps its text.
fn refused_and_untouched(served: &Served, outer: &tempfile::TempDir, from_source: bool) {
    let mut from = vec![("truths.spec", Position::new(0, 6))];
    if from_source {
        from.push(("a.test.js", Position::new(0, 11)));
    }
    for (file, position) in from {
        let refused = renamed(served, file, position);
        assert!(refused.is_err(), "{file}: {refused:?}");
    }
    assert_eq!(
        fs::read_to_string(outer.path().join("x.js")).unwrap(),
        SOURCE
    );
}

#[cfg(unix)]
#[spec(
    behavior = "rename_entity_id",
    verify = "rename refuses a source file it edits that is a link or lies outside the project"
)]
fn rename_refuses_a_symlinked_source_file() {
    let (outer, dir) = beside_an_outside_file();
    std::os::unix::fs::symlink(outer.path().join("x.js"), dir.path().join("a.test.js")).unwrap();
    let served = citing_in(dir, "T1", "a.test.js");
    refused_and_untouched(&served, &outer, true);
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename refuses a source file it edits that is a link or lies outside the project"
)]
fn rename_refuses_a_hard_linked_source_file() {
    let (outer, dir) = beside_an_outside_file();
    fs::hard_link(outer.path().join("x.js"), dir.path().join("a.test.js")).unwrap();
    let served = citing_in(dir, "T1", "a.test.js");
    refused_and_untouched(&served, &outer, true);
}

#[spec(
    behavior = "rename_entity_id",
    verify = "rename refuses a source file it edits that is a link or lies outside the project"
)]
fn rename_refuses_a_fact_placed_outside_the_project() {
    let (outer, dir) = beside_an_outside_file();
    let served = citing_in(dir, "T1", "../x.js");
    refused_and_untouched(&served, &outer, false);
}
