//! C2-06: entity-kind counts in the docs are derived claims — they rot
//! silently when an extension adds a kind. Derive the number from the
//! builtin manifests and pin the docs to it.

use std::collections::BTreeSet;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn manifest_domain_kinds() -> BTreeSet<String> {
    let mut kinds = BTreeSet::new();
    // The builtins' pinned wire declarations (the SDK greet fixture beside
    // them is not a builtin).
    let pinned = repo_root().join("crates/specforge-component/tests/declarations");
    for entry in std::fs::read_dir(&pinned).expect("pinned declarations") {
        let dir = entry.unwrap().path();
        let handshake: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("handshake.json")).expect("pinned handshake"),
        )
        .expect("valid handshake JSON");
        if !handshake["name"]
            .as_str()
            .is_some_and(|n| n.starts_with("@specforge/"))
        {
            continue;
        }
        let path = dir.join("describe_entities.json");
        let raw =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let doc: serde_json::Value = serde_json::from_str(&raw).expect("valid describe JSON");
        for item in doc["items"].as_array().expect("items array") {
            let keyword = item["keyword"]
                .as_str()
                .or_else(|| item["name"].as_str())
                .expect("kind keyword");
            kinds.insert(keyword.to_string());
        }
    }
    kinds
}

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

/// One CLI invocation found in the VS Code extension's sources: the argv
/// elements, each either a string literal (`Some`) or something computed at
/// runtime (`None`, or a template literal kept as its literal prefix).
#[derive(Debug)]
struct VscodeCliCall {
    site: String,
    args: Vec<Option<String>>,
}

/// Every argv the VS Code extension hands the `specforge` binary: the array
/// literal passed to `runCliCommand`, `runCliCommandOutput`, `execCli`, or
/// `cp.execFile(findCliBinary(), [...])`.
fn vscode_cli_calls() -> Vec<VscodeCliCall> {
    const CALLERS: [&str; 4] = [
        "runCliCommand(",
        "runCliCommandOutput(",
        "execCli(",
        "findCliBinary(),",
    ];
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
                });
            }
        }
    }
    calls
}

fn specforge_help(args: &[&str]) -> String {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_specforge"))
        .args(args)
        .arg("--help")
        .output()
        .expect("run specforge --help");
    assert!(
        out.status.success(),
        "`specforge {} --help` failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
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

#[test]
fn entity_model_doc_count_matches_builtin_manifests() {
    let kinds = manifest_domain_kinds();
    let count = kinds.len();
    assert_eq!(
        count, 22,
        "builtin manifest kinds drifted; update docs/entity-model.md"
    );

    let doc = std::fs::read_to_string(repo_root().join("docs/entity-model.md"))
        .expect("docs/entity-model.md");
    let claimed = format!("{count}, declared by the four builtin extensions");
    assert!(
        doc.contains(&claimed),
        "entity-model.md must claim '{claimed}' — the manifest-derived count changed"
    );
    let total = 2 + count;
    assert!(
        doc.contains(&format!("= {total} entity kinds")),
        "entity-model.md total must equal 2 structural + {count} domain = {total}"
    );
}
