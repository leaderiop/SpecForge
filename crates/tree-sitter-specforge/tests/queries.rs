use tree_sitter::{QueryError, StreamingIterator};

fn load_query(scm: &str) -> Result<tree_sitter::Query, QueryError> {
    let language = tree_sitter_specforge::LANGUAGE.into();
    tree_sitter::Query::new(&language, scm)
}

#[test]
fn highlights_scm_loads_without_error() {
    let scm = include_str!("../queries/highlights.scm");
    load_query(scm).expect("highlights.scm failed to load");
}

#[test]
fn folds_scm_loads_without_error() {
    let scm = include_str!("../queries/folds.scm");
    load_query(scm).expect("folds.scm failed to load");
}

#[test]
fn indents_scm_loads_without_error() {
    let scm = include_str!("../queries/indents.scm");
    load_query(scm).expect("indents.scm failed to load");
}

#[test]
fn highlights_captures_entity_kind_as_keyword() {
    let scm = include_str!("../queries/highlights.scm");
    let language = tree_sitter_specforge::LANGUAGE.into();
    let query = tree_sitter::Query::new(&language, scm).unwrap();

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).unwrap();
    let source = r#"behavior foo "T" { status "ok" }"#;
    let tree = parser.parse(source, None).unwrap();

    let mut cursor = tree_sitter::QueryCursor::new();
    let mut capture_names: Vec<&str> = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for c in m.captures {
            capture_names.push(&query.capture_names()[c.index as usize]);
        }
    }

    assert!(capture_names.contains(&"keyword"), "entity kind should be captured as @keyword, got: {:?}", capture_names);
    assert!(capture_names.contains(&"constant"), "entity name should be captured as @constant");
    assert!(capture_names.contains(&"string"), "strings should be captured as @string");
}
