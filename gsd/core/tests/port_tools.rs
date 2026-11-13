//! A ported line names only tools that exist: every `specforge <verb>` it adds is a verb of the CLI,
//! every `specforge gsd <command>` is a declared command, and no agent body keeps a blind rename.

use std::path::{Path, PathBuf};

use regex::Regex;

use asset_code::fenced_code;
use host_options::HOST_OPTIONS;

#[path = "common/asset_code.rs"]
mod asset_code;
#[path = "common/command_word.rs"]
mod command_word;
#[path = "common/files.rs"]
mod files;
#[path = "common/host_options.rs"]
mod host_options;
#[path = "common/ledger.rs"]
mod ledger;
#[path = "common/port_map.rs"]
mod port_map;
#[path = "common/specforge_only.rs"]
mod specforge_only;
#[path = "port_tools/state_files.rs"]
mod state_files;
#[path = "port_tools/writes.rs"]
mod writes;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every asset as `(path under assets/, its lines after the header)`.
fn every_asset() -> Vec<(String, Vec<String>)> {
    asset_code::assets_under("assets")
}

/// The verbs of the `specforge` CLI that ported lines name.
const VERBS: &[&str] = &[
    "add", "anchors", "check", "collect", "explain", "export", "files", "format", "git", "grants",
    "gsd", "http", "init", "install", "link", "new", "query", "run", "schema", "search", "set",
    "settings", "trace", "wait",
];

/// Commands of `@gsd/security` that core's text asks for when it is loaded (`security-hooks` for the security gate).
const EXTENSION_COMMANDS: &[&str] = &["security-gate", "security-hooks", "threat-ids"];

#[test]
fn a_ported_line_names_only_declared_tools() {
    let declaration = gsd_core::build().declaration();
    let commands: Vec<String> = declaration
        .surfaces
        .commands
        .iter()
        .map(|c| c.id.replace('_', "-"))
        .collect();
    let verb = Regex::new(r"specforge ([a-z][a-z0-9-]*)(?: ([a-z][a-z0-9-]*))?").expect("a regex");
    let mut seen = 0;
    for (path, lines) in every_asset() {
        let own = port_map::map(&path).into_iter().filter_map(|s| match s {
            port_map::Span::Own { asset, .. } => Some(asset),
            port_map::Span::Kept { .. } => None,
        });
        for n in own.flatten() {
            let mine = &lines[n];
            // the one prose phrase every shell block opens with
            let mine = mine.replace("are specforge commands", "");
            for cap in verb.captures_iter(&mine) {
                seen += 1;
                assert!(
                    VERBS.contains(&&cap[1]),
                    "{path}:{n}: `specforge {}` is not a verb the CLI has",
                    &cap[1]
                );
                if &cap[1] == "gsd" {
                    // a bare `specforge gsd` names the namespace
                    let name = cap.get(2).map_or("", |m| m.as_str());
                    assert!(
                        name.is_empty()
                            || EXTENSION_COMMANDS.contains(&name)
                            || commands.iter().any(|c| c == name),
                        "{path}:{n}: `specforge gsd {name}` is not a declared command"
                    );
                }
            }
        }
    }
    assert!(seen > 100, "only {seen} tools named");
}

#[test]
fn no_body_names_spec_gsd_or_gsd_cjs() {
    let rx = Regex::new(r"specforge gsd query|specforge gsd\.cjs|spec/gsd/").expect("a regex");
    let mut checked = 0;
    for (path, lines) in every_asset() {
        for (n, line) in lines.iter().enumerate() {
            assert!(!rx.is_match(line), "{path}:{n}: {line}");
        }
        checked += 1;
    }
    assert!(checked > 100, "{checked} assets");
}

#[test]
fn the_recording_section_matches_k17() {
    let text = gsd_core::agents::recording("gsd-verifier");
    assert!(!text.contains("E080"), "{text}");
    assert!(text.contains("replaces an earlier pass"), "{text}");
}

/// The `node` lines a shell block keeps, as `(asset, trimmed line)`. Each runs the user's own
/// JavaScript project, where node is that project's runtime and the prompt's reader is not SpecForge.
const KEEP: &[(&str, &str)] = &[
    // runs the user's own CLI to check it lists a subcommand
    (
        "agents/gsd-verifier.md",
        r#"node $CLI_PATH --help 2>&1 | grep -q "$EXPECTED_SUBCOMMAND""#,
    ),
];

/// A shell block runs no node: a project that uses SpecForge may have no node at all. Only code counts:
/// a line inside a fence that is not a `#` comment. Prose about a node script is not code.
#[test]
fn a_shell_block_runs_no_node() {
    let mut found = Vec::new();
    let listed: Vec<&(&str, &str)> = KEEP.iter().collect();
    let mut matched = vec![0; listed.len()];
    // a node invocation: `node -e|-p|--eval|--print`, `node "<file>"` / `node '<file>'` / `node $X`, `node <<`, or
    // `node \` continued on the next line
    let node = regex::Regex::new(
        r#"(^|[\s(;|&`])node(\s+(-e|-p|--eval|--print)\b|\s+["'$]|\s*<<|\s*\\$)"#,
    )
    .expect("a regex");
    for (line, is_node) in [
        ("X=$(node --eval 'x')", true),
        ("node --print 1", true),
        ("node -p 1 | jq .", true),
        ("node \"$F\" a", true),
        ("node $F a", true),
        ("node <<'EOF'", true),
        ("FOO=1 node \\", true),
        ("Get-Process node -ErrorAction SilentlyContinue", false),
        ("ls node_modules/x", false),
        ("node --version", false),
    ] {
        assert_eq!(node.is_match(line), is_node, "{line}");
    }
    for (path, lines) in every_asset() {
        for (n, code) in fenced_code(&lines) {
            if !node.is_match(code) {
                continue;
            }
            if let Some(k) = listed.iter().position(|(p, l)| *p == path && *l == code) {
                matched[k] += 1;
            } else {
                // the asset's line number: its line 1 is the attribution header
                found.push(format!("{path}:{}: {code}", n + 2));
            }
        }
    }
    assert!(
        found.is_empty(),
        "node reads in shell blocks:\n{}",
        found.join("\n")
    );
    for ((path, line), count) in listed.iter().zip(&matched) {
        assert_eq!(*count, 1, "KEEP ({path}, {line}) matches {count} lines");
    }
}

/// No shell block and no inline-code run of the prose runs jq: a SpecForge answer is
/// read with `--pick`, a third party's answer by the agent, and gh filters with its own `--jq`. Git Bash on Windows has
/// no jq, and a `| jq -r` there leaves the variable empty (upstream #2589).
#[test]
fn no_shell_line_runs_jq() {
    let mut found = Vec::new();
    for (path, lines) in specforge_only::shell_assets() {
        for (n, code) in fenced_code(&lines) {
            if command_word::runs(code, "jq") {
                // the asset's line number: its line 1 is the attribution header
                found.push(format!("{path}:{}: {code}", n + 2));
            }
        }
        for (n, run) in specforge_only::inline_runs(&lines) {
            if command_word::runs_in_prose(&run, "jq") {
                found.push(format!("{path}:{}: `{run}`", n + 2));
            }
        }
    }
    assert!(
        found.is_empty(),
        "{} jq runs; read a SpecForge answer with --pick:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// R0 of "SpecForge serves itself": a shell block, or an inline run of the prose, of an asset runs only `specforge`
/// and the shell's own words; what still runs another tool is a row of `pending_tools.txt` (`common/specforge_only.rs`).
#[test]
fn a_shell_block_calls_only_specforge() {
    let assets = specforge_only::shell_assets();
    let functions = specforge_only::functions(assets.iter().map(|(_, lines)| &lines[..]));
    let hits: Vec<specforge_only::Row> = assets
        .iter()
        .flat_map(|(path, lines)| specforge_only::hits(path, lines, &functions))
        .collect();
    specforge_only::assert_pending(
        &hits,
        specforge_only::pending(|f| !f.starts_with("prompt:")),
    );
}

/// A prose line or a routing probe is reworded (ADR 0343 D7, D8), never listed: the `H` and `K` groups of the ledger
/// keep their headings, at 0 rows, and hold no row.
#[test]
fn the_prose_and_routing_groups_are_empty() {
    let plan_id = Regex::new(r"\(R\d+\)").expect("a regex");
    let mut group = "";
    let mut seen = Vec::new();
    for line in specforge_only::PENDING_TOOLS.lines() {
        if let Some(rest) = line.strip_prefix("# ")
            && rest.as_bytes().get(1) == Some(&b' ')
            && rest.as_bytes()[0].is_ascii_uppercase()
        {
            group = &rest[..1];
            seen.push(group);
            if matches!(group, "H" | "K") {
                assert!(line.ends_with(" 0 rows."), "{line}");
            }
            // a heading names its building block, never the plan that builds it
            assert!(!plan_id.is_match(line), "{line}");
            // H says what ADR 0343 D8 says: nothing is probed, and no setting turns an integration on
            if group == "H" {
                assert!(
                    line.contains("never probed") && !line.contains("a setting"),
                    "{line}"
                );
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            assert!(
                !matches!(group, "H" | "K"),
                "a row under group {group}: {line}"
            );
        }
    }
    assert!(seen.contains(&"H") && seen.contains(&"K"), "{seen:?}");
}

/// One declared command as `(name, positionals in order, options and flags with their kind)`.
struct Shape {
    positionals: Vec<String>,
    /// `--long` -> takes a value
    options: Vec<(String, bool)>,
}

fn shapes() -> std::collections::BTreeMap<String, Shape> {
    let declaration = gsd_core::build().declaration();
    let mut out = std::collections::BTreeMap::new();
    for c in &declaration.surfaces.commands {
        let mut shape = Shape {
            positionals: Vec::new(),
            options: Vec::new(),
        };
        for a in serde_json::to_value(&c.args)
            .expect("args as JSON")
            .as_array()
            .expect("an array")
        {
            let name = a["name"].as_str().expect("a name").replace('_', "-");
            let flag = a["arg_type"] == "bool";
            if flag {
                shape.options.push((name, false));
            } else if a["required"] == true {
                shape.positionals.push(name);
            } else {
                shape.options.push((name, true));
            }
        }
        out.insert(c.id.replace('_', "-"), shape);
    }
    out
}

/// The words of a shell-ish tail, up to the end of the command: quotes (and `$( )` inside them) are one word.
fn words(rest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut chars = rest.chars().peekable();
    let mut quote: Option<char> = None;
    let mut depth = 0;
    let mut angle = false;
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(_), '$') if chars.peek() == Some(&'(') => {
                depth += 1;
                word.push_str("$(");
                chars.next();
            }
            (_, ')') if depth > 0 => {
                depth -= 1;
                word.push(c);
            }
            (_, _) if depth > 0 => word.push(c),
            (Some(q), _) if c == q => {
                quote = None;
                word.push(c);
            }
            (Some(_), _) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                word.push(c);
            }
            (None, '<') if word.is_empty() => {
                angle = true;
                word.push(c);
            }
            (None, '>') if !angle && matches!(word.as_str(), "1" | "2") => {
                word.clear();
                break;
            }
            (None, '>') if angle => {
                angle = false;
                word.push(c);
            }
            (None, '`' | ')' | ';' | '|' | '>' | '&' | '#' | '\n') if !angle => break,
            (None, c) if c.is_whitespace() => {
                if !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                }
            }
            (None, '.') if chars.peek().is_none_or(|n| n.is_whitespace()) => {
                // prose: the sentence ends here
                break;
            }
            (None, c) => word.push(c),
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

/// Why `specforge gsd <command> <words>` is not a call the CLI accepts, if it is not.
fn refusal(
    shapes: &std::collections::BTreeMap<String, Shape>,
    command: &str,
    words: &[String],
) -> Option<String> {
    // a bare mention names the command
    let shape = shapes.get(command).filter(|_| !words.is_empty())?;
    let mut positionals: Vec<&str> = Vec::new();
    let mut open = false;
    let mut i = 0;
    while i < words.len() {
        let w = words[i].as_str();
        if w == "..." || w.starts_with('[') {
            open = true;
        } else if let Some(long) = w.strip_prefix("--") {
            let long = long.split('=').next().unwrap_or(long);
            let declared = shape.options.iter().find(|(n, _)| n == long);
            // the CLI's own options belong to no command, unless the command declares the name
            if let Some((_, takes_value)) = HOST_OPTIONS
                .iter()
                .find(|(name, _)| *name == long)
                .filter(|_| declared.is_none())
            {
                i += if *takes_value && !w.contains('=') {
                    2
                } else {
                    1
                };
                continue;
            }
            match declared {
                None => return Some(format!("unexpected argument '--{long}'")),
                Some((_, true)) if !w.contains('=') => i += 1,
                _ => {}
            }
        } else {
            positionals.push(w);
        }
        i += 1;
    }
    if positionals.len() > shape.positionals.len() {
        return Some(format!(
            "{} positionals, the command takes {:?}",
            positionals.len(),
            shape.positionals
        ));
    }
    if !open && positionals.len() < shape.positionals.len() {
        return Some(format!(
            "{} positionals, the command needs {:?}",
            positionals.len(),
            shape.positionals
        ));
    }
    for (at, p) in positionals.iter().enumerate() {
        let named = p.trim_matches(|c| matches!(c, '<' | '>'));
        if let Some(declared) = shape
            .positionals
            .iter()
            .position(|n| n == named)
            .filter(|d| *d != at)
        {
            return Some(format!("'{p}' is positional {declared}, written at {at}"));
        }
    }
    None
}

/// Every `specforge gsd <command> ...` of `text` that the CLI would refuse.
fn refused_invocations(text: &str) -> Vec<String> {
    let shapes = shapes();
    let rx = Regex::new(r"specforge gsd ([a-z][a-z0-9-]*)([^\n]*)").expect("a regex");
    // A Rust string continues a line with `\` at its end.
    let joined = Regex::new(r"\\\n\s*")
        .expect("a regex")
        .replace_all(text, "");
    let mut found = Vec::new();
    for cap in rx.captures_iter(&joined) {
        // a call written inside a double-quoted string ends where the string does
        let at = cap.get(0).map_or(0, |m| m.start());
        let line_start = joined[..at].rfind('\n').map_or(0, |n| n + 1);
        let inside = joined[line_start..at].matches('"').count() % 2 == 1;
        let rest = if inside {
            cap[2].split('"').next().unwrap_or("")
        } else {
            &cap[2]
        };
        if let Some(why) = refusal(&shapes, &cap[1], &words(rest)) {
            found.push(format!("`specforge gsd {}{}`: {why}", &cap[1], &cap[2]));
        }
    }
    found
}

#[test]
fn every_gsd_invocation_in_an_asset_or_a_prompt_fits_its_command() {
    let files: Vec<PathBuf> = ["assets", "src", "../kit/src"]
        .iter()
        .flat_map(|dir| files::under(&root().join(dir)))
        .collect();
    let mut bad = Vec::new();
    let mut seen = 0;
    for path in files.iter().filter(|p| {
        p.extension().is_some_and(|e| e == "md" || e == "rs")
            && !p.to_string_lossy().contains("/tests")
    }) {
        let text = std::fs::read_to_string(path).expect("a file");
        seen += text.matches("specforge gsd ").count();
        for why in refused_invocations(&text) {
            bad.push(format!(
                "{}: {why}",
                path.strip_prefix(root()).expect("under root").display()
            ));
        }
    }
    assert!(seen > 60, "only {seen} invocations");
    assert!(bad.is_empty(), "the CLI refuses:\n{}", bad.join("\n"));
}

/// Every line of every shell block of the assets the ledger scans, with its asset's path.
fn shell_lines() -> Vec<(String, String)> {
    specforge_only::shell_assets()
        .into_iter()
        .flat_map(|(path, lines)| {
            specforge_only::shell_blocks(&lines)
                .into_iter()
                .flatten()
                .map(|line| (path.clone(), line.to_string()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// A graph read picks the export's `nodes` or `edges`: `specforge query` prints a subgraph's export, and `--pick` of a
/// key it lacks (an upstream init-bundle key: `phase_dir`, `roadmap_path`, …) prints nothing (ADR 0343 D3).
#[test]
fn every_graph_read_picks_nodes_or_edges() {
    let mut found = Vec::new();
    for (path, line) in shell_lines() {
        let Some(at) = line.find("specforge query") else {
            continue;
        };
        let Some(pick) = line[at..].find("--pick ") else {
            continue;
        };
        let rest = &line[at + pick + "--pick ".len()..];
        let word = rest
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|c| c == '\'' || c == '"');
        if !word.starts_with(".nodes") && !word.starts_with(".edges") {
            found.push(format!("{path}: {}", line.trim()));
        }
    }
    assert!(
        found.is_empty(),
        "{} graph reads pick a key the export lacks:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// Files still kept under the phase directory, each waiting for a home of its own: they are not entities.
const PHASE_DIR_FILES: &[&str] = &[
    // REVIEWS.md is written by /gsd:review at a path the model composes; no review entity answers it yet
    "REVIEWS.md",
];

/// No shell line builds a path under a phase directory: nothing answers one, and the files it held are entities (ADR
/// 0343 D3), save [`PHASE_DIR_FILES`].
#[test]
fn no_shell_line_reads_a_phase_directory() {
    let dir = r#"\$\{?(PHASE_DIR|phase_dir|PHASE_DIR_FOR_SPEC)\}?"?"#;
    let rx = Regex::new(&format!(
        r#"{dir}/|specforge files (list|mkdir) [^|]*"?{dir}"#
    ))
    .expect("a regex");
    let mut found = Vec::new();
    for (path, line) in shell_lines() {
        let code = line.trim_start();
        if !code.starts_with('#')
            && rx.is_match(code)
            && !PHASE_DIR_FILES.iter().any(|f| code.contains(f))
        {
            found.push(format!("{path}: {}", line.trim()));
        }
    }
    assert!(
        found.is_empty(),
        "{} shell lines read a phase directory:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// No asset reads an `@file:` payload: SpecForge prints its answer, never a marker naming a file that holds it
/// (ADR 0343 D4).
#[test]
fn no_asset_reads_a_file_payload() {
    let found: Vec<String> = specforge_only::shell_assets()
        .into_iter()
        .flat_map(|(path, lines)| {
            lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.contains("@file:"))
                .map(|(n, l)| format!("{path}:{}: {l}", n + 2))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(found.is_empty(), "@file: reads:\n{}", found.join("\n"));
}

/// progress.md's routes name their commands: no line reads a verification value that nothing in the file assigns
/// (the verification entity has no next-action or next-command field, so such a value renders empty).
#[test]
fn progress_reads_only_the_verification_values_it_assigns() {
    let text = std::fs::read_to_string(root().join("assets/gsd-core/workflows/progress.md"))
        .expect("progress.md");
    let read = Regex::new(r"\$\{?(VERIFICATION_[A-Z_]+)").expect("a regex");
    let mut unassigned: Vec<&str> = read
        .captures_iter(&text)
        .filter_map(|c| c.get(1))
        .map(|m| m.as_str())
        .filter(|name| !text.contains(&format!("{name}=")))
        .collect();
    unassigned.sort_unstable();
    unassigned.dedup();
    assert!(
        unassigned.is_empty(),
        "progress.md reads values nothing assigns: {unassigned:?}"
    );
}

/// plan-phase's agent prompts name entities read with `specforge`, never a value with no source: the project state is
/// computed, REVIEWS.md is no entity and a phase has no directory.
#[test]
fn plan_phase_prompts_name_no_unsourced_path() {
    let text = std::fs::read_to_string(root().join("assets/gsd-core/workflows/plan-phase.md"))
        .expect("plan-phase.md");
    for unsourced in ["{state_path}", "{reviews_path}", "{PHASE_DIR}/"] {
        assert!(!text.contains(unsourced), "plan-phase.md names {unsourced}");
    }
}

/// execute-phase runs no worktree isolation, so neither it, its steps nor its wave references set or read the
/// isolation values: each was a literal, and every guard on one was a branch no input reaches (ADR 0343 D4).
#[test]
fn execute_phase_holds_no_isolation_value() {
    let base = root().join("assets/gsd-core");
    let mut files = vec![base.join("workflows/execute-phase.md")];
    files.extend(files::under(&base.join("workflows/execute-phase")));
    for entry in std::fs::read_dir(base.join("references"))
        .expect("references")
        .flatten()
    {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("execute-phase-")
        {
            files.push(entry.path());
        }
    }
    let mut found = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("an asset");
        for (n, line) in text.lines().enumerate() {
            if line.contains("ISOLATION") || line.contains("USE_WORKTREES") {
                found.push(format!("{}:{}: {line}", file.display(), n + 1));
            }
        }
    }
    assert!(files.len() > 10, "only {} files", files.len());
    assert!(found.is_empty(), "isolation values:\n{}", found.join("\n"));
}

/// A list of keys to read from an answer ("Parse JSON for: …", "Extract from init JSON: …") names only keys something
/// prints: no init bundle prints a model, a phase directory, a padded phase, a section manifest or a path, so a list
/// naming one sends the model after a value that is never there. A key a JSON literal of the same file sets is printed.
#[test]
fn no_parse_list_names_a_key_nothing_prints() {
    let list = Regex::new(r"^Parse:|(?:Parse|Extract|From|Check)\b[^.]*?\b(?:JSON|init|INIT\w*)\b")
        .expect("a regex");
    let key = Regex::new(r"`([a-z][a-z0-9_]*)`").expect("a regex");
    let relic =
        Regex::new(r"^(?:[a-z_]+_model|phase_dir|padded_phase|section_manifest|[a-z_]+_path)$")
            .expect("a regex");
    let mut found = Vec::new();
    let mut lists = 0;
    for (path, lines) in asset_code::assets_under("assets") {
        let text = lines.join("\n");
        let mut in_list = false;
        for (n, line) in lines.iter().enumerate() {
            in_list = list.is_match(line) || (in_list && line.starts_with('`'));
            if !in_list {
                continue;
            }
            lists += 1;
            let unprinted: Vec<&str> = key
                .captures_iter(line)
                .filter_map(|c| c.get(1))
                .map(|m| m.as_str())
                .filter(|k| relic.is_match(k) && !text.contains(&format!("\"{k}\":")))
                .collect();
            if !unprinted.is_empty() {
                found.push(format!("{path}:{}: {unprinted:?}", n + 2));
            }
        }
    }
    assert!(lists > 20, "only {lists} list lines");
    assert!(
        found.is_empty(),
        "lists naming keys nothing prints:\n{}",
        found.join("\n")
    );
}

/// No line probes whether a program is installed (ADR 0343 D8): a user's program runs through `specforge run --`,
/// whose exit 127 says it is not installed.
#[test]
fn no_line_probes_for_a_program() {
    let mut found = Vec::new();
    for (path, lines) in asset_code::assets_under("assets") {
        for (n, line) in lines.iter().enumerate() {
            if line.contains("run --which") || line.contains("command -v") {
                found.push(format!("{path}:{}: {line}", n + 2));
            }
        }
    }
    assert!(found.is_empty(), "probes:\n{}", found.join("\n"));
}

/// A host file outside the roots (the user's profile, upstream's global defaults) is read with the host's Read tool
/// when it exists (ADR 0343 D9): no block reaches it, and no fence stands in for it with an empty value.
#[test]
fn a_host_file_outside_the_roots_is_read_with_the_read_tool() {
    for (path, file, stand_in) in [
        (
            "gsd-core/workflows/discuss-phase/modes/advisor.md",
            "~/.claude/gsd-core/USER-PROFILE.md",
            "PROFILE_CONTENT=",
        ),
        (
            "gsd-core/workflows/new-project.md",
            "${GSD_HOME:-$HOME}/.gsd/defaults.json",
            "DEFAULTS_JSON=",
        ),
    ] {
        let text = std::fs::read_to_string(root().join("assets").join(path)).expect("an asset");
        let says = text
            .lines()
            .any(|l| l.contains(&format!("`{file}`")) && l.contains("with your Read tool"));
        assert!(says, "{path}: no line reads {file} with the Read tool");
        assert!(!text.contains(stand_in), "{path}: a fence sets {stand_in}");
    }
}

/// One story per agent: the researcher records its research as entities with `specforge new` and writes no file, and
/// the pattern mapper's report is its reply. Neither tells the agent to use the Write tool beside that.
#[test]
fn an_agent_that_writes_no_file_names_no_write_tool() {
    let agent = |name: &str| {
        std::fs::read_to_string(root().join("assets/agents").join(name)).expect("an agent")
    };
    let researcher = agent("gsd-phase-researcher.md");
    assert!(researcher.contains("specforge new research"));
    for (name, text) in [
        ("gsd-phase-researcher.md", researcher.clone()),
        ("gsd-pattern-mapper.md", agent("gsd-pattern-mapper.md")),
        (
            "gsd-pattern-mapper.compact.md",
            agent("gsd-pattern-mapper.compact.md"),
        ),
    ] {
        for said in [
            "use the Write tool",
            "the Write tool —",
            "Use the `Write` tool",
            "single `Write` call",
            "only file you write",
            "written to correct phase directory",
            "content lives on disk",
        ] {
            assert!(!text.contains(said), "{name} says {said:?}");
        }
    }
}
