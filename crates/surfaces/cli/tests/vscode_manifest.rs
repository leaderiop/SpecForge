//! The VS Code client's manifest and sources: every setting, view, script,
//! command and flag they name is one the client provides and the CLI answers.

use crate::docs_truth::manifest_domain_kinds;
use crate::support::{Cli, TestProject};
use specforge_repo_root::repo_root;
use std::collections::BTreeSet;
/// The canonical repository URL is defined once, as the workspace's Cargo
/// `repository` (ADR 0004 D6-b); the VS Code manifest can't inherit it, so
/// it is pinned to it here.
#[test]
fn vscode_manifest_names_the_workspace_repository() {
    let raw = std::fs::read_to_string(repo_root().join("integrations/vscode/package.json"))
        .expect("integrations/vscode/package.json");
    let manifest: serde_json::Value = serde_json::from_str(&raw).expect("valid package.json");
    assert_eq!(
        manifest["repository"]["url"].as_str(),
        Some(env!("CARGO_PKG_REPOSITORY"))
    );
}

/// The VS Code extension types `.spec` files with the formatter's default
/// indentation (ADR 0021 D1), so typing indents as `specforge format` will
/// in a project without `.specforgefmt.toml`.
#[test]
fn vscode_manifest_indents_spec_files_as_the_formatter_does() {
    let raw = std::fs::read_to_string(repo_root().join("integrations/vscode/package.json"))
        .expect("integrations/vscode/package.json");
    let manifest: serde_json::Value = serde_json::from_str(&raw).expect("valid package.json");
    let defaults = &manifest["contributes"]["configurationDefaults"]["[specforge]"];
    let formatter = specforge_formatter::FormatConfig::default();

    assert_eq!(
        defaults["editor.tabSize"].as_u64(),
        Some(formatter.indent_width as u64)
    );
    assert_eq!(
        defaults["editor.insertSpaces"].as_bool(),
        Some(!formatter.use_tabs)
    );
}

/// One CLI invocation found in the VS Code extension's sources: the argv
/// elements, each either a string literal (`Some`) or something computed at
/// runtime (`None`, or a template literal kept as its literal prefix).
#[derive(Debug)]
struct VscodeCliCall {
    site: String,
    args: Vec<Option<String>>,
    /// An element is a spread (`...x`): the scan cannot see its flags.
    spread: bool,
}

/// Every argv the VS Code extension hands the `specforge` binary: the array
/// literal passed to `inTask` (a terminal task) or `cliOutput` (stdout).
fn vscode_cli_calls() -> Vec<VscodeCliCall> {
    const CALLERS: [&str; 2] = ["inTask(", "cliOutput("];
    let src = repo_root().join("integrations/vscode/src");
    let mut calls = Vec::new();
    for entry in std::fs::read_dir(&src).expect("integrations/vscode/src") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ts") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read .ts source");
        for caller in CALLERS {
            for (at, _) in text.match_indices(caller) {
                let rest = text[at + caller.len()..].trim_start();
                let Some(body) = rest.strip_prefix('[') else {
                    continue; // a definition or a call with a non-literal argv
                };
                let body = &body[..body.find(']').expect("closing bracket")];
                let line = text[..at].lines().count();
                let spread = body.split(',').any(|a| a.trim().starts_with("..."));
                let args = body
                    .split(',')
                    .map(str::trim)
                    .filter(|a| !a.is_empty())
                    .map(|a| {
                        if let Some(lit) = a.strip_prefix('"').and_then(|a| a.strip_suffix('"')) {
                            Some(lit.to_string())
                        } else {
                            // `--format=${format}` → its literal prefix `--format=`
                            a.strip_prefix('`')
                                .map(|tpl| tpl.split("${").next().unwrap_or("").to_string())
                        }
                    })
                    .collect();
                calls.push(VscodeCliCall {
                    site: format!("{}:{line}", path.file_name().unwrap().to_string_lossy()),
                    args,
                    spread,
                });
            }
        }
    }
    calls
}

/// Every `.ts` source of the VS Code extension: (file name, text).
fn vscode_sources() -> Vec<(String, String)> {
    let src = repo_root().join("integrations/vscode/src");
    let mut files: Vec<_> = std::fs::read_dir(&src)
        .expect("integrations/vscode/src")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ts"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).expect("read .ts source"))
        })
        .collect();
    files.sort();
    files
}

fn vscode_manifest() -> serde_json::Value {
    let raw = std::fs::read_to_string(repo_root().join("integrations/vscode/package.json"))
        .expect("integrations/vscode/package.json");
    serde_json::from_str(&raw).expect("valid package.json")
}

/// No argv element is a spread: the scan above sees only literals, so a
/// `...extArgs` hid `--extension` (the flag is `--extensions`) from it
/// since the first commit (ADR 0249 D4).
#[test]
fn vscode_extension_builds_every_flag_as_a_literal() {
    let spreads: Vec<_> = vscode_cli_calls()
        .into_iter()
        .filter(|c| c.spread)
        .map(|c| format!("{}: a spread element hides its flags from the scan", c.site))
        .collect();
    assert!(spreads.is_empty(), "{}", spreads.join("\n"));
}

/// The commands the extension runs are a list kept on purpose: a new call
/// is added here, and `search` (the registry's search, not the entity
/// search the editor command means) is not on it (ADR 0249 D4).
#[test]
fn vscode_extension_calls_only_the_commands_it_means() {
    const CALLED: [&str; 12] = [
        "analyze", "add", "check", "doctor", "explain", "export", "init", "model", "outline",
        "remove", "stats", "trace",
    ];
    let called: BTreeSet<String> = vscode_cli_calls()
        .iter()
        .filter_map(|c| c.args.first().cloned().flatten())
        .collect();
    let expected: BTreeSet<String> = CALLED.iter().map(|s| s.to_string()).collect();
    assert_eq!(called, expected);
}

/// One file resolves and runs the binaries (ADR 0249 D3): `binary.ts`
/// alone names `specforge-lsp` or spawns a process; no source walks
/// `target/` or types a command into a terminal; the spawn sets a buffer.
#[test]
fn vscode_extension_runs_binaries_by_one_rule() {
    let sources = vscode_sources();
    let spawners: Vec<_> = sources
        .iter()
        .filter(|(_, t)| {
            [
                "specforge-lsp",
                "cp.execFile(",
                "ProcessExecution(",
                "createTerminal(",
            ]
            .iter()
            .any(|m| t.contains(m))
        })
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(spawners, ["binary.ts"], "who resolves or spawns a binary");
    for (name, text) in &sources {
        assert!(!text.contains("\"target\""), "{name} walks target/");
        assert!(!text.contains("sendText("), "{name} types into a terminal");
    }
    let binary = &sources
        .iter()
        .find(|(n, _)| n == "binary.ts")
        .expect("binary.ts")
        .1;
    assert!(binary.contains("maxBuffer"), "binary.ts sets no maxBuffer");
}

/// A hover is marked trusted for one command only: a spec title can hold
/// a `command:` link, and `isTrusted = true` runs any command it names
/// (ADR 0249 D5).
#[test]
fn vscode_hover_trusts_only_the_graph_command() {
    let mut problems = Vec::new();
    for (name, text) in vscode_sources() {
        for (at, _) in text.match_indices("isTrusted") {
            let rest = &text[at + "isTrusted".len()..];
            if !rest.trim_start().starts_with("= { enabledCommands:") {
                let line = text[..at].lines().count();
                problems.push(format!("{name}:{line}: isTrusted without enabledCommands"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The manifest declares what the client provides (ADR 0249 D7): every
/// setting is read, every view is registered, every `node` script runs a
/// file that exists.
#[test]
fn vscode_manifest_declares_only_what_the_extension_provides() {
    let manifest = vscode_manifest();
    let sources = vscode_sources();
    let in_sources = |needle: &str| sources.iter().any(|(_, t)| t.contains(needle));
    let mut problems = Vec::new();
    for key in manifest["contributes"]["configuration"]["properties"]
        .as_object()
        .expect("settings")
        .keys()
    {
        let suffix = key.strip_prefix("specforge.").unwrap_or(key);
        if !in_sources(&format!("\"{suffix}\"")) {
            problems.push(format!("setting {key} is read by no source"));
        }
    }
    for views in manifest["contributes"]["views"]
        .as_object()
        .expect("views")
        .values()
    {
        for view in views.as_array().expect("view list") {
            let id = view["id"].as_str().expect("view id");
            if !in_sources(&format!("\"{id}\"")) {
                problems.push(format!("view {id} has no provider"));
            }
        }
    }
    let root = repo_root().join("integrations/vscode");
    for (script, cmd) in manifest["scripts"].as_object().expect("scripts") {
        if let Some(file) = cmd.as_str().and_then(|c| c.strip_prefix("node ")) {
            let file = file.split_whitespace().next().unwrap_or("");
            let main = manifest["main"].as_str().unwrap_or("");
            if !root.join(file).exists() && file != main {
                problems.push(format!("script {script} runs {file}, which is not there"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The `file:line` of each line of `text` that holds `needle`.
fn lines_with(name: &str, text: &str, needle: &str) -> Vec<String> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| l.contains(needle))
        .map(|(i, _)| format!("{name}:{}: {needle}", i + 1))
        .collect()
}

/// The entity under the cursor is the language server's answer
/// (`specforge/entity`, ADR 0268 D3), not the manifest's word pattern.
#[test]
fn vscode_client_reads_no_word_under_the_cursor() {
    let problems: Vec<_> = vscode_sources()
        .iter()
        .flat_map(|(n, t)| lines_with(n, t, "getWordRangeAtPosition("))
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Every `"specforge/…"` method the client names is one the language server serves
/// (the constants of `views.rs`, ADR 0268).
#[test]
fn vscode_client_asks_only_what_the_server_answers() {
    let literals = |text: &str| -> Vec<String> {
        text.split("\"specforge/")
            .skip(1)
            .map(|rest| format!("specforge/{}", rest.split('"').next().unwrap_or_default()))
            .collect()
    };
    let views = std::fs::read_to_string(repo_root().join("crates/surfaces/lsp/src/views.rs"))
        .expect("crates/surfaces/lsp/src/views.rs");
    let served = literals(&views);
    assert!(!served.is_empty(), "views.rs names no method");
    let problems: Vec<_> = vscode_sources()
        .iter()
        .flat_map(|(name, text)| {
            literals(text)
                .into_iter()
                .filter(|method| !served.contains(method))
                .map(|method| format!("{name}: `{method}` is no method views.rs serves"))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The init and add pickers offer the builtins that declare kinds, each with its kinds, as
/// `builtin-vocabulary.json` groups them (ADR 0268 D5): the client keeps the list because
/// Initialize Project runs before a project exists, and this pin fails the moment a builtin gains
/// or loses a kind.
#[test]
fn vscode_pickers_offer_the_builtins_that_declare_kinds() {
    use std::collections::BTreeMap;
    let commands = vscode_sources()
        .into_iter()
        .find(|(name, _)| name == "commands.ts")
        .expect("commands.ts")
        .1;
    // Each `label: "@specforge/…"` and the `description: "…"` that follows it (maybe on the next line).
    let quoted_after = |text: &str, key: &str| -> Option<String> {
        let rest = &text[text.find(key)? + key.len()..];
        let rest = rest.trim_start().strip_prefix('"')?;
        Some(rest[..rest.find('"')?].to_string())
    };
    let mut offered: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (at, _) in commands.match_indices("label: \"@specforge/") {
        let tail = &commands[at..];
        let label = quoted_after(tail, "label:").expect("a label");
        let description = quoted_after(tail, "description:").expect("a description");
        offered.insert(label, description.split(", ").map(str::to_string).collect());
    }
    let vocabulary: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            repo_root().join("crates/runtime/component/data/builtin-vocabulary.json"),
        )
        .expect("builtin-vocabulary.json"),
    )
    .expect("valid vocabulary JSON");
    let mut declared: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (kind, builtin) in vocabulary["kinds"].as_object().expect("kinds") {
        declared
            .entry(builtin.as_str().expect("builtin name").to_string())
            .or_default()
            .insert(kind.clone());
    }
    assert_eq!(offered, declared, "the Initialize Project picker");

    // The Add Extension picker lists the same builtins.
    let add = &commands[commands
        .find("\"specforge.addExtension\"")
        .expect("the add command")..];
    let add = &add[..add.find("{ placeHolder").expect("the add picker's options")];
    let listed: BTreeSet<String> = add
        .split('"')
        .filter(|w| w.starts_with("@specforge/"))
        .map(str::to_string)
        .collect();
    assert_eq!(
        listed,
        declared.keys().cloned().collect::<BTreeSet<_>>(),
        "the Add Extension picker"
    );
}

/// The tree and the graph show the session's graph (`specforge/graph`,
/// ADR 0268 D1): no `specforge` process reads the disk for them, and the
/// graph view counts no degree of its own (the host's rule drops self-edges).
#[test]
fn vscode_tree_and_graph_read_the_language_server() {
    let problems: Vec<_> = vscode_sources()
        .iter()
        .filter(|(n, _)| n == "entity-tree.ts" || n == "graph-webview.ts")
        .flat_map(|(n, t)| {
            let mut found = lines_with(n, t, "cliOutput(");
            found.extend(lines_with(n, t, "entityDegree.set("));
            found
        })
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
#[ignore = "flipped by T3"]
fn vscode_graph_panel_reads_every_field_the_client_posts() {
    let text =
        std::fs::read_to_string(repo_root().join("integrations/vscode/src/graph-webview.ts"))
            .expect("read graph-webview.ts");
    let start = text
        .find("<script nonce")
        .expect("the panel's inline script");
    let end = start + text[start..].find("</script>").expect("its end");
    let (script, client) = (
        &text[start..end],
        format!("{}{}", &text[..start], &text[end..]),
    );
    let mut problems = Vec::new();
    for (at, _) in client.match_indices("postMessage({") {
        let body = &client[at + "postMessage({".len()..];
        let body = &body[..body.find("})").expect("a closed object literal")];
        let kind = body.split('"').nth(1).unwrap_or("?");
        for piece in body
            .lines()
            .filter(|l| !l.trim().starts_with("//"))
            .flat_map(|l| l.split(','))
        {
            let key = piece.split(':').next().unwrap_or("").trim();
            let is_key = !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric());
            if is_key && key != "type" && !script.contains(&format!("msg.{key}")) {
                problems.push(format!(
                    "{kind} posts {key}, which the panel never reads (msg.{key})"
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// A kind's icon, colour and order come from its declared symbol kind
/// (ADR 0268 D4): no source keeps a table keyed by three or more builtin kinds.
#[test]
fn vscode_client_keeps_no_kind_table() {
    let kinds = manifest_domain_kinds();
    let mut problems = Vec::new();
    for (name, text) in vscode_sources() {
        let named: Vec<&String> = kinds
            .iter()
            .filter(|k| {
                [format!("{k}:"), format!("\"{k}\""), format!("'{k}'")]
                    .iter()
                    .any(|p| text.contains(p.as_str()))
            })
            .collect();
        if named.len() >= 3 {
            problems.push(format!(
                "{name}: kinds {named:?} are named as keys or literals"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

fn specforge_help(args: &[&str]) -> String {
    let mut argv = args.to_vec();
    argv.push("--help");
    let out = Cli::on(&TestProject::new().without_config()).bare(&argv);
    assert_eq!(
        out.code,
        0,
        "`specforge {} --help` failed: {}",
        args.join(" "),
        out.stderr
    );
    out.stdout
}

/// The VS Code extension shells out to the CLI. Every subcommand it names
/// must exist, and every `--flag` it passes must be one that subcommand
/// takes; otherwise the editor command fails at runtime with nothing to
/// show (it called `specforge inspect` and `specforge list`, neither of
/// which exists).
#[test]
fn vscode_extension_invokes_only_existing_cli_commands() {
    let calls = vscode_cli_calls();
    assert!(
        calls.len() >= 10,
        "the scan found too few CLI calls ({}); the call helpers were renamed",
        calls.len()
    );
    let top = specforge_help(&[]);
    let subcommands: BTreeSet<String> = top
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect();

    let mut problems = Vec::new();
    for call in &calls {
        let Some(Some(sub)) = call.args.first() else {
            problems.push(format!("{}: the subcommand is not a literal", call.site));
            continue;
        };
        if !subcommands.contains(sub) {
            problems.push(format!("{}: `specforge {sub}` does not exist", call.site));
            continue;
        }
        let help = specforge_help(&[sub]);
        for flag in call.args.iter().skip(1).flatten() {
            if !flag.starts_with("--") {
                continue;
            }
            let name = flag.split('=').next().unwrap();
            if !help.contains(&format!("{name} ")) && !help.contains(&format!("{name}\n")) {
                problems.push(format!(
                    "{}: `specforge {sub}` takes no `{name}` flag",
                    call.site
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
