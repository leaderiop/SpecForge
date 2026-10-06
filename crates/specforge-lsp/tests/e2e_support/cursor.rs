//! What the entity under a cursor is, and what completes there, over a
//! project with `@specforge/software` (plan 06's fixtures).

use super::*;

/// A ref, the behavior its ID's middle word spells, and a behavior
/// listing the ref (with a `[` inside a string before it).
const MAIN: &str = concat!(
    "ref gh.issue:42 \"Support Wasm\"\n",
    "\n",
    "behavior issue \"Issue tracking\" {\n",
    "  contract \"tracks issues\"\n",
    "}\n",
    "\n",
    "behavior login \"Login\" {\n",
    "  contract \"see [docs\"\n",
    "  refs [gh.issue:42]\n",
    "}\n",
);

/// Entity IDs written in a comment and in a string.
const PROSE: &str = concat!(
    "behavior issue \"Issue tracking\" {\n",
    "  contract \"tracks issues\"\n",
    "}\n",
    "\n",
    "behavior login \"Login\" {\n",
    "  contract \"x\"\n",
    "}\n",
    "\n",
    "behavior delta \"Delta\" {\n",
    "  // see issue for the details\n",
    "  description \"reuses login and the contract field\"\n",
    "}\n",
);

/// Two bodies that differ only by a `[` inside a string.
const COMP: &str = concat!(
    "behavior alpha \"Alpha\" {\n",
    "  contract \"see [docs\"\n",
    "  \n",
    "}\n",
    "\n",
    "behavior beta \"Beta\" {\n",
    "  contract \"see docs\"\n",
    "  \n",
    "}\n",
    "\n",
);

/// A nested block, then a string being typed.
const NEST: &str = concat!(
    "behavior eps \"Eps\" {\n",
    "  requires {\n",
    "    \n",
    "  }\n",
    "  description \"the \"\n",
    "}\n",
);

/// The (line, UTF-16 character) of the `nth` occurrence of `needle` in
/// `text`, plus `offset` characters.
fn pos(text: &str, needle: &str, nth: usize, offset: u32) -> (u32, u32) {
    let at = text
        .match_indices(needle)
        .nth(nth)
        .unwrap_or_else(|| panic!("no {needle:?} #{nth}"))
        .0;
    let before = &text[..at];
    let line = before.matches('\n').count() as u32;
    let column = before.rsplit('\n').next().unwrap_or("");
    (line, column.encode_utf16().count() as u32 + offset)
}

/// A server over `text` as `file`, in a project with software.
async fn open(file: &str, text: &str) -> (LspClient, String, tempfile::TempDir) {
    start_server_with_extensions(&["@specforge/software"], file, text).await
}

/// The hover's markdown at a position, or "" when there is none.
async fn hover_text(client: &mut LspClient, uri: &str, (line, character): (u32, u32)) -> String {
    let resp = client.hover(uri, line, character).await;
    resp["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The completion items at a position.
async fn items(client: &mut LspClient, uri: &str, line: u32, character: u32) -> Vec<Value> {
    let resp = client.completion(uri, line, character).await;
    resp["result"].as_array().cloned().unwrap_or_default()
}

#[spec(
    behavior = "prepare_rename",
    verify = "prepare rename on entity ID returns token range"
)]
#[tokio::test]
async fn prepare_rename_answers_a_ref_id() {
    let (mut client, uri, _dir) = open("main.spec", MAIN).await;
    let resp = client.rename_range_at(&uri, 0, 7).await;
    let range = &resp["result"];
    assert_eq!(
        (
            range["start"]["line"].as_u64(),
            range["start"]["character"].as_u64(),
            range["end"]["line"].as_u64(),
            range["end"]["character"].as_u64()
        ),
        (Some(0), Some(4), Some(0), Some(15)),
        "{resp}"
    );
}

// Flipped by 06-T3 (R1: hover still names another entity here).
#[tokio::test]
async fn pin_definition_on_a_ref_item_selects_its_block() {
    let (mut client, uri, _dir) = open("main.spec", MAIN).await;
    let resp = client.goto_definition(&uri, 8, 12).await;
    let start = &resp["result"]["range"]["start"];
    assert_eq!(
        (start["line"].as_u64(), start["character"].as_u64()),
        (Some(0), Some(4)),
        "{resp}"
    );
}

// Flipped by 06-T3.
#[tokio::test]
async fn pin_hover_on_a_ref_item_names_another_entity() {
    let (mut client, uri, _dir) = open("main.spec", MAIN).await;
    let value = hover_text(&mut client, &uri, (8, 12)).await;
    assert!(value.contains("`issue`"), "{value}");
    assert!(!value.contains("`gh.issue:42`"), "{value}");
}

// Flipped by 06-T3. Since 06-T1 the definition from a ref's own ID stays
// on it (its name is one token); hover still reads the middle word.
#[tokio::test]
async fn pin_hover_on_a_ref_declaration_names_another_entity() {
    let (mut client, uri, _dir) = open("main.spec", MAIN).await;
    let resp = client.goto_definition(&uri, 0, 7).await;
    let start = &resp["result"]["range"]["start"];
    assert_eq!(
        (start["line"].as_u64(), start["character"].as_u64()),
        (Some(0), Some(4)),
        "{resp}"
    );
    let value = hover_text(&mut client, &uri, (0, 7)).await;
    assert!(value.contains("`issue`"), "{value}");
}

// Flipped by 06-T3.
#[tokio::test]
async fn pin_words_in_prose_name_entities() {
    let (mut client, uri, _dir) = open("prose.spec", PROSE).await;
    let in_comment = pos(PROSE, "issue for", 0, 1);
    let in_string = pos(PROSE, "login and", 0, 1);
    let value = hover_text(&mut client, &uri, in_comment).await;
    assert!(value.contains("`issue`"), "{value}");
    let resp = client
        .goto_definition(&uri, in_comment.0, in_comment.1)
        .await;
    assert!(!resp["result"].is_null(), "{resp}");
    let value = hover_text(&mut client, &uri, in_string).await;
    assert!(value.contains("`login`"), "{value}");
}

// Flipped by 06-T3.
#[tokio::test]
async fn pin_field_help_for_a_word_in_a_string() {
    let (mut client, uri, _dir) = open("prose.spec", PROSE).await;
    let value = hover_text(&mut client, &uri, pos(PROSE, "contract field", 0, 1)).await;
    assert!(value.contains("**`contract`**"), "{value}");
}

// Flipped by 06-T4.
#[tokio::test]
async fn pin_a_bracket_in_a_string_opens_a_list() {
    let (mut client, uri, _dir) = open("comp.spec", COMP).await;
    let found = items(&mut client, &uri, 2, 2).await;
    assert!(!found.is_empty());
    assert!(found.iter().all(|i| i["kind"] == 18), "{found:?}");
}

// Flipped by 06-T4.
#[tokio::test]
async fn pin_define_is_offered_at_top_level() {
    let (mut client, uri, _dir) = open("comp.spec", COMP).await;
    let found = items(&mut client, &uri, 10, 0).await;
    assert!(found.iter().any(|i| i["label"] == "define"), "{found:?}");
}

// Flipped by 06-T4.
#[tokio::test]
async fn pin_a_nested_block_offers_entity_ids() {
    let (mut client, uri, _dir) = open("nest.spec", NEST).await;
    let found = items(&mut client, &uri, 2, 4).await;
    assert!(!found.is_empty());
    assert!(found.iter().all(|i| i["kind"] == 18), "{found:?}");
}

// Flipped by 06-T4.
#[tokio::test]
async fn pin_a_string_offers_field_names() {
    let (mut client, uri, _dir) = open("nest.spec", NEST).await;
    let found = items(&mut client, &uri, 4, 19).await;
    assert!(!found.is_empty());
    assert!(found.iter().all(|i| i["kind"] == 5), "{found:?}");
}
