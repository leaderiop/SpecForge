//! A test file's test calls: each statically named one an item with the attribute `test`, a leaf test carrying the id
//! node-test mints for it (ADR 0393 D4).

use specforge_code_facts::TestId;

use super::*;

/// The taskboard fixture's test file and the JUnit report node recorded running it.
const TASKBOARD_TESTS: &str =
    include_str!("../../../../../gsd/parity/fixtures/taskboard/test/tasks.test.js");
const TASKBOARD_JUNIT: &str = include_str!("../../../../../gsd/parity/goldens/taskboard/junit.xml");

/// `(attrs, test_id)` of the item `id`.
fn marks(facts: &[Fact], id: &str) -> (Value, Option<Value>) {
    let fact = item(facts, id);
    (
        fact.fields["attrs"].clone(),
        fact.fields.get("test_id").cloned(),
    )
}

/// The value of the attribute `name` in an XML element's text.
fn attribute<'x>(element: &'x str, name: &str) -> &'x str {
    let start = element.find(&format!(" {name}=\"")).expect("the attribute") + name.len() + 3;
    let len = element[start..].find('"').expect("its closing quote");
    &element[start..start + len]
}

#[specforge_test(requirement = "FEATURE-056")]
fn the_taskboards_test_calls_carry_the_ids_node_test_mints() {
    let package = ("package.json", r#"{"name":"taskboard"}"#);
    let source = ("src/tasks.js", "export function addTask() {}\n");
    let alone = run(host(&[package, source]), &[]).0.unwrap();
    let facts = run(
        host(&[package, source, ("test/tasks.test.js", TASKBOARD_TESTS)]),
        &[],
    )
    .0
    .unwrap();
    // node-test's ids, from the recorded report: a top-level test's path is its name.
    let node_ids: Vec<String> = TASKBOARD_JUNIT
        .split("<testcase")
        .skip(1)
        .map(|case| {
            let path = [attribute(case, "name").to_string()];
            TestId::js("taskboard", attribute(case, "file"), &path).to_string()
        })
        .collect();
    assert_eq!(
        node_ids,
        [
            "js:taskboard/test/tasks.test.js#a task is added with the next id",
            "js:taskboard/test/tasks.test.js#a task is completed by its id",
            "js:taskboard/test/tasks.test.js#completing an unknown id throws",
        ]
    );
    let tests: Vec<&str> = items(&facts)
        .into_iter()
        .filter(|id| id.starts_with("js:taskboard/test/"))
        .collect();
    assert_eq!(
        tests,
        [
            "js:taskboard/test/tasks.test.js::a task is added with the next id",
            "js:taskboard/test/tasks.test.js::a task is completed by its id",
            "js:taskboard/test/tasks.test.js::completing an unknown id throws",
        ]
    );
    for (id, node_id) in tests.iter().zip(&node_ids) {
        assert_eq!(item(&facts, id).fields["kind"], json!(["function"]), "{id}");
        assert_eq!(
            marks(&facts, id),
            (json!(["test"]), Some(json!(node_id))),
            "{id}"
        );
    }
    // The source file's items are what they are without the test file.
    let source_items = |facts: &[Fact]| -> Vec<Fact> {
        facts
            .iter()
            .filter(|f| f.id.starts_with("js:taskboard/src/"))
            .cloned()
            .collect()
    };
    assert_eq!(source_items(&facts), source_items(&alone));
    assert_eq!(items(&alone), ["js:taskboard/src/tasks.js::addTask"]);
}
