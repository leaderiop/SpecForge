//! The hover's markdown (`specforge_lsp::hover`): an entity's facts, read
//! by the inspect read view (`specforge_ops::inspect`) and rendered as the
//! backend renders them; a field's help; the diagnostics under the cursor.

use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_lsp::LineIndex;
use specforge_ops::view::ProjectView;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_project::Environment;
use specforge_project::coverage::RecordedCoverage;
use specforge_registry::{KindRegistryEntry, RegistryBuild};
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::Position;

pub fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: title.map(|t| t.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        methods: Vec::new(),
    }
}

fn edge(source: &str, target: &str, label: &str) -> Edge {
    Edge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

/// A kind `extension` declares.
fn kind(name: &str, extension: &str, testable: bool) -> KindRegistryEntry {
    KindRegistryEntry {
        kind_name: name.into(),
        source_extension: extension.into(),
        testable,
        supports_verify: testable,
        allowed_verify_kinds: vec![],
        lifecycle_field: None,
        ..Default::default()
    }
}

/// The registries of `kinds`.
fn declaring(kinds: Vec<KindRegistryEntry>) -> RegistryBuild {
    let mut build = RegistryBuild::default();
    for entry in kinds {
        build.kinds.register(entry);
    }
    build
}

/// The hover of the entity `id` of `graph`, compiled with `registries`
/// and no root: the inspect read view rendered as the backend does.
/// `None` for an entity the graph lacks.
pub fn entity_hover(graph: &Graph, registries: RegistryBuild, id: &str) -> Option<String> {
    let env = Environment::with_registries(registries);
    let recorded = RecordedCoverage::default();
    let view = ProjectView::new(graph, &env, None, &recorded);
    let facts = specforge_ops::inspect::inspect(&view, id).ok()?;
    Some(specforge_lsp::hover::entity(&facts))
}

/// [`entity_hover`] with no extension loaded.
pub fn plain_hover(graph: &Graph, id: &str) -> Option<String> {
    entity_hover(graph, RegistryBuild::default(), id)
}

#[spec(
    behavior = "hover_information",
    verify = "hover returns markdown-formatted content"
)]
fn hover_renders_markdown() {
    let mut g = Graph::new();
    g.add_node(node("my_type", "type", Some("My Type")));

    let text = plain_hover(&g, "my_type").expect("should produce hover");
    assert!(text.starts_with("**type** `my_type` — My Type"), "{text}");
    assert!(plain_hover(&g, "nonexistent").is_none());
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover shows entity kind and source extension"
)]
fn hover_shows_the_kind_and_its_extension() {
    let mut g = Graph::new();
    g.add_node(node("login", "behavior", Some("User Login")));
    let mut behavior = kind("behavior", "@specforge/software", true);
    behavior.declared.description = Some("A testable unit of system functionality".into());

    let text = entity_hover(&g, declaring(vec![behavior]), "login").unwrap();
    assert_eq!(
        text,
        "**behavior** `login` — User Login\n\n\
         A testable unit of system functionality\n\
         *@specforge/software* · `testable` · `verify`"
    );
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover shows testability for testable kinds"
)]
fn hover_shows_testability_from_the_standing() {
    let mut g = Graph::new();
    g.add_node(node("login", "behavior", Some("Login")));
    g.add_node(node("auth", "feature", Some("Auth")));
    let registries = || {
        declaring(vec![
            kind("behavior", "@specforge/software", true),
            kind("feature", "@specforge/product", false),
        ])
    };
    let env = Environment::with_registries(registries());
    let recorded = RecordedCoverage::default();
    let view = ProjectView::new(&g, &env, None, &recorded);
    for (id, testable) in [("login", true), ("auth", false)] {
        let facts = specforge_ops::inspect::inspect(&view, id).unwrap();
        assert_eq!(facts.standing.testable, testable, "{id}");
        let text = specforge_lsp::hover::entity(&facts);
        assert_eq!(text.contains("`testable`"), testable, "{id}:\n{text}");
    }
    assert!(
        entity_hover(&g, registries(), "auth")
            .unwrap()
            .ends_with("*@specforge/product*")
    );
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover shows reference count from graph"
)]
fn hover_shows_reference_counts_and_groups() {
    let mut g = Graph::new();
    g.add_node(node("create_user", "behavior", Some("Create User")));
    g.add_node(node("user_management", "feature", Some("User Management")));
    g.add_node(node("user_type", "type", None));
    g.add_node(node("data_integrity", "invariant", None));
    g.add_node(node("v1_launch", "milestone", None));
    g.add_edge(edge("create_user", "user_management", "features"));
    g.add_edge(edge("create_user", "user_type", "types"));
    g.add_edge(edge("data_integrity", "create_user", "enforced_by"));
    g.add_edge(edge("v1_launch", "create_user", "features"));

    let text = plain_hover(&g, "create_user").unwrap();
    assert_eq!(
        text,
        "**behavior** `create_user` — Create User\n\n---\n\n\
         **Refers to** *(2)*\n\
         - `features` → user_management\n\
         - `types` → user_type\n\n---\n\n\
         **Referenced by** *(2)*\n\
         - invariant via `enforced_by`: data_integrity\n\
         - milestone via `features`: v1_launch"
    );

    // Several references of one kind through one field are one group.
    let mut g = Graph::new();
    g.add_node(node("auth_feature", "feature", None));
    g.add_node(node("login", "behavior", None));
    g.add_node(node("logout", "behavior", None));
    g.add_node(node("v1_launch", "milestone", None));
    g.add_edge(edge("login", "auth_feature", "features"));
    g.add_edge(edge("logout", "auth_feature", "features"));
    g.add_edge(edge("v1_launch", "auth_feature", "features"));
    let text = plain_hover(&g, "auth_feature").unwrap();
    assert!(
        text.ends_with(
            "**Referenced by** *(3)*\n\
             - behavior via `features`: login, logout\n\
             - milestone via `features`: v1_launch"
        ),
        "{text}"
    );

    // No references, no sections.
    let mut g = Graph::new();
    g.add_node(node("orphan", "type", Some("Orphan Type")));
    assert_eq!(
        plain_hover(&g, "orphan").unwrap(),
        "**type** `orphan` — Orphan Type"
    );
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover content formatted as markdown"
)]
fn hover_without_registries_is_plain_markdown() {
    let mut g = Graph::new();
    g.add_node(node("g", "gizmo", Some("G")));
    // No extension line, no sections.
    assert_eq!(plain_hover(&g, "g").unwrap(), "**gizmo** `g` — G");
}

#[test]
fn hover_shows_field_values() {
    use specforge_parser::{FieldValue, SpannedRef};

    let mut login = node("login", "behavior", Some("Login"));
    login
        .fields
        .push(Sym::new("status"), FieldValue::Identifier("draft".into()));
    let ref_span = SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    };
    login.fields.push(
        Sym::new("invariants"),
        FieldValue::ReferenceList(vec![
            SpannedRef {
                id: "data_integrity".into(),
                span: ref_span.clone(),
            },
            SpannedRef {
                id: "auth_required".into(),
                span: ref_span,
            },
        ]),
    );
    login.fields.push(
        Sym::new("contract"),
        FieldValue::String("Given valid credentials, the user is authenticated".into()),
    );
    let mut g = Graph::new();
    g.add_node(login);

    let text = plain_hover(&g, "login").unwrap();
    assert!(
        text.contains("**Fields**"),
        "should have Fields section:\n{text}"
    );
    assert!(
        text.contains("`status` = `draft`"),
        "should show status value:\n{text}"
    );
    assert!(
        text.contains("`invariants` = [data_integrity, auth_required]"),
        "should show ref list:\n{text}"
    );
    assert!(
        text.contains("`contract` = \"Given valid credentials"),
        "should show contract string:\n{text}"
    );

    let mut g = Graph::new();
    g.add_node(node("bare", "behavior", Some("Bare")));
    let text = plain_hover(&g, "bare").unwrap();
    assert!(
        !text.contains("Fields"),
        "should not show Fields when entity has none:\n{text}"
    );
}

// -- hover_field_info (description) -------------------------------------------

#[test]
fn hover_shows_field_description() {
    use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};
    let mut reg = FieldRegistry::new();
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_type: ManifestFieldType::String,
        source_extension: "@specforge/software".into(),
        proof_role: None,
        declared: specforge_registry::FieldDescriptor {
            name: "contract".into(),
            description: Some("The behavioral contract this entity fulfills".into()),
            ..Default::default()
        },
    });

    let text = specforge_lsp::hover_field_info("contract", "behavior", &reg).unwrap();
    assert!(
        text.contains("The behavioral contract this entity fulfills"),
        "should show field description:\n{text}"
    );
}

// -- hover_field_info --------------------------------------------------------

fn make_field_registry() -> specforge_registry::FieldRegistry {
    use specforge_registry::{FieldRegistry, FieldRegistryEntry, ManifestFieldType};
    let mut reg = FieldRegistry::new();
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_type: ManifestFieldType::String,
        source_extension: "@specforge/software".into(),
        proof_role: None,
        declared: specforge_registry::FieldDescriptor {
            name: "contract".into(),
            ..Default::default()
        },
    });
    reg.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_type: ManifestFieldType::ReferenceList,
        source_extension: "@specforge/software".into(),
        proof_role: None,
        declared: specforge_registry::FieldDescriptor {
            name: "features".into(),
            edge: Some("BehaviorImplementsFeature".into()),
            target_kind: Some("feature".into()),
            required: true,
            ..Default::default()
        },
    });
    reg
}

#[test]
fn field_hover_shows_type_and_extension() {
    let reg = make_field_registry();
    let text = specforge_lsp::hover_field_info("contract", "behavior", &reg).unwrap();
    assert!(
        text.contains("`contract`"),
        "should show field name:\n{text}"
    );
    assert!(text.contains("string"), "should show type:\n{text}");
    assert!(
        text.contains("*@specforge/software*"),
        "should show extension:\n{text}"
    );
}

#[test]
fn field_hover_shows_target_kind_and_edge() {
    let reg = make_field_registry();
    let text = specforge_lsp::hover_field_info("features", "behavior", &reg).unwrap();
    assert!(
        text.contains("→ **feature**"),
        "should show target kind:\n{text}"
    );
    assert!(
        text.contains("Edge `BehaviorImplementsFeature`"),
        "should show edge type:\n{text}"
    );
    assert!(text.contains("*required*"), "should show required:\n{text}");
}

#[test]
fn field_hover_unknown_field_returns_none() {
    let reg = make_field_registry();
    assert!(specforge_lsp::hover_field_info("nonexistent", "behavior", &reg).is_none());
}

#[test]
fn field_hover_unknown_kind_returns_none() {
    let reg = make_field_registry();
    assert!(specforge_lsp::hover_field_info("contract", "unknown_kind", &reg).is_none());
}

fn diagnostic_at(
    code: &str,
    message: &str,
    line: usize,
    start: usize,
    end: usize,
) -> specforge_common::Diagnostic {
    specforge_common::Diagnostic {
        code: code.into(),
        severity: specforge_common::Severity::Error,
        message: message.into(),
        span: Some(SourceSpan {
            file: Sym::new("test.spec"),
            start_line: line,
            start_col: start,
            end_line: line,
            end_col: end,
        }),
        suggestion: None,
        data: None,
    }
}

/// The cursor section of the hover at `position`: the published
/// diagnostics there, rendered.
fn diagnostic_hover(
    published: &[specforge_common::Diagnostic],
    index: &LineIndex,
    position: Position,
) -> Option<String> {
    specforge_lsp::hover::diagnostics(&specforge_lsp::hover::diagnostics_at(
        published, index, position,
    ))
}

#[spec(
    behavior = "hover_diagnostic",
    verify = "hovering a diagnostic shows its catalogued title and explanation"
)]
fn hovering_a_diagnostic_shows_the_catalogue_entry() {
    let content = "behavior login \"Login\" {\n  types [ghost]\n}\n";
    // `ghost`: line 2, columns 10..15 (1-based).
    let diagnostics = [diagnostic_at(
        "E003",
        "unresolved reference 'ghost'",
        2,
        10,
        15,
    )];

    let md =
        diagnostic_hover(&diagnostics, &LineIndex::new(content), Position::new(1, 11)).unwrap();
    assert_eq!(
        md,
        "**E003** · Unresolved reference\n\nunresolved reference 'ghost'\n\n\
         A reference field names an entity ID that doesn't resolve to any declared entity. \
         Fix the typo or add the missing entity; a `did you mean` suggestion is included when \
         a close match exists.\n\n\
         [Documentation](https://github.com/leaderiop/SpecForge/blob/main/docs/diagnostics.md#e003)"
    );
    // Outside the range, nothing.
    assert_eq!(
        diagnostic_hover(&diagnostics, &LineIndex::new(content), Position::new(0, 3)),
        None
    );
}

#[spec(
    behavior = "hover_diagnostic",
    verify = "an uncatalogued diagnostic's hover shows its code and message only"
)]
fn an_uncatalogued_diagnostic_hover_shows_code_and_message() {
    let content = "behavior login \"Login\" {\n  types [ghost]\n}\n";
    // E901 is a third-party extension's code.
    let diagnostics = [diagnostic_at("E901", "acme says no", 2, 10, 15)];
    assert_eq!(
        diagnostic_hover(&diagnostics, &LineIndex::new(content), Position::new(1, 12)).as_deref(),
        Some("**E901**\n\nacme says no")
    );
}

// -- field values -------------------------------------------------------------

/// The `contract` line of the hover of a behavior whose contract is `value`.
fn contract_line(value: &str) -> String {
    use specforge_parser::FieldValue;
    let mut g = Graph::new();
    let mut long = node("long_one", "behavior", Some("Long"));
    long.fields
        .push(Sym::new("contract"), FieldValue::String(value.to_string()));
    g.add_node(long);
    let text = plain_hover(&g, "long_one").unwrap();
    text.lines()
        .find(|l| l.starts_with("- `contract`"))
        .unwrap_or_else(|| panic!("no contract line:\n{text}"))
        .to_string()
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "a long field value is cut at a character boundary"
)]
fn a_long_field_is_cut_at_a_character_boundary() {
    let value = format!("{}é{}", "a".repeat(119), "b".repeat(20));
    assert_eq!(
        contract_line(&value),
        format!("- `contract` = \"{}…\"", "a".repeat(119))
    );
    // ASCII is cut at 120 bytes, as before.
    let ascii = "x".repeat(200);
    assert_eq!(
        contract_line(&ascii),
        format!("- `contract` = \"{}…\"", "x".repeat(120))
    );
}
