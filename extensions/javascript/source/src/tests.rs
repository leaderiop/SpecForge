//! The pack over small projects, its query run by the host's own reader (`specforge-source`) behind the in-process
//! syntax import, under a counted deadline.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::{json, Value};
use specforge_extension_sdk::host::in_process::{serve, InProcessHost};
use specforge_extension_sdk::host::{
    HostError, NotParsed, SyntaxAnswer, SyntaxFile, SyntaxFiles, SyntaxOutcome, SyntaxRequest,
};
use specforge_javascript_workspace::PACKAGES;
use specforge_source::{Language, SourceQuery, COUNTED_DEADLINE};
use specforge_test::prelude::*;

use super::*;
use crate::files::LISTINGS;

mod marks;

/// Each request's language and files, in the order made.
type Asked = Vec<(String, SyntaxFiles)>;

/// The project's files in memory, parsed as the host parses them: with a cap, only the first `cap` files of an answer
/// are parsed and the rest `capped`; `short`, a glob's walk stops before the project's end.
#[derive(Default)]
struct Host {
    files: BTreeMap<String, String>,
    cap: Option<usize>,
    short: bool,
    asked: RefCell<Asked>,
}

impl InProcessHost for Host {
    fn syntax_parse(&self, request: &SyntaxRequest) -> Result<SyntaxAnswer, HostError> {
        self.asked
            .borrow_mut()
            .push((request.language.clone(), request.files.clone()));
        let language = Language::named(&request.language).expect("a grammar the reader has");
        let query = SourceQuery::compile(language, &request.query).expect("the query compiles");
        let (paths, short) = match &request.files {
            SyntaxFiles::Paths(paths) => (paths.clone(), false),
            SyntaxFiles::Glob(glob) => {
                let suffix = glob.strip_prefix("**/*").expect("an extension's glob");
                let matched = self.files.keys().filter(|p| p.ends_with(suffix));
                (matched.cloned().collect(), self.short)
            }
        };
        let capped = |at: usize| self.cap.is_some_and(|cap| at >= cap);
        let files = paths
            .iter()
            .enumerate()
            .map(|(at, path)| SyntaxFile {
                path: path.clone(),
                digest: None,
                outcome: match self.files.get(path) {
                    _ if capped(at) => SyntaxOutcome::NotParsed {
                        reason: NotParsed::Capped,
                    },
                    Some(text) => query.parse(text.as_bytes(), COUNTED_DEADLINE),
                    None => SyntaxOutcome::NotParsed {
                        reason: NotParsed::Missing,
                    },
                },
            })
            .collect();
        Ok(SyntaxAnswer {
            files,
            truncated: short || capped(paths.len().saturating_sub(1)),
            parsed: 0,
            cached: 0,
        })
    }
}

/// The input the host sends for `host`'s files: every `package.json` with its text, each listing the files of its
/// extension (none read past a listing named in `truncated`, which says so).
fn input(host: &Host, truncated: &[&str]) -> CollectInput {
    let mut files = serde_json::Map::new();
    let packages: Vec<Value> = host
        .files
        .iter()
        .filter(|(path, _)| path.rsplit('/').next() == Some("package.json"))
        .map(|(path, text)| json!({"path": path, "exists": true, "text": text}))
        .collect();
    files.insert(PACKAGES.to_string(), json!({"facts": packages}));
    for listing in &LISTINGS {
        let cut = truncated.contains(&listing.extension);
        let listed: Vec<Value> = host
            .files
            .keys()
            .filter(|path| !cut && path.rsplit_once('.').map(|(_, e)| e) == Some(listing.extension))
            .map(|path| json!({"path": path, "exists": true}))
            .collect();
        files.insert(
            listing.name.to_string(),
            json!({"facts": listed, "truncated": cut}),
        );
    }
    serde_json::from_value(json!({ "files": files })).unwrap()
}

/// What javascript-source answers over `files` (a listing of each extension in `truncated` cut by the host), and the
/// requests it made.
fn run(host: Host, truncated: &[&str]) -> (Result<Vec<Fact>, String>, Asked) {
    let input = input(&host, truncated);
    let host = Rc::new(host);
    let out = serve(host.clone(), || collect(&input)).map(|out| out.facts);
    let asked = host.asked.borrow().clone();
    (out, asked)
}

fn host(files: &[(&str, &str)]) -> Host {
    Host {
        files: files
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect(),
        ..Host::default()
    }
}

/// The root package `app`.
const APP: (&str, &str) = ("package.json", r#"{"name":"app"}"#);

/// The facts over `files`, the root package `app` among them.
fn collected(files: &[(&str, &str)]) -> Vec<Fact> {
    let mut all = vec![APP];
    all.extend_from_slice(files);
    run(host(&all), &[]).0.expect("javascript-source collects")
}

/// The item facts' ids.
fn items(facts: &[Fact]) -> Vec<&str> {
    facts
        .iter()
        .filter(|f| f.kind == "item")
        .map(|f| f.id.as_str())
        .collect()
}

fn item<'f>(facts: &'f [Fact], id: &str) -> &'f Fact {
    facts
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("no fact {id} among {:?}", items(facts)))
}

/// Each place of `fact`: `(file, start line, start col, end line, end col)`.
fn places(fact: &Fact) -> Vec<(&str, usize, usize, usize, usize)> {
    fact.at
        .iter()
        .map(|at| {
            (
                at.file.as_str(),
                at.start_line,
                at.start_col,
                at.end_line,
                at.end_col,
            )
        })
        .collect()
}

/// The paths each request named, by language (a glob as itself).
fn asked_paths(asked: &[(String, SyntaxFiles)]) -> Vec<(&str, Vec<&str>)> {
    asked
        .iter()
        .map(|(language, files)| {
            let paths = match files {
                SyntaxFiles::Paths(paths) => paths.iter().map(String::as_str).collect(),
                SyntaxFiles::Glob(glob) => vec![glob.as_str()],
            };
            (language.as_str(), paths)
        })
        .collect()
}

#[specforge_test(
    behavior = "js_declare_source_collector",
    verify = "javascript-source declares its collector, its inputs, its capability and its peer"
)]
fn javascript_source_declares_its_collector_its_inputs_and_its_peer() {
    let declaration = crate::specforge_extension_build().declaration();
    assert_eq!(declaration.handshake.name, "@javascript/source");
    let peers: Vec<(&str, &str, bool)> = declaration
        .handshake
        .peer_dependencies
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str(), p.optional))
        .collect();
    assert_eq!(peers, [("@code/core", "^1.1", false)]);
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.answers, ["item", "package"]);
    let globs: Vec<String> = LISTINGS.iter().map(|l| l.glob()).collect();
    assert_eq!(
        globs,
        [
            "**/*.ts", "**/*.mts", "**/*.cts", "**/*.tsx", "**/*.js", "**/*.jsx", "**/*.mjs",
            "**/*.cjs"
        ]
    );
    let mut inputs = vec![packages_input()];
    inputs.extend(LISTINGS.iter().map(|l| l.input()));
    assert_eq!(collector.inputs, inputs);
    assert_eq!(
        (&collector.capability, &collector.report),
        (&None, &None),
        "no command, no report"
    );
    let detect = collector.auto_detect.as_ref().expect("detected");
    assert_eq!(detect.file_patterns, ["package.json"]);
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(
        capabilities,
        json!([{
            "name": COLLECTOR,
            "port": "fs",
            "roots": ["project"],
            "ops": ["read"],
            "call_kinds": ["collect"],
            "why": "parse the project's JavaScript and TypeScript source with the host's reader",
        }])
    );
    for language in [Language::TypeScript, Language::Tsx] {
        assert!(
            SourceQuery::compile(language, QUERY).is_ok(),
            "{language:?}: {:?}",
            SourceQuery::compile(language, QUERY).err()
        );
    }
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "a file is read by the grammar its extension names"
)]
fn a_file_is_read_by_the_grammar_its_extension_names() {
    let files = [
        APP,
        ("src/a.ts", "const n = <number>value;\n"),
        ("src/b.mts", "export const b = 1;\n"),
        ("src/c.cts", "export const c = 1;\n"),
        ("src/d.d.ts", "export declare function d(): void;\n"),
        ("src/e.tsx", "export const E = () => <div />;\n"),
        ("src/f.js", "export const F = () => <span />;\n"),
        ("src/g.jsx", "export function g() {}\n"),
        ("src/h.mjs", "export const h = 1;\n"),
        ("src/i.cjs", "exports.i = 1;\n"),
    ];
    let (out, asked) = run(host(&files), &[]);
    assert_eq!(
        asked_paths(&asked),
        [
            (
                "tsx",
                vec![
                    "src/e.tsx",
                    "src/f.js",
                    "src/g.jsx",
                    "src/h.mjs",
                    "src/i.cjs"
                ]
            ),
            (
                "typescript",
                vec!["src/a.ts", "src/b.mts", "src/c.cts", "src/d.d.ts"]
            ),
        ]
    );
    let facts = out.unwrap();
    // `<number>value` is a cast only the typescript grammar reads; JSX only tsx.
    assert_eq!(
        items(&facts),
        [
            "js:app/src/a.ts::n",
            "js:app/src/b.mts::b",
            "js:app/src/c.cts::c",
            "js:app/src/d.d.ts::d",
            "js:app/src/e.tsx::E",
            "js:app/src/f.js::F",
            "js:app/src/g.jsx::g",
            "js:app/src/h.mjs::h",
            "js:app/src/i.cjs::i",
        ]
    );
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "a test file is never parsed"
)]
fn a_test_file_is_never_parsed() {
    let files = [
        APP,
        ("src/a.js", "export const a = 1;\n"),
        ("src/latest.js", "export const latest = 1;\n"),
        ("attest/x.js", "export const x = 1;\n"),
        ("src/a.test.js", "function helper() {}\n"),
        ("src/b.spec.ts", "function helper() {}\n"),
        ("src/c-test.js", "function helper() {}\n"),
        ("src/d_test.mjs", "function helper() {}\n"),
        ("src/test-e.js", "function helper() {}\n"),
        ("src/test.js", "function helper() {}\n"),
        ("test/f.js", "function helper() {}\n"),
        ("tests/g.ts", "function helper() {}\n"),
        ("src/__tests__/h.jsx", "function helper() {}\n"),
        ("src/__mocks__/i.cjs", "function helper() {}\n"),
    ];
    let (out, asked) = run(host(&files), &[]);
    assert_eq!(
        asked_paths(&asked),
        [("tsx", vec!["attest/x.js", "src/a.js", "src/latest.js"])]
    );
    assert_eq!(
        items(&out.unwrap()),
        [
            "js:app/attest/x.js::x",
            "js:app/src/a.js::a",
            "js:app/src/latest.js::latest"
        ]
    );
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "generated output is never parsed"
)]
fn generated_output_is_never_parsed() {
    let files = [
        APP,
        ("src/builder.js", "export const builder = 1;\n"),
        ("vendor.min.js", "var a=1;\n"),
        ("src/x.min.mjs", "var a=1;\n"),
        ("src/y.min.cjs", "var a=1;\n"),
        ("coverage/lcov-report/x.js", "var a=1;\n"),
        ("build/x.js", "var a=1;\n"),
        ("out/x.js", "var a=1;\n"),
        (".nuxt/x.js", "var a=1;\n"),
        (".svelte-kit/x.js", "var a=1;\n"),
        (".turbo/x.js", "var a=1;\n"),
        (".cache/x.js", "var a=1;\n"),
        ("storybook-static/x.js", "var a=1;\n"),
    ];
    let (out, asked) = run(host(&files), &[]);
    assert_eq!(asked_paths(&asked), [("tsx", vec!["src/builder.js"])]);
    assert_eq!(items(&out.unwrap()), ["js:app/src/builder.js::builder"]);
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "a file under no named package is not read"
)]
fn a_file_under_no_named_package_is_not_read() {
    let files = [
        ("package.json", r#"{"private":true}"#),
        ("packages/core/package.json", r#"{"name":"core"}"#),
        ("src/stray.js", "export const stray = 1;\n"),
        ("packages/core/a.js", "export const a = 1;\n"),
    ];
    let (out, asked) = run(host(&files), &[]);
    assert_eq!(asked_paths(&asked), [("tsx", vec!["packages/core/a.js"])]);
    assert_eq!(items(&out.unwrap()), ["js:core/a.js::a"]);
    let unnamed = [
        ("package.json", r#"{"private":true}"#),
        ("src/stray.js", "export const stray = 1;\n"),
    ];
    let (out, asked) = run(host(&unnamed), &[]);
    assert!(asked.is_empty(), "nothing is parsed: {asked:?}");
    assert_eq!(out.unwrap(), Vec::<Fact>::new());
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "a truncated listing parses its glob and filters the answer"
)]
fn a_truncated_listing_parses_its_glob_and_filters_the_answer() {
    let files = [
        APP,
        ("src/a.js", "export const a = 1;\n"),
        ("src/a.test.js", "export const helper = 1;\n"),
        ("src/b.js", "export const b = 1;\n"),
        ("src/c.ts", "export const c = 1;\n"),
        ("vendor.min.js", "var a=1;\n"),
    ];
    let (out, asked) = run(host(&files), &["js"]);
    assert_eq!(
        asked_paths(&asked),
        [("typescript", vec!["src/c.ts"]), ("tsx", vec!["**/*.js"])]
    );
    assert_eq!(
        items(&out.unwrap()),
        [
            "js:app/src/a.js::a",
            "js:app/src/b.js::b",
            "js:app/src/c.ts::c"
        ]
    );
    // The glob's answer capped after one file: the kept file it left is asked again by path.
    let capped = Host {
        cap: Some(1),
        ..host(&files)
    };
    let (out, asked) = run(capped, &["js"]);
    assert_eq!(
        asked_paths(&asked),
        [
            ("typescript", vec!["src/c.ts"]),
            ("tsx", vec!["**/*.js"]),
            ("tsx", vec!["src/b.js"])
        ]
    );
    assert_eq!(items(&out.unwrap()).len(), 3);
    // A walk that stopped short cannot be asked for the rest: the collector's error.
    let short = Host {
        short: true,
        ..host(&files)
    };
    let error = run(short, &["js"]).0.unwrap_err();
    assert!(
        error.contains("**/*.js") && error.contains("more files than the host walks"),
        "{error}"
    );
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "the files an answer's cap left unparsed are asked again"
)]
fn the_files_an_answers_cap_left_unparsed_are_asked_again() {
    let files = [
        APP,
        ("src/a.js", "export const a = 1;\n"),
        ("src/b.js", "export const b = 1;\n"),
        ("src/c.js", "export const c = 1;\n"),
    ];
    let (out, asked) = run(
        Host {
            cap: Some(1),
            ..host(&files)
        },
        &[],
    );
    assert_eq!(
        asked_paths(&asked),
        [
            ("tsx", vec!["src/a.js", "src/b.js", "src/c.js"]),
            ("tsx", vec!["src/b.js", "src/c.js"]),
            ("tsx", vec!["src/c.js"]),
        ]
    );
    assert_eq!(items(&out.unwrap()).len(), 3);
    let error = run(
        Host {
            cap: Some(0),
            ..host(&files)
        },
        &[],
    )
    .0
    .unwrap_err();
    assert_eq!(
        error,
        "src/a.js: its matches alone pass the host's answer cap"
    );
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "items are named after their package and file"
)]
fn items_are_named_after_their_package_and_file() {
    let facts = collected(&[
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
        (
            "packages/core/src/index.ts",
            "export function add(a: number) {\n  return a;\n}\n",
        ),
        ("src/main.js", "/* é */ function run() {}\n"),
    ]);
    assert_eq!(
        items(&facts),
        ["js:@acme/core/src/index.ts::add", "js:app/src/main.js::run"]
    );
    let add = item(&facts, "js:@acme/core/src/index.ts::add");
    assert_eq!(add.title.as_deref(), Some("add"));
    assert_eq!(add.fields["package"], "pkg:npm/%40acme/core");
    assert_eq!(add.fields["file"], "packages/core/src/index.ts");
    assert_eq!(add.fields["kind"], json!(["function"]));
    assert_eq!(places(add), [("packages/core/src/index.ts", 1, 8, 3, 2)]);
    // The column after `é` (two bytes) counts bytes, as a fact's place does.
    let run_ = item(&facts, "js:app/src/main.js::run");
    assert_eq!(places(run_), [("src/main.js", 1, 10, 1, 27)]);
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "a member is named by its class and a namespace member by its namespace"
)]
fn a_member_is_named_by_its_class_and_a_namespace_member_by_its_namespace() {
    let facts = collected(&[(
        "src/shapes.ts",
        r#"export class Board {
  add(x: number) {}
  size = 0;
  static make() {}
  #secret() {}
  'quoted-name'() {}
}
export interface Shape {
  area(): number;
  name: string;
}
export namespace Geo {
  export function dist() {}
  export namespace Inner {
    export const deep = 1;
  }
}
namespace A.B {
  export const c = 1;
}
"#,
    )]);
    let id = |path: &str| format!("js:app/src/shapes.ts::{path}");
    assert_eq!(
        items(&facts),
        [
            "A.B",
            "A.B.c",
            "Board",
            "Board.%23secret",
            "Board.add",
            "Board.make",
            "Board.quoted-name",
            "Board.size",
            "Geo",
            "Geo.Inner",
            "Geo.Inner.deep",
            "Geo.dist",
            "Shape",
            "Shape.area",
            "Shape.name",
        ]
        .map(id)
    );
    let kind = |path: &str| item(&facts, &id(path)).fields["kind"].clone();
    assert_eq!(kind("Board.add"), json!(["method"]));
    assert_eq!(kind("Board.size"), json!(["field"]));
    assert_eq!(kind("Shape.name"), json!(["field"]));
    assert_eq!(kind("Geo.Inner"), json!(["namespace"]));
    assert_eq!(kind("A.B"), json!(["namespace"]));
    assert_eq!(
        item(&facts, &id("Board.%23secret")).title.as_deref(),
        Some("#secret")
    );
    assert_eq!(item(&facts, &id("A.B")).title.as_deref(), Some("B"));
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "a local declaration is not an item"
)]
fn a_local_declaration_is_not_an_item() {
    let facts = collected(&[(
        "src/locals.js",
        r#"(function () { function iife() {} })();
items.forEach(function (x) { const inner = x; });
{ const inBlock = 1; }
function outer() {
  function nested() {}
  class Local { m() {} }
}
register(class { render() {} });
if (ready) { var hoisted = 1; }
export const api = { get() {}, value: 1 };
"#,
    )]);
    assert_eq!(
        items(&facts),
        ["js:app/src/locals.js::api", "js:app/src/locals.js::outer"]
    );
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "a declare module block mints no item"
)]
fn a_declare_module_block_mints_no_item() {
    let facts = collected(&[(
        "src/amb.d.ts",
        r#"declare module "pkg" {
  export function m(): void;
  export interface Opts { x: number }
}
declare global {
  interface Window { x: number }
}
export declare function real(): void;
"#,
    )]);
    assert_eq!(items(&facts), ["js:app/src/amb.d.ts::real"]);
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "an unnamed default export is the item default"
)]
fn an_unnamed_default_export_is_the_item_default() {
    let facts = collected(&[
        ("src/f.js", "export default function () {}\n"),
        ("src/c.js", "export default class { m() {} }\n"),
        ("src/o.js", "export default { a: 1 };\n"),
        ("src/a.js", "export default () => 1;\n"),
        ("src/named.js", "export default function main() {}\n"),
    ]);
    assert_eq!(
        items(&facts),
        [
            "js:app/src/a.js::default",
            "js:app/src/c.js::default",
            "js:app/src/c.js::default.m",
            "js:app/src/f.js::default",
            "js:app/src/named.js::main",
            "js:app/src/o.js::default",
        ]
    );
    let kind = |id: &str| item(&facts, id).fields["kind"].clone();
    assert_eq!(kind("js:app/src/f.js::default"), json!(["function"]));
    assert_eq!(kind("js:app/src/a.js::default"), json!(["function"]));
    assert_eq!(kind("js:app/src/c.js::default"), json!(["class"]));
    assert_eq!(kind("js:app/src/o.js::default"), json!(["const"]));
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "function-valued variables are functions"
)]
fn function_valued_variables_are_functions() {
    let facts = collected(&[(
        "src/vars.js",
        r#"const arrow = () => 1;
let fexpr = function () {};
var gen = function* () {};
exports.cjs = function (a) {};
module.exports.alsoCjs = (b) => b;
exports.value = 42;
const Widget = class { render() {} };
const plain = 1;
let counter = 0;
"#,
    )]);
    let kinds: Vec<(String, Value)> = facts
        .iter()
        .filter(|f| f.kind == "item")
        .map(|f| (f.id.clone(), f.fields["kind"].clone()))
        .collect();
    let expected: Vec<(String, Value)> = [
        ("Widget", "class"),
        ("Widget.render", "method"),
        ("alsoCjs", "function"),
        ("arrow", "function"),
        ("cjs", "function"),
        ("counter", "variable"),
        ("fexpr", "function"),
        ("gen", "function"),
        ("plain", "const"),
        ("value", "const"),
    ]
    .iter()
    .map(|(path, kind)| (format!("js:app/src/vars.js::{path}"), json!([kind])))
    .collect();
    assert_eq!(kinds, expected);
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "definitions of one name are one item with every place"
)]
fn definitions_of_one_name_are_one_item_with_every_place() {
    let facts = collected(&[(
        "src/one.ts",
        r#"export function over(a: string): void;
export function over(a: number): void;
export function over(a: any) {}
export class Box {
  get size() { return 1; }
  set size(v) {}
  static add() {}
  add() {}
}
export type Id = string;
export const Id = "id";
"#,
    )]);
    let fact = |path: &str| item(&facts, &format!("js:app/src/one.ts::{path}"));
    let lines = |path: &str| -> Vec<usize> { places(fact(path)).iter().map(|p| p.1).collect() };
    assert_eq!(lines("over"), [1, 2, 3]);
    assert_eq!(fact("over").fields["kind"], json!(["function"]));
    assert_eq!(lines("Box.size"), [5, 6]);
    assert_eq!(lines("Box.add"), [7, 8]);
    assert_eq!(fact("Box.add").fields["kind"], json!(["method"]));
    assert_eq!(lines("Id"), [10, 11]);
    assert_eq!(fact("Id").fields["kind"], json!(["type", "const"]));
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "each named package is a package fact its items point at"
)]
fn each_named_package_is_a_package_fact_its_items_point_at() {
    let facts = run(
        host(&[
            (
                "package.json",
                r#"{"name":"app","version":"1.0.0","dependencies":{"@acme/core":"*"}}"#,
            ),
            (
                "packages/core/package.json",
                r#"{"name":"@acme/core","engines":{"node":">=20"},"dependencies":{"lodash":"^4","app":"*"}}"#,
            ),
            ("packages/cli/package.json", r#"{"private":true}"#),
            ("packages/core/src/a.js", "export function a() {}\n"),
            ("packages/cli/bin/cli.mjs", "export function main() {}\n"),
        ]),
        &[],
    )
    .0
    .unwrap();
    let packages: Vec<&Fact> = facts.iter().filter(|f| f.kind == "package").collect();
    let ids: Vec<&str> = packages.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["pkg:npm/%40acme/core", "pkg:npm/app"]);
    let core = item(&facts, "pkg:npm/%40acme/core");
    assert_eq!(core.title.as_deref(), Some("@acme/core"));
    assert_eq!(core.fields["name"], "@acme/core");
    assert_eq!(core.fields["path"], "packages/core");
    assert_eq!(core.fields["engines"], json!(["node >=20"]));
    assert_eq!(
        core.fields["depends_on"],
        json!(["pkg:npm/app"]),
        "never lodash"
    );
    let app = item(&facts, "pkg:npm/app");
    assert_eq!(app.fields["path"], "");
    assert_eq!(app.fields["version"], "1.0.0");
    assert_eq!(app.fields["depends_on"], json!(["pkg:npm/%40acme/core"]));
    for fact in facts.iter().filter(|f| f.kind == "item") {
        assert!(
            ids.contains(&fact.fields["package"].as_str().unwrap()),
            "{} points at {}",
            fact.id,
            fact.fields["package"]
        );
    }
    assert_eq!(
        items(&facts),
        [
            "js:@acme/core/src/a.js::a",
            "js:app/packages/cli/bin/cli.mjs::main"
        ]
    );
}
