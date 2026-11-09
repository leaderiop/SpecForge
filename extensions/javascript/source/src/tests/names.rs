//! An item's name as the grammar's nodes give it: never split or unquoted by hand.

use specforge_code_facts::Definition;

use super::*;

/// The item ids of `file`'s items under the package `app`, by path.
fn paths(facts: &[Fact], file: &str) -> Vec<String> {
    let prefix = format!("js:app/{file}::");
    items(facts)
        .into_iter()
        .filter_map(|id| Some(id.strip_prefix(&prefix)?.to_string()))
        .collect()
}

/// The definitions `items::read` gives `text`, read as the TypeScript file `src/a.ts` of the root package `app`.
fn definitions(text: &str) -> Vec<Definition> {
    let workspace = Workspace::of(&input(&host(&[APP]), &[]));
    let query = SourceQuery::compile(Language::TypeScript, QUERY).unwrap();
    let SyntaxOutcome::Parsed { matches, .. } = query.parse(text.as_bytes(), COUNTED_DEADLINE)
    else {
        panic!("src/a.ts parses");
    };
    crate::items::read(&workspace.packages()[0], "src/a.ts", "src/a.ts", &matches)
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "a dotted namespace name is one segment per identifier, whatever is written between them"
)]
fn a_dotted_namespace_name_is_one_segment_per_identifier() {
    let facts = collected(&[(
        "src/a.ts",
        "namespace A . /* the second */ B.C {\n  export const x = 1;\n}\n\
         namespace D.E.F.G.H.I.J {\n  export const y = 1;\n}\n",
    )]);
    // Past six segments a name has no capture: no item (the query's documented limit).
    assert_eq!(paths(&facts, "src/a.ts"), ["A.B.C", "A.B.C.x"]);
}

#[specforge_test(
    behavior = "js_mint_item_facts",
    verify = "a string member name is its value, escapes decoded"
)]
fn a_string_member_name_is_its_value_escapes_decoded() {
    let facts = collected(&[(
        "src/a.ts",
        "export class Box {\n  'a\\'b'() {}\n  \"c\\u0041\\x42\"() {}\n  'x.y' = 1;\n  ''() {}\n  '#p' = 2;\n}\n\
         export interface I {\n  'line\\\n  end': string;\n}\n",
    )]);
    assert_eq!(
        paths(&facts, "src/a.ts"),
        [
            "Box",
            "Box.%23p",
            "Box.a'b",
            "Box.cAB",
            "Box.x%2Ey",
            "I",
            "I.line  end"
        ]
    );
    let quoted = item(&facts, "js:app/src/a.ts::Box.%23p");
    assert_eq!(
        quoted.fields["visibility"], "public",
        "a string name is never #private"
    );
}

#[specforge_test(
    behavior = "js_mark_item_exports",
    verify = "a signature keeps its parts' source text"
)]
fn a_signature_keeps_its_parts_source_text() {
    let read = definitions("export function f(a: 'a  b',\n  c: number): void {}\n");
    let [f] = &read[..] else {
        panic!("one definition: {read:?}");
    };
    assert_eq!(
        f.signature.as_deref(),
        Some("(a: 'a  b',\n  c: number) : void")
    );
}
