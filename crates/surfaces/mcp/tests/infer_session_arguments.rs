use crate::support::*;
use serde_json::{Value, json};
use specforge_test::prelude::*;

/// `specforge.infer_session` over a project with an inference manifest and a source file.
fn served() -> Served {
    let tmp = TestProject::new();
    std::fs::write(
        tmp.root().join("specforge-infer.json"),
        r#"{"version":1,"source_roots":["src"]}"#,
    )
    .unwrap();
    std::fs::create_dir_all(tmp.root().join("src")).unwrap();
    std::fs::write(tmp.root().join("src/lib.rs"), "pub fn hello() {}").unwrap();
    tmp.serve(&[])
}

fn manifest(server: &Served) -> String {
    std::fs::read_to_string(server.root().join("specforge-infer.json")).unwrap()
}

#[specforge_test(
    behavior = "provide_mcp_infer_session_tool",
    verify = "an argument of another action is refused on that argument"
)]
fn an_argument_of_another_action_is_refused_on_it() {
    let mut server = served();
    let started = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({"action": "start", "agent": "a"}),
    );
    let id = serde_json::from_str::<Value>(&tool_text(&started)).unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let before = manifest(&server);

    for (args, argument) in [
        (
            json!({"action": "start", "agent": "a", "source_file": "src/lib.rs"}),
            "source_file",
        ),
        (
            json!({"action": "mark_analyzed", "source_file": "src/lib.rs", "agent": "b"}),
            "agent",
        ),
        (
            json!({"action": "end", "session_id": id, "status": "paused", "entities_produced": ["x"]}),
            "entities_produced",
        ),
        (
            json!({"action": "start", "agent": "a", "status": "paused"}),
            "status",
        ),
    ] {
        let resp = call_tool(&mut server, "specforge.infer_session", args.clone());
        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["code"], "invalid_input", "{args}: {error}");
        assert_eq!(error["argument"], argument, "{args}: {error}");
        assert_eq!(error["data"]["files_written"], json!([]), "{error}");
        assert_eq!(manifest(&server), before, "{args}");
    }

    let ended = call_tool(
        &mut server,
        "specforge.infer_session",
        json!({"action": "end", "session_id": id}),
    );
    let ended: Value = serde_json::from_str(&tool_text(&ended)).unwrap();
    assert_eq!(ended["status"], "completed", "{ended}");
}
