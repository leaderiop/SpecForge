//! Extension prompts over MCP (ADR 0291): what `prompts/list` and `prompts/get` serve of a project's extensions.
//! `prompts.rs` is over the file-size cap and may not grow, so this is where the extension half lives.

use crate::prompt_golden::{get, project, send, server};
use serde_json::{Value, json};
use specforge_mcp::testing::{Served, rpc};
use specforge_project::SharedRuntime;
use specforge_project::testing::{
    CORE as EXT, GsdPrompts, NO_CONTEXT, TestProject, gsd_prompts_sources,
};
use std::sync::Arc;

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

/// `@t/core` served in process over a project whose phase has `goal`, with `settings` for the extension.
fn gsd(goal: &str, state: &str, settings: Value) -> Served {
    gsd_for(goal, state, settings, json!({}))
}

/// [`gsd`], the client's `initialize` also sending `init`.
fn gsd_for(goal: &str, state: &str, settings: Value, init: Value) -> Served {
    gsd_using(GsdPrompts::new(), goal, state, settings, init)
}

fn gsd_using(fixture: GsdPrompts, goal: &str, state: &str, settings: Value, init: Value) -> Served {
    let project = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["settings"] = json!({ EXT: settings });
        })
        .enabling(&[EXT]);
    for (path, text) in gsd_prompts_sources(goal, state) {
        project.write(path, text);
    }
    let runtime: SharedRuntime = Arc::new(fixture.runtime());
    Served::initialized_in_with(project, runtime, init)
}

fn listed(served: &mut Served) -> Vec<String> {
    let reply = rpc::call(served, "prompts/list", json!({}));
    reply["result"]["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "an extension prompt is listed after the core ones and served by prompts/get"
)]
fn an_extension_prompt_is_listed_and_served() {
    let mut served = gsd("fine", "ok", json!({}));
    let names = listed(&mut served);
    assert_eq!(
        names[CORE_PROMPTS..],
        ["specforge.t.plan_phase", "specforge.t.research_phase"],
        "{names:?}"
    );
    let reply = rpc::get_prompt(
        &mut served,
        "specforge.t.plan_phase",
        json!({"phase": "03", "research": "true"}),
    );
    let messages = reply["result"]["messages"]
        .as_array()
        .unwrap_or_else(|| panic!("{reply}"));
    assert!(
        messages[0]["content"]["text"]
            .as_str()
            .unwrap()
            .starts_with("prompt__plan_phase ")
    );
    let payload: Value = serde_json::from_str(
        messages.last().unwrap()["content"]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(payload["prompt"], "@t/core@0.4.0:plan_phase");
    assert_eq!(payload["subject"], "v1.0/03");
}

/// The core prompts `prompts/list` gives before an extension's.
const CORE_PROMPTS: usize = 5;

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt whose when setting is off is not listed or served"
)]
fn a_prompt_disabled_by_its_setting_is_not_served() {
    let mut served = gsd("fine", "ok", json!({"workflow.research": false}));
    let names = listed(&mut served);
    assert!(
        names.contains(&"specforge.t.plan_phase".to_string()),
        "{names:?}"
    );
    assert!(
        !names.contains(&"specforge.t.research_phase".to_string()),
        "{names:?}"
    );
    let reply = rpc::get_prompt(
        &mut served,
        "specforge.t.research_phase",
        json!({"phase": "03"}),
    );
    assert!(reply.get("error").is_some(), "{reply}");
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "an agent renders any prompt through specforge.prompt, by name and arguments or by argv"
)]
fn an_agent_renders_a_prompt_through_specforge_prompt() {
    let mut served = gsd("fine", "ok", json!({}));
    let by_name = rpc::tool(
        &mut served,
        "specforge.prompt",
        json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03", "depth": 2}}),
    );
    let by_argv = rpc::tool(
        &mut served,
        "specforge.prompt",
        json!({"name": "specforge.t.plan_phase", "argv": "03 --depth 2"}),
    );
    assert_eq!(by_name, by_argv, "one render behind both");
    assert_eq!(by_name["messages"][0]["role"], "user");
    assert_eq!(by_name["payload"]["subject"], "v1.0/03");

    let both = rpc::call_tool(
        &mut served,
        "specforge.prompt",
        json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03"}, "argv": "03"}),
    );
    assert_eq!(
        both["result"]["isError"], true,
        "at most one of arguments and argv: {both}"
    );
    let core = rpc::tool(&mut served, "specforge.prompt", json!({"name": "review"}));
    assert!(
        core["messages"].is_array(),
        "a core prompt is reachable too: {core}"
    );

    let mut empty = gsd(NO_CONTEXT, "ok", json!({}));
    let refused = rpc::call_tool(
        &mut empty,
        "specforge.prompt",
        json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03"}}),
    );
    assert!(
        rpc::tool_text(&refused).contains("gate_failed"),
        "{refused}"
    );
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "the client names the host a prompt renders for, and a host argument overrides it"
)]
fn the_client_names_the_host_a_prompt_renders_for() {
    let host_of = |served: &mut Served, extra: Value| {
        let mut arguments = json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03"}});
        arguments
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let rendered = rpc::tool(served, "specforge.prompt", arguments);
        let text = rendered["messages"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        let echoed: Value = serde_json::from_str(text.split_once(' ').unwrap().1).unwrap();
        echoed["host"].clone()
    };
    let mut served = gsd("fine", "ok", json!({}));
    assert_eq!(
        host_of(&mut served, json!({})),
        Value::Null,
        "an unnamed client has no host"
    );
    assert_eq!(host_of(&mut served, json!({"host": "cursor"})), "cursor");
    // `initialize` carries `clientInfo.name`.
    let init = json!({"clientInfo": {"name": "claude-code", "version": "2"}});
    let mut named = gsd_for("fine", "ok", json!({}), init);
    assert_eq!(host_of(&mut named, json!({})), "claude-code");
    assert_eq!(
        host_of(&mut named, json!({"host": "codex"})),
        "codex",
        "a host argument overrides the client's"
    );
}

#[specforge_test_macros::test(
    behavior = "gate_a_command",
    verify = "a gated command is refused gate_failed over MCP with its failing diagnostics, and its export is not called"
)]
fn a_gated_command_is_refused_over_mcp() {
    let mut served = gsd("fine", "blocked", json!({}));
    let reply = rpc::call_tool(
        &mut served,
        "specforge.t.execute_phase",
        json!({"phase": "03"}),
    );
    let text = rpc::tool_text(&reply);
    assert!(
        text.contains("gate_failed") && text.contains("t/E902"),
        "{text}"
    );
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "prompts/get of a prompt whose gate fails is a JSON-RPC error naming gate_failed, and its export is not called"
)]
fn prompts_get_of_a_failing_gate_is_a_rpc_error() {
    let mut served = gsd(NO_CONTEXT, "ok", json!({}));
    let reply = rpc::call(
        &mut served,
        "prompts/get",
        json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03"}}),
    );
    assert!(reply.get("result").is_none(), "{reply}");
    assert!(
        reply["error"].to_string().contains("gate_failed"),
        "{reply}"
    );
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "specforge.prompt refuses an unknown host, an unknown prompt (naming the served ones) and argv for a core prompt"
)]
fn specforge_prompt_refuses_what_it_cannot_render() {
    let mut served = gsd("fine", "ok", json!({}));
    let mut refuse = |arguments: Value| {
        let reply = rpc::call_tool(&mut served, "specforge.prompt", arguments);
        assert_eq!(reply["result"]["isError"], true, "{reply}");
        rpc::tool_text(&reply)
    };
    let host = refuse(
        json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03"}, "host": "vim"}),
    );
    assert!(host.contains("unknown host 'vim'"), "{host}");
    let unknown = refuse(json!({"name": "specforge.t.nope"}));
    assert!(
        unknown.contains("no prompt") && unknown.contains("specforge.t.plan_phase"),
        "{unknown}"
    );
    let argv = refuse(json!({"name": "review", "argv": "x"}));
    assert!(argv.contains("by name"), "{argv}");
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt's multi-line message reaches the agent with its lines, and other controls escaped"
)]
fn a_multi_line_prompt_message_reaches_the_agent_whole() {
    use specforge_wasm::WasmCallResult;
    let answer =
        json!({"messages": [{"role": "user", "text": "# Plan\n\n- one\n\t- two\u{1b}[0m"}]});
    let fixture = GsdPrompts::new().answering(
        EXT,
        "prompt__plan_phase",
        WasmCallResult::Ok(answer.to_string().into_bytes()),
    );
    let mut served = gsd_using(fixture, "fine", "ok", json!({}), json!({}));
    let got = rpc::call(
        &mut served,
        "prompts/get",
        json!({"name": "specforge.t.plan_phase", "arguments": {"phase": "03"}}),
    );
    assert_eq!(
        got["result"]["messages"][0]["content"]["text"], "# Plan\n\n- one\n\t- two\\u{1b}[0m",
        "{got}"
    );
}
