//! Every tool a prompt or an installed agent names is one the engine has (ADR 0334): an engine tool of the catalogue,
//! an extension's declared tool or command, a port or host verb, the manifest or a declared setting key. MCP names are
//! read anywhere in the text; a CLI line only as a command in code, the way `tool_names::named` reads it.

use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use super::harness::project;
use super::prompts_support::{HOSTS, payload, prompt, text_of};
use super::render_prompts::ROWS;
use super::rendered_lines::rendered;
use super::tool_names::{
    PORT_AND_HOST_VERBS, claude_name_is_known, code_commands, is_cli_verb, mcp_name_is_known,
};

#[test]
fn every_port_and_host_verb_is_a_command() {
    let p = project("empty");
    for verb in PORT_AND_HOST_VERBS {
        let run = p.run(&[verb, "--help"]);
        assert_eq!(run.code, 0, "specforge {verb} --help: {}", run.all());
    }
}

/// `(host:file, text)` of the 44 prompts rendered for each host and of every agent file each host's install writes.
fn corpus() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = rendered()
        .iter()
        .map(|(id, text)| (format!("claude-code:prompt:{id}"), text.clone()))
        .collect();
    for host in HOSTS.into_iter().filter(|h| *h != "claude-code") {
        for row in &ROWS {
            let p = (row.setup)();
            let text = text_of(&payload(&prompt(&p, row.id, row.args, host)));
            out.push((format!("{host}:prompt:{}", row.id), text));
        }
    }
    for host in HOSTS {
        let p = project("empty");
        super::agents::installed(&p, host);
        let folder = match host {
            "claude-code" => ".claude/agents",
            "codex" => ".codex/agents",
            _ => ".cursor/agents",
        };
        let mut files: Vec<_> = std::fs::read_dir(p.root().join(folder))
            .unwrap_or_else(|e| panic!("{host}'s agents: {e}"))
            .map(|entry| entry.expect("an entry").path())
            .collect();
        files.sort();
        assert!(files.len() >= 15, "{host}: {} agent files", files.len());
        for file in files {
            let name = file.file_name().map(Path::new).expect("a file name");
            let text = std::fs::read_to_string(&file).expect("an agent file");
            out.push((format!("{host}:{}", name.display()), text));
        }
    }
    out
}

/// The `specforge.<x>` and `mcp__specforge__<n>` words of `text`: not inside a longer word or path.
fn mcp_words(text: &str) -> Vec<String> {
    static RX: OnceLock<Regex> = OnceLock::new();
    let rx = RX.get_or_init(|| {
        Regex::new(r"(?:^|[^A-Za-z0-9_./~-])(specforge\.[A-Za-z0-9_]+(?:[.-][A-Za-z0-9_]+)*|mcp__specforge__[A-Za-z0-9_*-]*)")
            .expect("a regex")
    });
    rx.captures_iter(text).map(|c| c[1].to_string()).collect()
}

#[test]
fn every_tool_a_prompt_or_agent_names_is_an_engine_tool() {
    // the dictionary refuses what it does not hold
    assert!(mcp_name_is_known("specforge.trace") && mcp_name_is_known("specforge.json"));
    assert!(mcp_name_is_known("specforge.task_trailers"));
    assert!(mcp_name_is_known("specforge.gsd.start_plan"));
    assert!(!mcp_name_is_known("specforge.coverage_report"));
    assert!(claude_name_is_known(
        "mcp__specforge__specforge_gsd_start_plan"
    ));
    assert!(!claude_name_is_known("mcp__specforge__*"));
    assert!(is_cli_verb(&["query", "--inputs", "gsd/E501"]) && is_cli_verb(&["gsd", "start-plan"]));
    assert!(!is_cli_verb(&["gsd", "start-plans"]) && !is_cli_verb(&["coverage"]));
    assert_eq!(
        mcp_words("see `specforge.edit`. ~/.specforge.x"),
        ["specforge.edit"]
    );

    let (mut checked, mut unknown) = (0, Vec::new());
    for (source, text) in corpus() {
        for name in mcp_words(&text) {
            checked += 1;
            let known = if name.starts_with("mcp__") {
                claude_name_is_known(&name)
            } else {
                mcp_name_is_known(&name)
            };
            if !known {
                unknown.push(format!("{source}: {name}"));
            }
        }
        for words in code_commands(&text) {
            let Some(("specforge", rest)) = words.split_first().map(|(f, r)| (f.as_str(), r))
            else {
                continue;
            };
            let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
            // `specforge` and `specforge gsd` alone name the program and the namespace, no tool
            if rest.is_empty() || rest == ["gsd"] {
                continue;
            }
            checked += 1;
            if !is_cli_verb(&rest) {
                unknown.push(format!("{source}: specforge {}", rest.join(" ")));
            }
        }
    }
    unknown.sort();
    unknown.dedup();
    assert!(checked > 1000, "only {checked} names read");
    assert!(
        unknown.is_empty(),
        "names no engine, extension, port or host tool answers:\n{}",
        unknown.join("\n")
    );
}
