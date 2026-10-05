//! Navigation (`specforge_ops::navigate`) over compiled projects: every
//! position comes from the parser, as the LSP and MCP see it.

use specforge_common::SourceSpan;
use specforge_ops::navigate::{Direction, Navigator, Occurrence, Precision, ReferenceQuery, Role};
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_test::prelude::*;
use tempfile::TempDir;

/// A project of `files` with `extensions`, compiled with its extensions
/// loaded.
pub struct Compiled {
    _dir: TempDir,
    pub project: CompiledProject,
}

pub fn compile(extensions: &[&str], files: &[(&str, &str)]) -> Compiled {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        serde_json::json!({"name": "nav", "extensions": extensions}).to_string(),
    )
    .unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    let runtime = specforge_component::project_runtime(dir.path());
    let project = CompiledProject::compile(dir.path(), Some(&runtime));
    let unloaded: Vec<_> = project
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == "I002")
        .collect();
    assert!(
        unloaded.is_empty(),
        "the extensions did not load: {unloaded:?}"
    );
    Compiled { _dir: dir, project }
}

impl Compiled {
    /// A navigator reading the project's files from disk.
    pub fn navigator(&self) -> Navigator<'_, impl Fn(&str) -> Option<String> + '_> {
        let spec_root = &self.project.env.spec_root;
        Navigator::new(ProjectView::of(&self.project), move |file| {
            std::fs::read_to_string(spec_root.join(file)).ok()
        })
    }

    /// A navigator reading `text_of` instead of the files.
    pub fn navigator_over<F: Fn(&str) -> Option<String>>(&self, text_of: F) -> Navigator<'_, F> {
        Navigator::new(ProjectView::of(&self.project), text_of)
    }
}

/// `"file L:C-L:C"`.
pub fn at(span: &SourceSpan) -> String {
    format!(
        "{} {}:{}-{}:{}",
        span.file, span.start_line, span.start_col, span.end_line, span.end_col
    )
}

/// Each occurrence as `"file L:C-L:C holder->target field"`.
pub fn keys(occurrences: &[Occurrence]) -> Vec<String> {
    occurrences
        .iter()
        .map(|o| {
            format!(
                "{} {}->{} {}",
                at(&o.span),
                o.holder,
                o.target,
                o.field.map_or("-".to_string(), |f| f.to_string())
            )
        })
        .collect()
}

pub const SOFTWARE: &[&str] = &["@specforge/software"];

pub const LIMIT: &str = "invariant session_limit \"Limit\" {\n  guarantee \"x\"\n}\n";
pub const LOGIN: &str = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n\n\
                         behavior logout \"Logout\" {\n  invariants [sesion_limit]\n}\n";

fn nav() -> Compiled {
    compile(SOFTWARE, &[("limit.spec", LIMIT), ("login.spec", LOGIN)])
}

const INCOMING: ReferenceQuery = ReferenceQuery {
    direction: Direction::Incoming,
    include_declaration: false,
};

const WITH_DECLARATION: ReferenceQuery = ReferenceQuery {
    direction: Direction::Incoming,
    include_declaration: true,
};

#[specforge_test(
    behavior = "find_all_references",
    verify = "find-refs returns all reference sites"
)]
fn references_are_every_field_that_names_the_entity() {
    let p = compile(
        SOFTWARE,
        &[
            ("limit.spec", LIMIT),
            (
                "login.spec",
                "behavior login \"Login\" {\n  invariants [session_limit]\n}\n\
                 behavior audit \"Audit\" {\n  invariants [session_limit]\n}\n",
            ),
        ],
    );
    let refs = p.navigator().references("session_limit", INCOMING).unwrap();
    assert_eq!(
        keys(&refs),
        [
            "login.spec 2:15-2:28 login->session_limit invariants",
            "login.spec 5:15-5:28 audit->session_limit invariants",
        ]
    );
    assert!(refs.iter().all(|o| o.role == Role::Reference));
}

#[specforge_test(
    behavior = "find_all_references",
    verify = "find-refs includes the declaration site"
)]
fn the_declaration_is_included_on_request() {
    let p = nav();
    let refs = p
        .navigator()
        .references("session_limit", WITH_DECLARATION)
        .unwrap();
    assert_eq!(
        keys(&refs),
        [
            "limit.spec 1:11-1:24 session_limit->session_limit -",
            "login.spec 2:15-2:28 login->session_limit invariants",
        ]
    );
    assert_eq!(refs[0].role, Role::Declaration);
}

#[specforge_test(
    behavior = "find_all_references",
    verify = "find-refs omits the declaration when the request excludes it"
)]
fn the_declaration_is_left_out_otherwise() {
    let p = nav();
    let refs = p.navigator().references("session_limit", INCOMING).unwrap();
    assert_eq!(
        keys(&refs),
        ["login.spec 2:15-2:28 login->session_limit invariants"]
    );
}

#[specforge_test(
    behavior = "find_all_references",
    verify = "find-refs across multiple files"
)]
fn references_come_from_every_file() {
    let p = compile(
        SOFTWARE,
        &[
            ("limit.spec", LIMIT),
            (
                "a.spec",
                "behavior a_one \"A\" {\n  invariants [session_limit]\n}\n",
            ),
            (
                "b.spec",
                "behavior b_one \"B\" {\n  invariants [session_limit]\n}\n",
            ),
        ],
    );
    let refs = p
        .navigator()
        .references("session_limit", WITH_DECLARATION)
        .unwrap();
    let files: Vec<&str> = refs.iter().map(|o| o.span.file.as_str()).collect();
    assert_eq!(files, ["a.spec", "b.spec", "limit.spec"]);
}

#[specforge_test(
    behavior = "find_all_references",
    verify = "find-refs excludes what the entity itself references"
)]
fn what_an_entity_refers_to_is_not_a_reference_to_it() {
    let p = nav();
    let nav = p.navigator();
    assert_eq!(nav.references("login", INCOMING).unwrap(), []);
    let outgoing = nav
        .references(
            "login",
            ReferenceQuery {
                direction: Direction::Outgoing,
                include_declaration: false,
            },
        )
        .unwrap();
    assert_eq!(
        keys(&outgoing),
        ["login.spec 2:15-2:28 login->session_limit invariants"]
    );
    let both = nav
        .references(
            "login",
            ReferenceQuery {
                direction: Direction::Both,
                include_declaration: true,
            },
        )
        .unwrap();
    assert_eq!(
        keys(&both),
        [
            "login.spec 1:10-1:15 login->login -",
            "login.spec 2:15-2:28 login->session_limit invariants",
        ]
    );
}

#[specforge_test(
    behavior = "find_all_references",
    verify = "each reference is the identifier token as written"
)]
fn a_reference_is_its_token() {
    let p = compile(
        SOFTWARE,
        &[
            ("limit.spec", LIMIT),
            (
                "login.spec",
                "behavior login \"session_limit\" {\n  invariants [  session_limit ]\n}\n",
            ),
        ],
    );
    let refs = p.navigator().references("session_limit", INCOMING).unwrap();
    assert_eq!(
        keys(&refs),
        ["login.spec 2:17-2:30 login->session_limit invariants"]
    );
    assert_eq!(refs[0].precision, Precision::Token);
}

#[specforge_test(
    behavior = "go_to_definition",
    verify = "go-to-def navigates to entity declaration"
)]
fn the_definition_is_the_declaration() {
    let p = nav();
    let definition = p.navigator().definition("session_limit").unwrap();
    assert_eq!(at(&definition.block), "limit.spec 1:1-3:2");
    assert_eq!(definition.kind, "invariant");
}

#[specforge_test(
    behavior = "go_to_definition",
    verify = "go-to-def on non-existent ID returns no result"
)]
fn an_unknown_id_has_no_definition() {
    let p = nav();
    let error = p.navigator().definition("nope").unwrap_err();
    assert_eq!(error.code, specforge_ops::navigate::NOT_FOUND);
    // A misspelled reference is not an entity either.
    assert!(p.navigator().definition("sesion_limit").is_err());
}

#[specforge_test(behavior = "go_to_definition", verify = "go-to-def works across files")]
fn a_reference_leads_to_its_definition_in_another_file() {
    let p = nav();
    let nav = p.navigator();
    let reference = nav.occurrence_at("login.spec", 2, 20).unwrap();
    assert_eq!(reference.target, "session_limit");
    let definition = nav.definition(reference.target.as_str()).unwrap();
    assert_eq!(at(&definition.name), "limit.spec 1:11-1:24");
}

#[specforge_test(
    behavior = "go_to_definition",
    verify = "the definition's selection is the entity's name token"
)]
fn the_definition_selects_the_name() {
    let p = compile(
        SOFTWARE,
        &[(
            "limit.spec",
            "// a comment naming session_limit\ninvariant   session_limit \"session_limit\" {\n  guarantee \"x\"\n}\n",
        )],
    );
    let definition = p.navigator().definition("session_limit").unwrap();
    assert_eq!(at(&definition.name), "limit.spec 2:13-2:26");
    assert_eq!(definition.precision, Precision::Token);
}

#[test]
fn derived_references_are_the_names_in_types_and_signatures() {
    let p = compile(
        SOFTWARE,
        &[(
            "store.spec",
            "type Status = open | done\n\
             type Task \"T\" {\n  status Status\n  title string\n}\n\
             port store \"Store\" {\n  direction outbound\n  method save(task: Task) -> Status\n}\n",
        )],
    );
    let nav = p.navigator();
    assert_eq!(
        keys(&nav.references("Status", INCOMING).unwrap()),
        [
            "store.spec 3:10-3:16 Task->Status composed_types",
            "store.spec 8:30-8:36 store->Status types",
        ]
    );
    assert_eq!(
        keys(&nav.references("Task", INCOMING).unwrap()),
        ["store.spec 8:21-8:25 store->Task types"]
    );
}

#[test]
fn strings_comments_and_verify_texts_are_not_occurrences() {
    let p = compile(
        &["@specforge/software", "@specforge/testing"],
        &[(
            "a.spec",
            "invariant session_limit \"session_limit cap\" {\n  guarantee \"session_limit is never exceeded\"\n}\n\
             behavior login \"Login\" {\n  invariants [session_limit]\n  // keeps session_limit\n  verify unit \"login respects session_limit\"\n}\n",
        )],
    );
    let refs = p
        .navigator()
        .references("session_limit", WITH_DECLARATION)
        .unwrap();
    assert_eq!(
        keys(&refs),
        [
            "a.spec 1:11-1:24 session_limit->session_limit -",
            "a.spec 5:15-5:28 login->session_limit invariants",
        ]
    );
}

#[test]
fn a_stale_text_gives_entity_precision() {
    let p = nav();
    // The file changed since the compile: the token is not where the
    // graph says.
    let nav = p.navigator_over(|file| match file {
        "login.spec" => Some(format!("\n{LOGIN}")),
        "limit.spec" => Some(LIMIT.to_string()),
        _ => None,
    });
    let refs = nav.references("session_limit", INCOMING).unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].precision, Precision::Entity);
    assert_eq!(
        at(&refs[0].span),
        "login.spec 1:1-3:2",
        "the holder's block"
    );

    // Unreadable text: the same fallback, the declaration included.
    let unreadable = p.navigator_over(|_| None);
    let refs = unreadable
        .references("session_limit", WITH_DECLARATION)
        .unwrap();
    assert!(refs.iter().all(|o| o.precision == Precision::Entity));
    assert_eq!(
        unreadable.definition("session_limit").unwrap().precision,
        Precision::Entity
    );
}

#[test]
fn the_occurrence_at_a_position_is_the_token_there() {
    let p = nav();
    let nav = p.navigator();
    let declaration = nav.occurrence_at("limit.spec", 1, 11).unwrap();
    assert_eq!(declaration.role, Role::Declaration);
    assert_eq!(at(&declaration.span), "limit.spec 1:11-1:24");
    // Just past the token's end still names it.
    assert_eq!(
        nav.occurrence_at("limit.spec", 1, 24).unwrap().target,
        "session_limit"
    );
    let reference = nav.occurrence_at("login.spec", 2, 15).unwrap();
    assert_eq!(reference.role, Role::Reference);
    assert_eq!(reference.holder, "login");
    // The title, a keyword, and an unresolved reference are no occurrence.
    assert_eq!(nav.occurrence_at("limit.spec", 1, 27), None);
    assert_eq!(nav.occurrence_at("limit.spec", 1, 3), None);
    assert_eq!(nav.occurrence_at("login.spec", 6, 16), None);
}
