//! The minting of test, item and package facts.

use super::{owner, strings};
use crate::*;
use specforge_code_junit::{cases, relative};
use specforge_extension_sdk::host::SyntaxPoint;
use specforge_extension_sdk::prelude::{Fact, FactLocation, TestStatus};
use specforge_test::prelude::*;

/// The taskboard twin's recorded report (`gsd/parity/goldens/taskboard/junit.xml`, kept by the JUnit library): three
/// top-level tests, files relative to the project.
const TASKBOARD: &str = include_str!("../../../junit/fixtures/taskboard.xml");

fn place(file: &str, line: usize) -> FactLocation {
    FactLocation {
        file: file.to_string(),
        start_line: line,
        start_col: 1,
        end_line: line,
        end_col: 9,
    }
}

/// A case of `id` that ran.
fn case(id: TestId, status: TestStatus, seconds: Option<f64>) -> TestCase {
    TestCase {
        seconds,
        ran: true,
        ..TestCase::unrun(id, status)
    }
}

/// Each fact's `(kind, id)`.
fn kinds_and_ids(facts: &[Fact]) -> Vec<(&str, &str)> {
    facts
        .iter()
        .map(|f| (f.kind.as_str(), f.id.as_str()))
        .collect()
}

#[test]
fn cases_of_one_id_keep_the_worst_status() {
    let adds = TestId::js("app", "test/a.test.js", &strings(&["adds"]));
    let lists = TestId::js("app", "test/a.test.js", &strings(&["lists"]));
    let facts = test_facts([
        case(adds.clone(), TestStatus::Passed, Some(0.002)),
        case(lists.clone(), TestStatus::Skipped, None),
        case(adds.clone(), TestStatus::Failed, Some(0.003)),
        case(lists, TestStatus::Passed, None),
    ]);
    assert_eq!(
        kinds_and_ids(&facts),
        [
            ("test", "js:app/test/a.test.js#adds"),
            ("test_run", "run:js:app/test/a.test.js#adds"),
            ("test", "js:app/test/a.test.js#lists"),
            ("test_run", "run:js:app/test/a.test.js#lists"),
        ]
    );
    assert_eq!(facts[0].fields["status"], "failed");
    assert_eq!(facts[1].fields["status"], "failed");
    assert_eq!(facts[1].fields["duration_ms"], 5);
    assert_eq!(facts[1].fields["test"], "js:app/test/a.test.js#adds");
    assert_eq!(facts[2].fields["status"], "skipped");
    assert!(!facts[3].fields.contains_key("duration_ms"));
}

#[test]
fn an_unrun_test_has_no_run() {
    let id = TestId::rust("toy", "tests::later");
    let facts = test_facts([TestCase::unrun(id, TestStatus::Skipped)]);
    assert_eq!(kinds_and_ids(&facts), [("test", "rust:toy#tests::later")]);
    assert_eq!(facts[0].fields["status"], "skipped");
}

#[test]
fn a_rust_test_splits_its_path_into_suite_and_name() {
    let id = TestId::rust("specforge-ops::golden", "read::tests::adds");
    let mut names = Names::new();
    names.insert("TR1".to_string(), vec![place("tests/golden.rs", 3)]);
    names.insert("TR2".to_string(), Vec::new());
    let facts = test_facts([TestCase {
        file: Some("crates/ops/tests/golden.rs".to_string()),
        names,
        at: vec![place("tests/golden.rs", 4)],
        ..case(id, TestStatus::Passed, None)
    }]);
    let test = &facts[0];
    assert_eq!(test.title.as_deref(), Some("read::tests::adds"));
    assert_eq!(test.fields["package"], "specforge-ops");
    assert_eq!(test.fields["binary"], "specforge-ops::golden");
    assert_eq!(test.fields["suite"], serde_json::json!(["read", "tests"]));
    assert_eq!(test.fields["name"], "adds");
    assert_eq!(test.fields["file"], "crates/ops/tests/golden.rs");
    assert_eq!(test.fields["names"], serde_json::json!(["TR1", "TR2"]));
    assert_eq!(test.named_at["names"]["TR1"], [place("tests/golden.rs", 3)]);
    assert_eq!(test.at, [place("tests/golden.rs", 4)]);
}

fn definition(id: &ItemId, kind: ItemKind, visibility: Visibility, at: FactLocation) -> Definition {
    Definition {
        id: id.clone(),
        package: PackageId::cargo("toy"),
        kind,
        visibility,
        file: at.file.clone(),
        at,
        signature: None,
        doc: None,
        attrs: Vec::new(),
        names: Names::new(),
        test_id: None,
    }
}

#[specforge_test(
    behavior = "cc_mint_code_ids",
    verify = "definitions of one id merge into one fact listing every location and kind"
)]
fn cfg_twins_merge_into_one_item_listing_every_location() {
    let id = ItemId::rust("toy", &strings(&["sys"]), None, "open");
    let unix = Definition {
        attrs: strings(&["cfg(unix)"]),
        doc: Some("Opens it.".to_string()),
        ..definition(
            &id,
            ItemKind::Function,
            Visibility::Public,
            place("src/sys.rs", 20),
        )
    };
    let windows = Definition {
        attrs: strings(&["cfg(windows)"]),
        ..definition(
            &id,
            ItemKind::Function,
            Visibility::Public,
            place("src/sys.rs", 9),
        )
    };
    let facts = item_facts([unix, windows]);
    assert_eq!(kinds_and_ids(&facts), [("item", "rust:toy::sys::open")]);
    let item = &facts[0];
    assert_eq!(item.at, [place("src/sys.rs", 9), place("src/sys.rs", 20)]);
    assert_eq!(item.fields["kind"], serde_json::json!(["function"]));
    assert_eq!(
        item.fields["attrs"],
        serde_json::json!(["cfg(unix)", "cfg(windows)"])
    );
    assert_eq!(item.fields["doc"], "Opens it.");
    assert_eq!(item.fields["file"], "src/sys.rs");
    assert_eq!(item.fields["package"], "pkg:cargo/toy");
}

#[test]
fn a_merged_item_lists_each_kind_and_its_widest_visibility() {
    let id = ItemId::rust("toy", &[], None, "parse");
    let facts = item_facts([
        definition(
            &id,
            ItemKind::Function,
            Visibility::Crate,
            place("src/lib.rs", 8),
        ),
        definition(
            &id,
            ItemKind::Module,
            Visibility::Private,
            place("src/lib.rs", 2),
        ),
        definition(
            &id,
            ItemKind::Macro,
            Visibility::Public,
            place("src/m.rs", 1),
        ),
    ]);
    assert_eq!(
        facts[0].fields["kind"],
        serde_json::json!(["module", "function", "macro"])
    );
    assert_eq!(facts[0].fields["visibility"], "public");
}

#[test]
fn a_rust_modifier_normalises_to_the_vocabulary() {
    for (modifier, visibility) in [
        (Some("pub"), "public"),
        (Some("pub(crate)"), "crate"),
        (Some("pub( crate )"), "crate"),
        (Some("pub(self)"), "private"),
        (Some("pub(super)"), "restricted"),
        (Some("pub(in crate::a)"), "restricted"),
        (None, "private"),
    ] {
        assert_eq!(
            Visibility::rust(modifier).as_str(),
            visibility,
            "{modifier:?}"
        );
    }
    assert_eq!(Visibility::js(true).as_str(), "public");
    assert_eq!(Visibility::js(false).as_str(), "private");
    assert_eq!(
        Visibility::NAMES,
        ["public", "crate", "restricted", "private"]
    );
}

#[test]
fn an_items_title_is_its_own_name() {
    let fmt = ItemId::rust("toy", &[], Some(&owner("Foo", Some("fmt::Display"))), "fmt");
    let add = ItemId::rust("toy", &[], Some(&owner("Board", None)), "add");
    let js = ItemId::js("app", "src/board.ts", &strings(&["Board", "add"]));
    let facts = item_facts([fmt, add, js].iter().map(|id| {
        definition(
            id,
            ItemKind::Method,
            Visibility::Public,
            place("src/lib.rs", 1),
        )
    }));
    let titles: Vec<(&str, &str)> = facts
        .iter()
        .map(|f| (f.id.as_str(), f.title.as_deref().unwrap()))
        .collect();
    assert_eq!(
        titles,
        [
            ("js:app/src/board.ts::Board.add", "add"),
            ("rust:toy::<Foo as Display>::fmt", "fmt"),
            ("rust:toy::Board::add", "add"),
        ]
    );
}

#[test]
fn a_signature_digest_ignores_whitespace_and_order() {
    let id = ItemId::rust("toy", &[], None, "f");
    let signed = |text: &str, line| Definition {
        signature: Some(text.to_string()),
        ..definition(
            &id,
            ItemKind::Function,
            Visibility::Public,
            place("src/lib.rs", line),
        )
    };
    let one = item_facts([signed("fn f(a: u8)", 1), signed("fn f(a: u16)", 5)]);
    let two = item_facts([
        signed("fn  f(a:\n u16)", 1),
        signed("fn f(a: u8)", 5),
        signed("fn f(a: u8)", 9),
    ]);
    let digest = one[0].fields["signature"].as_str().unwrap();
    assert!(
        digest.starts_with("sha256:") && digest.len() == 7 + 64,
        "{digest}"
    );
    assert_eq!(one[0].fields["signature"], two[0].fields["signature"]);
    let unsigned = item_facts([definition(
        &id,
        ItemKind::Function,
        Visibility::Public,
        place("a.rs", 1),
    )]);
    assert!(!unsigned[0].fields.contains_key("signature"));
}

#[test]
fn a_package_fact_names_its_dependencies_by_purl() {
    let package = Package {
        id: PackageId::cargo("toy"),
        name: "toy".to_string(),
        version: Some("0.1.0".to_string()),
        edition: Some("2021".to_string()),
        rust_version: None,
        engines: Vec::new(),
        features: strings(&["fast"]),
        targets: strings(&["lib:toy:src/lib.rs"]),
        path: "crates/toy".to_string(),
        depends_on: vec![PackageId::npm("@acme/core"), PackageId::cargo("base")],
        at: Some(place("crates/toy/Cargo.toml", 1)),
    };
    let fact = package.fact();
    assert_eq!(
        (fact.kind.as_str(), fact.id.as_str()),
        ("package", "pkg:cargo/toy")
    );
    assert_eq!(
        fact.fields["depends_on"],
        serde_json::json!(["pkg:cargo/base", "pkg:npm/%40acme/core"])
    );
    assert_eq!(
        fact.fields["targets"],
        serde_json::json!(["lib:toy:src/lib.rs"])
    );
    assert_eq!(fact.fields["path"], "crates/toy");
    assert!(!fact.fields.contains_key("rust_version") && !fact.fields.contains_key("engines"));
    assert_eq!(fact.at, [place("crates/toy/Cargo.toml", 1)]);
}

#[test]
fn facts_are_answered_in_id_order() {
    let ids = ["rust:toy::b", "rust:toy::a", "rust:toy::Board::c"];
    let facts = item_facts(ids.iter().map(|id| {
        definition(
            &id.parse().unwrap(),
            ItemKind::Function,
            Visibility::Public,
            place("a.rs", 1),
        )
    }));
    assert_eq!(
        facts.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
        ["rust:toy::Board::c", "rust:toy::a", "rust:toy::b"]
    );
}

#[test]
fn a_location_takes_the_readers_byte_columns() {
    let point = |line, col, byte_col| SyntaxPoint {
        byte: 0,
        line,
        col,
        byte_col,
    };
    assert_eq!(
        location("src/a.rs", point(3, 2, 4), point(3, 5, 9)),
        FactLocation {
            file: "src/a.rs".to_string(),
            start_line: 3,
            start_col: 4,
            end_line: 3,
            end_col: 9,
        }
    );
}

#[test]
fn the_taskboard_test_id_is_the_exit_string() {
    let found = cases(TASKBOARD).expect("the twin's report");
    let minted = test_facts(found.iter().map(|case| {
        let file = relative(case.file.as_deref().unwrap(), "/anywhere").unwrap();
        let path: Vec<String> = case.suites.iter().chain([&case.name]).cloned().collect();
        TestCase::of(TestId::js("taskboard", &file, &path), case, Some(file))
    }));
    let tests: Vec<(&str, Option<&str>)> = minted
        .iter()
        .filter(|f| f.kind == "test")
        .map(|f| (f.id.as_str(), f.fields["status"].as_str()))
        .collect();
    assert_eq!(
        tests,
        [
            (
                "js:taskboard/test/tasks.test.js#a task is added with the next id",
                Some("passed")
            ),
            (
                "js:taskboard/test/tasks.test.js#a task is completed by its id",
                Some("passed")
            ),
            (
                "js:taskboard/test/tasks.test.js#completing an unknown id throws",
                Some("passed")
            ),
        ]
    );
    assert_eq!(minted.len(), 6, "a test and a run per case");
}
