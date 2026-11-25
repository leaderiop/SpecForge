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

/// Each item of the package `app`'s file `file` holding `text`, with its test id: every one has the attribute `test`.
fn tests_in(file: &str, text: &str) -> Vec<(String, Option<String>)> {
    let facts = collected(&[(file, text)]);
    items(&facts)
        .into_iter()
        .map(|id| {
            let (attrs, test_id) = marks(&facts, id);
            assert_eq!(attrs, json!(["test"]), "{id}");
            (
                id.to_string(),
                test_id.map(|t| t.as_str().unwrap().to_string()),
            )
        })
        .collect()
}

/// `(item id, test id)` of `test/b.test.js` of the package `app`, by the item's path.
fn test_item(path: &str, test_id: Option<&str>) -> (String, Option<String>) {
    (
        format!("js:app/test/b.test.js::{path}"),
        test_id.map(|name| format!("js:app/test/b.test.js#{name}")),
    )
}

#[specforge_test(requirement = "FEATURE-056")]
fn a_suite_names_its_tests_and_carries_no_test_id() {
    let text = r#"
import { describe, it } from "node:test";
describe("board", () => {
  describe("inner", () => {
    it("adds", () => {});
  });
});
"#;
    assert_eq!(
        tests_in("test/b.test.js", text),
        [
            test_item("board", None),
            test_item("board.inner", None),
            test_item("board.inner.adds", Some("board > inner > adds")),
        ]
    );
}

#[specforge_test(requirement = "FEATURE-056")]
fn a_test_with_a_subtest_carries_no_test_id() {
    let text = r#"
test("parent", async (t) => {
  await t.test("child", () => {});
  assert.ok(/x/.test("x"));
});
"#;
    assert_eq!(
        tests_in("test/b.test.js", text),
        [
            test_item("parent", None),
            test_item("parent.child", Some("parent > child")),
        ]
    );
}

#[specforge_test(requirement = "FEATURE-056")]
fn the_runner_shorthands_are_tests() {
    let text = r#"
test.skip("s");
test.todo("t");
it.only("o", () => {});
describe.skip("d", () => {});
"#;
    assert_eq!(
        tests_in("test/b.test.js", text),
        [
            test_item("d", None),
            test_item("o", Some("o")),
            test_item("s", Some("s")),
            test_item("t", Some("t")),
        ]
    );
}

#[specforge_test(requirement = "FEATURE-056")]
fn a_dynamic_name_mints_nothing() {
    let text = r#"
const name = "n", b = "b", y = 1;
test(name, () => {});
test("a" + b, () => {});
test(`x ${y}`, () => {});
test.each([1])("n %s", () => {});
it.concurrent("c", () => {});
test("no callback");
regex.test("text");
describe(name, () => {
  test("inside", () => {});
});
describe.each([1])("each %s", () => {
  test("inside each", () => {});
});
"#;
    assert_eq!(tests_in("test/b.test.js", text), []);
}

#[specforge_test(requirement = "FEATURE-056")]
fn a_template_without_substitution_and_escapes_are_decoded() {
    let text = r#"
test(`plain template`, () => {});
test("quote \" and A", () => {});
"#;
    assert_eq!(
        tests_in("test/b.test.js", text),
        [
            test_item("plain template", Some("plain template")),
            test_item("quote \" and A", Some("quote \" and A")),
        ]
    );
}

#[specforge_test(requirement = "FEATURE-056")]
fn a_test_call_outside_a_test_file_is_ignored() {
    let facts = collected(&[("src/helper.js", "test(\"x\", () => {});\n")]);
    assert_eq!(items(&facts), Vec::<&str>::new());
}

#[specforge_test(requirement = "FEATURE-056")]
fn a_typescript_test_file_is_read_by_its_grammar() {
    let text = "it(\"typed\", (): void => {});\nconst n = <number>1;\n";
    let (out, asked) = run(host(&[APP, ("test/b.test.ts", text)]), &[]);
    assert_eq!(
        asked_paths(&asked),
        [("typescript", vec!["test/b.test.ts"])]
    );
    let facts = out.unwrap();
    assert_eq!(
        marks(&facts, "js:app/test/b.test.ts::typed").1,
        Some(json!("js:app/test/b.test.ts#typed"))
    );
}
