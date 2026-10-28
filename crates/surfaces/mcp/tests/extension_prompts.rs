//! Extension prompts over MCP (ADR 0291): what `prompts/list` and `prompts/get` serve of a project's extensions.
//! `prompts.rs` is over the file-size cap and may not grow, so this is where the extension half lives.

use crate::prompt_golden::{get, project, send, server};
use serde_json::json;

/// The core prompts, by name, in the order `prompts/list` gives them.
const CORE: [&str; 5] = ["context", "review", "trace", "explore", "infer"];

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a project whose extensions declare no prompt lists and serves the core prompts unchanged"
)]
fn a_project_without_extension_prompts_lists_the_core_prompts_unchanged() {
    let dir = project();
    let mut server = server(&dir);
    let listed = send(&mut server, "prompts/list", json!({}));
    let names: Vec<&str> = listed["result"]["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    let core: Vec<String> = CORE
        .iter()
        .map(|name| format!("specforge://prompts/{name}"))
        .collect();
    assert_eq!(names, core, "the core five and nothing after them");
    // The listing is the one `prompt_golden` pins, byte for byte.
    insta::assert_json_snapshot!(
        "core_prompts_listing_without_extension_prompts",
        listed["result"]
    );
    // Each core prompt still renders its instruction then its payload.
    for name in ["review", "explore"] {
        let reply = get(&mut server, name, json!({}));
        let messages = reply["result"]["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2, "{name}: {reply}");
        assert!(messages.iter().all(|m| m["role"] == "user"), "{name}");
    }
}
