use crate::*;

#[test]
fn a_namespace_is_the_scope_and_name_and_core_is_the_scope() {
    for (extension, namespace) in [
        ("@gsd/core", Some("gsd")),
        ("@gsd/security", Some("gsd.security")),
        ("lint", Some("lint")),
        ("@acme/lint", Some("acme.lint")),
        ("@a/b.c", Some("a.b.c")),
        ("@specforge/product", None),
    ] {
        assert_eq!(
            code_namespace(extension).as_deref(),
            namespace,
            "{extension}"
        );
    }
    assert_eq!(qualify("@gsd/core", "E101"), "gsd/E101");
    assert_eq!(qualify("@specforge/product", "W041"), "W041");
}

#[test]
fn a_qualified_code_parses_and_normalizes_any_case() {
    let parsed = CodeRef::parse("gsd/E101");
    assert_eq!((parsed.namespace, parsed.local), (Some("gsd"), "E101"));
    let bare = CodeRef::parse("E001");
    assert_eq!((bare.namespace, bare.local), (None, "E001"));
    assert_eq!(
        CodeRef::normalized("GSD.Security/w003"),
        "gsd.security/W003"
    );
    assert_eq!(CodeRef::normalized("e001"), "E001");
}

#[test]
fn codebook_describes_a_static_code_to_the_host_and_its_owner_only() {
    let book = Codebook::default();
    for entry in CATALOG {
        for origin in [None, Some(entry.owner), Some("@acme/x")] {
            let described = origin.is_none_or(|o| o == entry.owner);
            let expected = described.then_some((entry.title, entry.owner, entry.explanation));
            let got = book
                .describes(entry.code, origin)
                .map(|d| (d.title, d.owner, d.explanation));
            assert_eq!(got, expected, "{} for {origin:?}", entry.code);
        }
    }
}

fn code(local: &str) -> ExtensionCode {
    ExtensionCode {
        local: local.into(),
        title: "Too many tasks".into(),
        explanation: "A plan holds at most four tasks.".into(),
        docs: Some("https://gsd.dev/codes#w304".into()),
    }
}

#[test]
fn a_codebook_refuses_a_second_owner_of_a_namespace() {
    let mut book = Codebook::default();
    book.add("@gsd/core", vec![code("W304")]).unwrap();
    let refused = book.add("gsd", vec![code("W305")]).unwrap_err();
    assert_eq!(
        (
            refused.namespace.as_str(),
            refused.owner.as_str(),
            refused.other.as_str()
        ),
        ("gsd", "@gsd/core", "gsd")
    );
    assert!(!book.catalogued("gsd", "W305"));
    assert!(book.describes("gsd/W304", Some("gsd")).is_none());
    assert!(book.describes("gsd/W304", Some("@gsd/core")).is_some());
}

#[test]
fn a_codebook_answers_what_an_extension_catalogues_and_a_codes_letter() {
    let mut book = Codebook::default();
    book.add("@gsd/core", vec![code("W304")]).unwrap();
    assert!(book.catalogued("@gsd/core", "W304"));
    assert!(!book.catalogued("@gsd/core", "W999"));
    assert!(!book.catalogued("@specforge/product", "W041"));
    assert_eq!(book.letter("gsd/W304"), Some(Level::Warning));
    assert_eq!(book.letter("E001"), Some(Level::Error));
    assert_eq!(book.letter("A010"), None);
    assert_eq!(book.letter("gsd/W999"), None);
    let described = book.describes("GSD/w304", None).unwrap();
    assert_eq!(described.code, "gsd/W304");
    assert_eq!(described.owner, "@gsd/core");
    assert_eq!(
        described.docs.as_deref(),
        Some("https://gsd.dev/codes#w304")
    );
    assert_eq!(book.check("@gsd/core", "W304", Level::Warning), Ok(()));
    assert_eq!(
        book.check("@gsd/core", "W304", Level::Error),
        Err(CodeMisuse::PrefixContradictsLevel)
    );
    assert_eq!(
        book.check("@gsd/core", "W999", Level::Warning),
        Err(CodeMisuse::Uncatalogued)
    );
    assert_eq!(
        book.check("@gsd/core", "gsd/W304", Level::Warning),
        Err(CodeMisuse::NotALocalCode)
    );
}
