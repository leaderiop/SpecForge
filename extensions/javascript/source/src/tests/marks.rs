//! Exports, visibility, docs, signatures and modifiers: what an item says beyond its name and place.

use serde_json::{json, Value};
use specforge_test::prelude::*;

use super::{collected, item, Fact};

/// The visibility of each item of `facts`, by id.
fn visibilities(facts: &[Fact]) -> Vec<(&str, &str)> {
    facts
        .iter()
        .filter(|f| f.kind == "item")
        .map(|f| (f.id.as_str(), f.fields["visibility"].as_str().unwrap()))
        .collect()
}

/// The visibility of each item of `file` (under the package `app`), by path.
fn visibility_of(facts: &[Fact], file: &str) -> Vec<(String, String)> {
    let prefix = format!("js:app/{file}::");
    visibilities(facts)
        .into_iter()
        .filter_map(|(id, v)| Some((id.strip_prefix(&prefix)?.to_string(), v.to_string())))
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "an exported declaration is public and a local one private"
)]
fn an_exported_declaration_is_public_and_a_local_one_private() {
    let facts = collected(&[(
        "src/a.ts",
        r#"export function a() {}
function b() {}
export const c = 1;
exports.d = 1;
export namespace N {
  export function f() {}
  function g() {}
}
namespace M {
  export function h() {}
}
"#,
    )]);
    assert_eq!(
        visibility_of(&facts, "src/a.ts"),
        pairs(&[
            ("M", "private"),
            ("M.h", "private"),
            ("N", "public"),
            ("N.f", "public"),
            ("N.g", "private"),
            ("a", "public"),
            ("b", "private"),
            ("c", "public"),
            ("d", "public"),
        ])
    );
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "an export list and module.exports make an item public"
)]
fn an_export_list_and_module_exports_make_an_item_public() {
    let facts = collected(&[
        (
            "src/esm.js",
            "function a() {}\nfunction b() {}\nfunction c() {}\nexport { a, b as renamed };\nexport { c } from './other.js';\n",
        ),
        (
            "src/cjs.js",
            "function sum() {}\nfunction helper() {}\nfunction hidden() {}\nmodule.exports = { sum, alias: helper };\n",
        ),
    ]);
    assert_eq!(
        visibility_of(&facts, "src/esm.js"),
        pairs(&[("a", "public"), ("b", "public"), ("c", "private")])
    );
    assert_eq!(
        visibility_of(&facts, "src/cjs.js"),
        pairs(&[
            ("helper", "public"),
            ("hidden", "private"),
            ("sum", "public")
        ])
    );
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "export default of a name marks it default"
)]
fn export_default_of_a_name_marks_it_default() {
    let facts = collected(&[
        ("src/main.js", "function main() {}\nexport default main;\n"),
        ("src/named.js", "export default function named() {}\n"),
    ]);
    let main = item(&facts, "js:app/src/main.js::main");
    assert_eq!(main.fields["visibility"], "public");
    assert_eq!(main.fields["attrs"], json!(["default"]));
    let named = item(&facts, "js:app/src/named.js::named");
    assert_eq!(named.fields["visibility"], "public");
    assert_eq!(named.fields["attrs"], json!(["default", "export"]));
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "an export list naming a default marks it default"
)]
fn an_export_list_naming_a_default_marks_it_default() {
    let facts = collected(&[(
        "src/a.js",
        "function a() {}\nfunction b() {}\nexport { a as default, b };\n",
    )]);
    let a = item(&facts, "js:app/src/a.js::a");
    assert_eq!(a.fields["visibility"], "public");
    assert_eq!(a.fields["attrs"], json!(["default"]));
    let b = item(&facts, "js:app/src/a.js::b");
    assert_eq!(b.fields["visibility"], "public");
    assert_eq!(b.fields.get("attrs"), None, "b is no default export");
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "a private member of a public class is private"
)]
fn a_private_member_of_a_public_class_is_private() {
    let facts = collected(&[(
        "src/a.ts",
        r#"export class A {
  #x = 1;
  private y = 2;
  protected z = 3;
  w = 4;
  #m() {}
  private n() {}
  o() {}
}
"#,
    )]);
    assert_eq!(
        visibility_of(&facts, "src/a.ts"),
        pairs(&[
            ("A", "public"),
            ("A.%23m", "private"),
            ("A.%23x", "private"),
            ("A.n", "private"),
            ("A.o", "public"),
            ("A.w", "public"),
            ("A.y", "private"),
            ("A.z", "public"),
        ])
    );
    assert_eq!(
        item(&facts, "js:app/src/a.ts::A.z").fields["attrs"],
        json!(["protected"])
    );
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "a member of a private class is private"
)]
fn a_member_of_a_private_class_is_private() {
    let facts = collected(&[(
        "src/a.ts",
        "class B { m() {} }\nexport interface I { p: string }\ninterface J { q(): void }\n",
    )]);
    assert_eq!(
        visibility_of(&facts, "src/a.ts"),
        pairs(&[
            ("B", "private"),
            ("B.m", "private"),
            ("I", "public"),
            ("I.p", "public"),
            ("J", "private"),
            ("J.q", "private"),
        ])
    );
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "a JSDoc comment is the doc of the item below it"
)]
fn a_jsdoc_comment_is_the_doc_of_the_item_below_it() {
    let facts = collected(&[(
        "src/doc.js",
        r#"/**
 * Adds two numbers.
 * Returns the sum.
 */
export function add(a, b) {}

/** One line. */ export const one = 1;
/* not a doc */
function plain() {}
/** Far away. */

function far() {}
class K {
  /** A method. */
  m() {}
}
"#,
    )]);
    let doc = |path: &str| -> Value {
        item(&facts, &format!("js:app/src/doc.js::{path}"))
            .fields
            .get("doc")
            .cloned()
            .unwrap_or(Value::Null)
    };
    assert_eq!(doc("add"), "Adds two numbers.\nReturns the sum.");
    assert_eq!(doc("one"), "One line.");
    assert_eq!(doc("plain"), Value::Null);
    assert_eq!(doc("far"), Value::Null);
    assert_eq!(doc("K.m"), "A method.");
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "a decorated class takes the doc above its decorator, exported or not"
)]
fn a_decorated_class_takes_the_doc_above_its_decorator_exported_or_not() {
    let facts = collected(&[(
        "src/a.ts",
        r#"/** Plain. */
@dec
class Plain {}

/** Exported. */
@dec
@other()
export class Exported {}

/** Default. */
@dec
export default class {}
"#,
    )]);
    let doc = |path: &str| item(&facts, &format!("js:app/src/a.ts::{path}")).fields["doc"].clone();
    assert_eq!(doc("Plain"), "Plain.");
    assert_eq!(doc("Exported"), "Exported.");
    assert_eq!(doc("default"), "Default.");
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "a signature holds types and never a value"
)]
fn a_signature_holds_types_and_never_a_value() {
    let signatures = |text: &str| -> Vec<(String, Value)> {
        collected(&[("src/s.ts", text)])
            .iter()
            .filter(|f| f.kind == "item")
            .map(|f| {
                (
                    f.id.clone(),
                    f.fields.get("signature").cloned().unwrap_or(Value::Null),
                )
            })
            .collect()
    };
    let before = signatures(
        "export function f(a: number): string { return \"x\"; }\nexport const c: number = 1;\nexport const d = 1;\n",
    );
    let values_changed = signatures(
        "export function f(a: number):   string { return \"y\" + a; }\nexport const c: number = 2;\nexport const d = 2;\n",
    );
    assert_eq!(
        before, values_changed,
        "a body, a value and whitespace are no signature"
    );
    // By id: c, d, f.
    assert_eq!(before[1].1, Value::Null, "an untyped const has none");
    assert!(before[2].1.as_str().unwrap().starts_with("sha256:"));
    let typed_changed = signatures(
        "export function f(a: string): string { return a; }\nexport const c: string = \"1\";\nexport const d = 1;\n",
    );
    assert_ne!(before[0].1, typed_changed[0].1);
    assert_ne!(before[2].1, typed_changed[2].1);
}

#[specforge_test(behavior = "js_mark_item_exports", verify = "modifiers are attrs")]
fn modifiers_are_attrs() {
    let facts = collected(&[(
        "src/m.ts",
        r#"export abstract class A {
  static s = 1;
  readonly r = 2;
  declare d: number;
  abstract a(): void;
  protected p() {}
  public q() {}
  private v() {}
  override o() {}
  async load() {}
  get g() { return 1; }
  set g(v) {}
  *gen() {}
}
export declare function df(): void;
export const enum E { X }
let l = 1;
var v = 2;
export async function af() {}
export function* gf() {}
"#,
    )]);
    let attrs =
        |path: &str| item(&facts, &format!("js:app/src/m.ts::{path}")).fields["attrs"].clone();
    for (path, expected) in [
        ("A", json!(["abstract", "export"])),
        ("A.s", json!(["static"])),
        ("A.r", json!(["readonly"])),
        ("A.d", json!(["declare"])),
        ("A.a", json!(["abstract"])),
        ("A.p", json!(["protected"])),
        ("A.q", json!(["public"])),
        ("A.v", json!(["private"])),
        ("A.o", json!(["override"])),
        ("A.load", json!(["async"])),
        ("A.g", json!(["get", "set"])),
        ("A.gen", json!(["*"])),
        ("df", json!(["declare", "export"])),
        ("E", json!(["const", "export"])),
        ("l", json!(["let"])),
        ("v", json!(["var"])),
        ("af", json!(["async", "export"])),
        ("gf", json!(["*", "export"])),
    ] {
        assert_eq!(attrs(path), expected, "{path}");
    }
}
