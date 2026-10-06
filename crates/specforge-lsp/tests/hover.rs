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
    Some(specforge_lsp::hover::entity(&facts, &[], false))
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
    // A SymbolKind name, for document symbols: never printed in the hover.
    behavior.declared.lsp_icon = Some("Method".into());

    let text = entity_hover(&g, declaring(vec![behavior]), "login").unwrap();
    assert!(text.starts_with("**"), "editor-neutral header:\n{text}");
    let header = text.split("\n\n---\n\n").next().unwrap();
    assert_eq!(
        header,
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
        let text = specforge_lsp::hover::entity(&facts, &[], false);
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

// -- headline, coverage and the entity's diagnostics ------------------------

/// The W004 rule requiring `kind`'s entities to declare obligations.
fn obligations_rule(
    kind: &str,
) -> (
    specforge_registry::validation_engine::ValidationRulePattern,
    String,
) {
    use specforge_registry::validation_engine::{ValidationPatternKind, ValidationRulePattern};
    (
        ValidationRulePattern {
            code: "W004".into(),
            severity: specforge_common::Severity::Warning,
            message_template: "{kind} '{id}' is testable but declares no verify obligations".into(),
            check: ValidationPatternKind::NoVerifyStatements,
            target_kind: Some(kind.to_string()),
            edge_type: None,
            edge_peer_kind: None,
            field: Some("verify".into()),
            constraint: None,
            wasm_function: None,
        },
        "@t/soft".into(),
    )
}

/// A project on disk: `source` compiled with `registries`, rooted at a
/// temp directory where a test report can be recorded.
struct OnDisk {
    dir: tempfile::TempDir,
    graph: Graph,
    env: Environment,
    recorded: RecordedCoverage,
}

impl OnDisk {
    fn new(source: &str, registries: RegistryBuild) -> Self {
        let (graph, _) =
            specforge_graph::build_graph(&[specforge_parser::parse(source, "main.spec")]);
        OnDisk {
            dir: tempfile::TempDir::new().unwrap(),
            graph,
            env: Environment::with_registries(registries),
            recorded: RecordedCoverage::default(),
        }
    }

    fn report(&self, report: &str) {
        std::fs::write(self.dir.path().join("specforge-report.json"), report).unwrap();
    }

    fn view(&self) -> ProjectView<'_> {
        ProjectView::new(
            &self.graph,
            &self.env,
            Some(self.dir.path()),
            &self.recorded,
        )
    }

    /// The hover of `id`, nothing shown for the cursor.
    fn hover(&self, id: &str, rebuilding: bool) -> String {
        let facts = specforge_ops::inspect::inspect(&self.view(), id).unwrap();
        specforge_lsp::hover::entity(&facts, &[], rebuilding)
    }

    /// The hover's Coverage line of `id`, if it has one.
    fn coverage(&self, id: &str, rebuilding: bool) -> Option<String> {
        self.hover(id, rebuilding)
            .split("\n\n---\n\n")
            .find(|section| section.starts_with("**Coverage**"))
            .map(str::to_string)
    }
}

/// `behavior` (obligated) with a headline `contract`, `type` (obligated),
/// `constraint` (testable, no rule) and `note` (not testable).
fn coverage_registries() -> RegistryBuild {
    use specforge_registry::{FieldDescriptor, FieldRegistryEntry, ManifestFieldType};
    let mut build = declaring(vec![
        kind("behavior", "@t/soft", true),
        kind("type", "@t/soft", true),
        kind("constraint", "@t/gov", true),
        kind("note", "@t/doc", false),
    ]);
    build.rules.push(obligations_rule("behavior"));
    build.rules.push(obligations_rule("type"));
    build.fields.register(FieldRegistryEntry {
        kind_name: "behavior".into(),
        field_type: ManifestFieldType::String,
        source_extension: "@t/soft".into(),
        proof_role: None,
        declared: FieldDescriptor {
            name: "contract".into(),
            headline: true,
            normative: true,
            ..Default::default()
        },
    });
    build
}

const COVERAGE: &str = "\
behavior done \"Done\" {\n  verify unit \"first\"\n}\n\
behavior half \"Half\" {\n  verify unit \"first\"\n  verify unit \"second\"\n}\n\
type U = a | b\n\
constraint free \"Free\" {\n}\n\
note doc \"Doc\" {\n  verify unit \"read\"\n}\n\
note plain \"Plain\" {\n}\n";

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover shows the entity's coverage as specforge.inspect reports it"
)]
fn hover_shows_each_coverage_case() {
    let project = OnDisk::new(COVERAGE, coverage_registries());
    let line = |id: &str| project.coverage(id, false);

    // No report recorded.
    assert_eq!(
        line("done").as_deref(),
        Some("**Coverage** · `uncovered` · 0/1 obligations proven · no test report recorded")
    );

    project.report(
        r#"{"results": {
            "done": {"tests": [{"name": "t", "status": "pass", "verify": "first"}]},
            "half": {"tests": [
                {"name": "t1", "status": "pass", "verify": "first"},
                {"name": "t2", "status": "fail", "verify": "second"}]},
            "doc": {"tests": []}
        }}"#,
    );
    let rows = [
        (
            "done",
            Some("**Coverage** · `covered` · 1/1 obligations proven · 1 test"),
        ),
        (
            "half",
            Some("**Coverage** · `partial` · 1/2 obligations proven · 2 tests · 1 failing"),
        ),
        (
            "U",
            Some("**Coverage** · exempt: it owes none (a union or an exempting field)"),
        ),
        (
            "free",
            Some("**Coverage** · exempt: its kind need not declare obligations"),
        ),
        (
            "doc",
            Some(
                "**Coverage** · `uncovered` · 0/1 obligations proven · 0 tests · \
                 its kind is not testable, so it does not count",
            ),
        ),
        ("plain", None),
    ];
    for (id, expected) in rows {
        assert_eq!(line(id).as_deref(), expected, "{id}");
        // The line agrees with the read view MCP inspect renders.
        let facts = specforge_ops::inspect::inspect(&project.view(), id).unwrap();
        if let Some(expected) = expected
            && !facts.standing.exempt
        {
            let status =
                specforge_ops::coverage::STATUS.name_of(facts.coverage.as_ref().unwrap().status());
            assert!(expected.contains(&format!("`{status}`")), "{id}");
        }
    }

    // A report that cannot be read is shown, not a failed hover.
    project.report("{not json");
    let unreadable = line("done").unwrap();
    assert!(
        unreadable.starts_with("**Coverage** · the recorded test report cannot be read (E045): "),
        "{unreadable}"
    );
    assert_eq!(
        line("U").as_deref(),
        Some("**Coverage** · exempt: it owes none (a union or an exempting field)"),
        "the standing does not depend on the report"
    );
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover never states a coverage fact it cannot read: while the project rebuilds it says so"
)]
fn hover_says_coverage_is_unavailable_while_the_project_rebuilds() {
    let project = OnDisk::new(COVERAGE, coverage_registries());
    project.report(
        r#"{"results": {"done": {"tests": [{"name": "t", "status": "pass", "verify": "first"}]}}}"#,
    );
    for id in ["done", "U", "doc"] {
        assert_eq!(
            project.coverage(id, true).as_deref(),
            Some("**Coverage** · unavailable while the project rebuilds"),
            "{id}"
        );
    }
    assert_eq!(project.coverage("plain", true), None);
    // The standing badges stay.
    assert!(project.hover("done", true).contains("`testable`"));
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover shows the headline statement as its summary"
)]
fn hover_quotes_the_headline_and_fields_skip_it() {
    let source = "\
behavior one \"One\" {\n  contract \"The system MUST do one thing\"\n  verify unit \"it does\"\n}\n\
behavior three \"Three\" {\n  contract \"\"\"\n    The system MUST do a\n      and then b\n    and c\n  \"\"\"\n}\n\
note doc \"Doc\" {\n  verify unit \"read\"\n}\n";
    let project = OnDisk::new(source, coverage_registries());

    let one = project.hover("one", false);
    let header = one.split("\n\n---\n\n").next().unwrap();
    assert!(
        header.ends_with("\n\n> The system MUST do one thing"),
        "{header}"
    );
    assert!(!one.contains("`contract` ="), "Fields skips it:\n{one}");
    assert!(one.contains("- `verify` = unit: it does"), "{one}");

    let three = project.hover("three", false);
    let header = three.split("\n\n---\n\n").next().unwrap();
    assert!(
        header.ends_with("\n\n> The system MUST do a\n>   and then b\n> and c"),
        "a multi-line headline is quoted whole, dedented:\n{header}"
    );
    assert!(!three.contains("`contract` ="), "{three}");

    // A kind with no headline has no summary.
    let doc = project.hover("doc", false);
    assert!(!doc.contains("\n> "), "{doc}");
}

#[spec(
    behavior = "provide_extension_entity_hover",
    verify = "hover lists the entity's diagnostics the cursor's do not already show"
)]
fn hover_lists_the_diagnostics_not_already_shown() {
    use specforge_common::{Diagnostic, DiagnosticData};
    let mut g = Graph::new();
    g.add_node(node("alpha", "behavior", Some("A")));
    g.add_node(node("beta", "behavior", Some("B")));
    g.add_edge(edge("alpha", "beta", "depends_on"));
    g.add_edge(edge("beta", "alpha", "depends_on"));
    // Spanless: about the entities its data names.
    let cycle = Diagnostic::warning("W061", "reference cycle detected: alpha -> beta -> alpha")
        .with_data(DiagnosticData::ReferenceCycle {
            path: vec!["alpha".into(), "beta".into(), "alpha".into()],
        });
    let third_party =
        Diagnostic::warning("X900", "acme says no").with_data(DiagnosticData::Subject {
            entity: "beta".into(),
        });
    let reported = vec![cycle.clone(), third_party];
    let env = Environment::empty();
    let recorded = RecordedCoverage::default();
    let view = ProjectView::new(&g, &env, None, &recorded).reporting(&reported);
    let facts = specforge_ops::inspect::inspect(&view, "beta").unwrap();

    let text = specforge_lsp::hover::entity(&facts, &[], false);
    assert!(
        text.ends_with(
            "**Diagnostics** *(2)*\n\
             - [**W061**](https://github.com/leaderiop/SpecForge/blob/main/docs/diagnostics.md#w061) \
             reference cycle detected: alpha -> beta -> alpha\n\
             - **X900** acme says no"
        ),
        "{text}"
    );

    // What the cursor already shows is not listed again.
    let text = specforge_lsp::hover::entity(&facts, &[&cycle], false);
    assert!(
        text.ends_with("**Diagnostics** *(1)*\n- **X900** acme says no"),
        "{text}"
    );
    let all = [&reported[0], &reported[1]];
    let text = specforge_lsp::hover::entity(&facts, &all, false);
    assert!(!text.contains("**Diagnostics**"), "{text}");
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
