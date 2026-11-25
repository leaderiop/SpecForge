//! A test file's test calls: each statically named one an item with the attribute `test`, a test case carrying the id
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
            test_item("d", Some("d")),
            test_item("o", Some("o")),
            test_item("s", Some("s")),
            test_item("t", Some("t")),
        ]
    );
}

/// A test file and the `<testcase>` paths `node --test --test-reporter=junit` (node v26.13.0) wrote running it, each
/// path's names joined by ` > `: a `<testsuite>` is never among them.
const NODE_FILE: &str = r#"const { test, it, describe, suite } = require("node:test");
test("dyn", async (t) => { for (const n of ["a"]) await t.test(n, () => {}); });
test("ref", async (t) => { await t.test("x", fnRef); });
function fnRef() {}
test("opt", async (t) => { await t.test("q", { skip: true }); });
test("nocbsub", async (t) => { await t.test("w"); });
describe.skip("ds", () => { it("inner", () => {}); });
test.skip("ts", async (t) => { await t.test("sub", () => {}); });
test.todo("tt", async (t) => { await t.test("tsub", () => {}); });
describe.todo("dt", () => { it("dtinner", () => {}); });
describe("empty", () => {});
describe("nocb");
suite("suitenocb");
test("");
test("", () => {});
describe("", () => { it("inanon", () => {}); });
test("nested plain", async () => { test("np child", () => {}); });
test("nested it", async () => { await it("ni child", () => {}); });
test("optskip", { skip: true }, async (t) => { await t.test("os child", () => {}); });
describe("doptskip", { skip: true }, () => { it("dos child", () => {}); });
test("leafskip", { "skip": "why" }, () => {});
test("opttodo", { todo: true }, async (t) => { await t.test("ot child", () => {}); });
test("no callback");
test.todo("todo nocb");
describe("only holder", () => { it.only("o1", () => {}); it("o2", () => {}); });
test("crlf\
cont", () => {});
test("u \u0041", () => {});
test(`tpl
line`, () => {});
it.skip("itskip", () => { it("never"); });
"#;
const NODE_CASES: [&str; 28] = [
    "dyn > a",
    "ref > x",
    "opt > q",
    "nocbsub > w",
    "ds",
    "ts",
    "tt > tsub",
    "dt > dtinner",
    "empty",
    "nocb",
    "suitenocb",
    "<anonymous>",
    "<anonymous>",
    "<anonymous> > inanon",
    "nested plain > np child",
    "nested it > ni child",
    "optskip",
    "doptskip",
    "leafskip",
    "opttodo > ot child",
    "no callback",
    "todo nocb",
    "only holder > o1",
    "only holder > o2",
    "crlfcont",
    "u A",
    "tpl\nline",
    "itskip",
];

#[specforge_test(requirement = "FEATURE-056")]
fn every_test_id_is_a_test_case_node_reports() {
    let found = tests_in("test/b.test.js", NODE_FILE);
    let path = |id: &str| {
        id.strip_prefix("js:app/test/b.test.js#")
            .expect("the file's test id")
            .to_string()
    };
    for (item, test_id) in &found {
        if let Some(test_id) = test_id {
            assert!(
                NODE_CASES.contains(&path(test_id).as_str()),
                "{item}: node reports no <testcase> {test_id}"
            );
        }
    }
    assert_eq!(
        found,
        [
            test_item("crlfcont", Some("crlfcont")),
            // A suite holding nothing, or with no function, is a test case; one holding a test is a suite.
            test_item("doptskip", None),
            test_item("ds", Some("ds")),
            test_item("dt", None),
            test_item("dt.dtinner", Some("dt > dtinner")),
            // A test whose subtest is not minted (a dynamic name, a function reference, options only, no function)
            // is a suite all the same.
            test_item("dyn", None),
            test_item("empty", Some("empty")),
            test_item("itskip", Some("itskip")),
            test_item("leafskip", Some("leafskip")),
            test_item("nested it", None),
            test_item("nested it.ni child", Some("nested it > ni child")),
            test_item("nested plain", None),
            test_item("nested plain.np child", Some("nested plain > np child")),
            test_item("nocb", Some("nocb")),
            test_item("nocbsub", None),
            test_item("only holder", None),
            test_item("only holder.o1", Some("only holder > o1")),
            test_item("only holder.o2", Some("only holder > o2")),
            test_item("opt", None),
            // Options that may skip it: whether it or what it holds is reported is decided when it runs.
            test_item("optskip", None),
            // A todo test runs its function: what it holds is reported.
            test_item("opttodo", None),
            test_item("opttodo.ot child", Some("opttodo > ot child")),
            test_item("ref", None),
            test_item("suitenocb", Some("suitenocb")),
            test_item("todo nocb", Some("todo nocb")),
            test_item("tpl\nline", Some("tpl\nline")),
            // A skipped call is a test case, and nothing inside it runs.
            test_item("ts", Some("ts")),
            test_item("tt", None),
            test_item("tt.tsub", Some("tt > tsub")),
            test_item("u A", Some("u A")),
        ]
    );
}

#[specforge_test(requirement = "FEATURE-056")]
fn a_template_name_in_a_crlf_file_has_node_line_ends() {
    // node v26.13.0 named these `a&#10;b`, `cd` and `ef`: a template's line end is a line feed, a continuation none.
    let text = "test(`a\r\nb`, () => {});\r\ntest(\"c\\\r\nd\", () => {});\r\ntest(`e\\\r\nf`, () => {});\r\n";
    assert_eq!(
        tests_in("test/b.test.js", text),
        [
            test_item("a\nb", Some("a\nb")),
            test_item("cd", Some("cd")),
            test_item("ef", Some("ef")),
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
test("quote \" and \u0041", () => {});
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
