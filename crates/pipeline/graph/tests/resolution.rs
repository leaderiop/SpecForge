//! What one written reference is (ADR 0117): an edge, E003 or I004, decided in the linker against its
//! field's target kind (behavior `link_entity_references`).

use specforge_graph::{
    FieldForm, FieldForms, Graph, GraphConfig, GraphInputs, build_graph_with_config,
};
use specforge_parser::parse;
use specforge_protocol_types::FieldType;
use specforge_test_macros::test as specforge_test;

/// `behavior.features` names `feature`s; `behavior.peers` names `behavior`s; `behavior.note` names nothing in
/// particular. `loaded` are the kinds a loaded extension declares; `owners` are the absent kinds' providers.
fn build(
    source: &str,
    loaded: &[&str],
    owners: &[(&str, &str)],
) -> (Graph, Vec<specforge_graph::Diagnostic>) {
    let form = |field: &str, target: Option<&str>| {
        let form = FieldForm::new(FieldType::ReferenceList, Vec::new());
        (
            ("behavior".to_string(), field.to_string()),
            match target {
                Some(kind) => form.targeting(kind),
                None => form,
            },
        )
    };
    let config = GraphConfig {
        inputs: GraphInputs {
            field_forms: FieldForms::new(
                [
                    form("features", Some("feature")),
                    form("peers", Some("behavior")),
                    form("note", None),
                ]
                .into_iter()
                .collect(),
                Default::default(),
                loaded.iter().map(|k| k.to_string()).collect(),
            ),
            target_owners: owners
                .iter()
                .map(|(kind, owner)| (kind.to_string(), owner.to_string()))
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    };
    build_graph_with_config(&[parse(source, "main.spec")], &config)
}

fn codes(diagnostics: &[specforge_graph::Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(|d| d.code.as_str()).collect()
}

const MISSING: &str = "behavior b1 \"B1\" { features [f_missing] }\n";

#[specforge_test(
    behavior = "link_entity_references",
    verify = "an unresolved reference in a field whose target kind no extension declares is I004 naming the extension"
)]
fn a_missing_id_of_an_absent_kind_is_i004_naming_its_provider() {
    let (graph, diagnostics) = build(MISSING, &["behavior"], &[("feature", "@specforge/product")]);
    assert_eq!(codes(&diagnostics), ["I004"], "{diagnostics:?}");
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some(
            "enable the extension that declares 'feature' with `specforge add @specforge/product`, or drop the field"
        )
    );
    assert_eq!(
        diagnostics[0].message,
        "reference 'f_missing' in field 'features' of 'b1' targets kind 'feature', which no enabled extension provides"
    );
    assert_eq!(graph.edge_count(), 0);
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "an unresolved reference in a field whose target kind no extension declares is I004 naming the extension"
)]
fn a_missing_id_of_an_absent_kind_with_no_provider_suggests_a_search() {
    let (_, diagnostics) = build(MISSING, &["behavior"], &[]);
    assert_eq!(codes(&diagnostics), ["I004"], "{diagnostics:?}");
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some(
            "find an extension that declares 'feature' with `specforge search feature`, or drop the field"
        )
    );
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "an unresolved reference to a loaded or undeclared target kind is E003 alone"
)]
fn a_missing_id_is_e003_alone() {
    // `feature` is loaded: the field's target exists, so the id is simply missing.
    let (_, diagnostics) = build(MISSING, &["behavior", "feature"], &[]);
    assert_eq!(codes(&diagnostics), ["E003"], "{diagnostics:?}");
    // A field with no target kind: E003 as well.
    let (_, diagnostics) = build("behavior b1 \"B1\" { note [ghost] }\n", &["behavior"], &[]);
    assert_eq!(codes(&diagnostics), ["E003"], "{diagnostics:?}");
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a reference naming an existing entity of the absent target kind links without a diagnostic"
)]
fn an_id_of_the_absent_kind_links_silently() {
    let (graph, diagnostics) = build(
        "behavior b1 \"B1\" { features [f1] }\nfeature f1 \"F1\" { }\n",
        &["behavior"],
        &[("feature", "@specforge/product")],
    );
    assert_eq!(codes(&diagnostics), Vec::<&str>::new(), "{diagnostics:?}");
    assert_eq!(graph.edge_count(), 1);
}

const WRONG_KIND: &str = "behavior b1 \"B1\" {\n  features [b2]\n}\nbehavior b2 \"B2\" {\n}\n";

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a reference to an existing entity of another kind than its field's target kind is E022 at the reference"
)]
fn a_reference_to_another_kind_is_e022_at_the_reference_and_links() {
    let (graph, diagnostics) = build(WRONG_KIND, &["behavior", "feature"], &[]);
    assert_eq!(codes(&diagnostics), ["E022"], "{diagnostics:?}");
    let span = diagnostics[0].span.as_ref().unwrap();
    assert_eq!(
        (span.start_line, span.start_col, span.end_col),
        (2, 13, 15),
        "the token"
    );
    assert_eq!(
        graph.edge_count(),
        1,
        "it still links, so navigation reaches it"
    );
    assert_eq!(
        diagnostics[0].message,
        "reference 'b2' in field 'features' of behavior 'b1' targets a behavior, but this field expects feature"
    );
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some("name a feature here, or drop the reference")
    );
}

#[test]
fn a_loaded_target_kind_suggests_a_close_id_of_that_kind() {
    let (_, diagnostics) = build(
        "behavior b1 \"B1\" {\n  features [f_one]\n}\nbehavior f_one \"B\" {\n}\nfeature f_on \"F\" {\n}\n",
        &["behavior", "feature"],
        &[],
    );
    assert_eq!(codes(&diagnostics), ["E022"], "{diagnostics:?}");
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some("did you mean 'f_on'?")
    );
}

#[test]
fn a_reference_to_its_target_kind_links_silently() {
    let (graph, diagnostics) = build(
        "behavior b1 \"B1\" { peers [b2] }\nbehavior b2 \"B2\" { }\n",
        &["behavior"],
        &[],
    );
    assert_eq!(codes(&diagnostics), Vec::<&str>::new(), "{diagnostics:?}");
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn a_field_without_a_target_kind_links_any_kind() {
    let (graph, diagnostics) = build(
        "behavior b1 \"B1\" { note [f1] }\nfeature f1 \"F\" { }\n",
        &["behavior", "feature"],
        &[],
    );
    assert_eq!(codes(&diagnostics), Vec::<&str>::new(), "{diagnostics:?}");
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn an_undeclared_kinds_field_has_no_target() {
    let (graph, diagnostics) = build(
        "gadget g1 \"G\" { features [b2] }\nbehavior b2 \"B2\" { }\n",
        &["behavior", "feature"],
        &[],
    );
    assert_eq!(codes(&diagnostics), Vec::<&str>::new(), "{diagnostics:?}");
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn each_wrong_kind_reference_is_its_own_e022_in_written_order() {
    let (_, diagnostics) = build(
        "behavior b1 \"B1\" { features [b3, b2] }\nbehavior b2 \"B2\" { }\nbehavior b3 \"B3\" { }\n",
        &["behavior", "feature"],
        &[],
    );
    assert_eq!(codes(&diagnostics), ["E022", "E022"], "{diagnostics:?}");
    assert!(diagnostics[0].message.contains("'b3'"));
    assert!(diagnostics[1].message.contains("'b2'"));
}

#[test]
fn e022_names_the_target_the_field_the_kinds_and_the_source() {
    let (_, diagnostics) = build(WRONG_KIND, &["behavior", "feature"], &[]);
    let message = &diagnostics[0].message;
    for word in ["b2", "features", "behavior", "feature", "b1"] {
        assert!(message.contains(word), "{word}: {message}");
    }
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a reference to an existing entity of another kind than its field's target kind is E022 at the reference"
)]
fn a_wrong_kind_id_of_an_absent_kind_is_e022_naming_its_provider() {
    let (graph, diagnostics) = build(
        WRONG_KIND,
        &["behavior"],
        &[("feature", "@specforge/product")],
    );
    assert_eq!(codes(&diagnostics), ["E022"], "{diagnostics:?}");
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some(
            "name a feature here (enable the extension that declares it with `specforge add @specforge/product`), or drop the reference"
        )
    );
    assert_eq!(graph.edge_count(), 1);

    let (_, diagnostics) = build(WRONG_KIND, &["behavior"], &[]);
    assert_eq!(
        diagnostics[0].suggestion.as_deref(),
        Some(
            "name a feature here (find an extension that declares it with `specforge search feature`), or drop the reference"
        )
    );
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a quoted reference is written at its ID, inside its quotes"
)]
fn a_quoted_reference_is_e003_and_e022_at_its_id() {
    let source = "behavior b1 \"B1\" {\n  features [\"ghost\"]\n  peers \"ghost\"\n}\n\n\
                  behavior b2 \"B2\" {\n  features [\"b1\"]\n}\n";
    let (_, diagnostics) = build(source, &["behavior", "feature"], &[]);
    let mut at: Vec<String> = diagnostics
        .iter()
        .filter_map(|d| {
            let s = d.span.as_ref()?;
            Some(format!(
                "{} {}:{}-{}",
                d.code, s.start_line, s.start_col, s.end_col
            ))
        })
        .collect();
    at.sort();
    // A list item, a single value coerced to a list, and a mistyped id (b1 is a behavior).
    assert_eq!(
        at,
        ["E003 2:14-19", "E003 3:10-15", "E022 7:14-16"],
        "{diagnostics:?}"
    );
}

fn placed(graph: &mut Graph, path: &str, kind: &str, scope: Option<&str>) {
    use specforge_common::{SourceSpan, Sym};
    use specforge_graph::Node;
    use specforge_parser::{EntityId, EntityKind, FieldMap};
    graph.add_node(Node {
        id: EntityId {
            raw: Sym::new(path),
        },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: None,
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("p.spec"),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
        },
        methods: Vec::new(),
        obligations: Vec::new(),
        verdicts: Vec::new(),
        scope: scope.map(Sym::new),
    });
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a surface finds a numeric id whatever its zero padding, and a reference in the spec text does not"
)]
fn a_numeric_segment_matches_its_padded_form() {
    use specforge_graph::Found;

    let mut graph = Graph::new();
    placed(&mut graph, "v1.0", "milestone", None);
    for phase in ["03", "02.1", "03a"] {
        placed(&mut graph, &format!("v1.0/{phase}"), "phase", Some("v1.0"));
    }
    placed(&mut graph, "v1.0/03/03-01", "plan", Some("v1.0/03"));
    let one = |spelling: &str| match graph.find(spelling) {
        Found::One(path) => Some(path.to_string()),
        _ => None,
    };
    assert_eq!(one("3").as_deref(), Some("v1.0/03"));
    assert_eq!(one("v1.0/3").as_deref(), Some("v1.0/03"));
    assert_eq!(one("003").as_deref(), Some("v1.0/03"));
    assert_eq!(one("2.1").as_deref(), Some("v1.0/02.1"));
    assert_eq!(one("3a").as_deref(), Some("v1.0/03a"));
    for none in ["3.1", "2.01", "3-1", "4"] {
        assert!(matches!(graph.find(none), Found::None(_)), "{none}");
    }
    // The exact spelling is still found first.
    assert_eq!(one("03").as_deref(), Some("v1.0/03"));

    placed(&mut graph, "v2.0", "milestone", None);
    placed(&mut graph, "v2.0/03", "phase", Some("v2.0"));
    assert!(matches!(graph.find("3"), Found::Ambiguous(paths) if paths.len() == 2));
    assert_eq!(
        graph.find("v2.0/3"),
        Found::One(specforge_common::Sym::new("v2.0/03"))
    );

    // A reference in the spec's text reads its name exactly.
    let (_, diagnostics) = build(
        "behavior 03 \"Three\" {}\n\nbehavior b \"B\" {\n  peers [3]\n}\n",
        &["behavior"],
        &[],
    );
    assert!(
        diagnostics.iter().any(|d| d.code == "E003"),
        "{diagnostics:?}"
    );
}

#[specforge_test(
    behavior = "link_entity_references",
    verify = "a rooted spelling is never a suffix, numeric or not"
)]
fn a_rooted_spelling_is_never_a_suffix() {
    use specforge_graph::Found;

    let mut graph = Graph::new();
    placed(&mut graph, "v1.0", "milestone", None);
    placed(&mut graph, "v1.0/03", "phase", Some("v1.0"));
    placed(&mut graph, "v1.0/03/03-01", "plan", Some("v1.0/03"));
    for rooted in ["/3", "/03", "/03-01", "/01-01"] {
        assert!(matches!(graph.find(rooted), Found::None(_)), "{rooted}");
    }
    assert_eq!(
        graph.find("/v1.0/3"),
        Found::One(specforge_common::Sym::new("v1.0/03"))
    );
    // A root-level phase is still found by its padded rooted spelling.
    placed(&mut graph, "04", "phase", None);
    assert_eq!(
        graph.find("/4"),
        Found::One(specforge_common::Sym::new("04"))
    );
    assert!(matches!(graph.find("3"), Found::One(_)));
}
