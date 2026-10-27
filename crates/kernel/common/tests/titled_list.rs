//! A diagnostic list is titled from the code book it was built with (ADR 0286).

use specforge_common::DiagnosticList;

#[test]
fn a_list_and_a_document_title_a_code_from_the_code_book_they_were_built_with() {
    let mut codes = specforge_common::Codebook::default();
    codes
        .add(
            "@sdk/greet",
            vec![specforge_common::ExtensionCode {
                local: "W901".into(),
                title: "Greeting missing".into(),
                explanation: "Add one.".into(),
                docs: None,
            }],
        )
        .unwrap();
    let diagnostic = specforge_common::Diagnostic::from_extension(
        "@sdk/greet",
        "sdk.greet/W901",
        specforge_common::Severity::Warning,
        "m",
    );
    let list = serde_json::to_value(DiagnosticList::new(vec![diagnostic.clone()], &codes)).unwrap();
    assert_eq!(list[0]["title"], "Greeting missing");
    let document = serde_json::to_value(specforge_common::DiagnosticDocument::new(
        diagnostic.clone(),
        &codes,
    ))
    .unwrap();
    assert_eq!(document["title"], "Greeting missing");
    let bare = serde_json::to_value(DiagnosticList::from(vec![diagnostic])).unwrap();
    assert!(bare[0]["title"].is_null());
}
