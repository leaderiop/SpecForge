//! A ported line names only tools that exist (REVIEW-r40 M6): every `specforge <verb>` it adds is a verb of the CLI,
//! every `specforge gsd <command>` is a declared command, and no agent body keeps a blind rename.

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "common/command_word.rs"]
mod command_word;
#[path = "common/specforge_only.rs"]
mod specforge_only;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Body lines of an upstream file; a command or agent file loses its frontmatter.
fn upstream_body(relative: &str) -> Vec<String> {
    let base = Path::new(gsd_kit::snapshot!());
    let plain = base.join(relative);
    let file = if plain.is_file() {
        plain
    } else {
        base.join(format!("{relative}.txt"))
    };
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    if relative.starts_with("commands/") || relative.starts_with("agents/") {
        let end = lines[1..]
            .iter()
            .position(|l| l == "---")
            .expect("a closing ---")
            + 1;
        lines.drain(..=end);
    }
    lines
}

/// Every asset as `(path under assets/, its lines after the header, upstream's body)`.
fn every_asset() -> Vec<(String, Vec<String>, Vec<String>)> {
    let assets = root().join("assets");
    let mut paths = Vec::new();
    walk(&assets, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| {
            let relative = p
                .strip_prefix(&assets)
                .expect("under assets")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&p).expect("an asset");
            let lines: Vec<String> = text.split('\n').skip(1).map(String::from).collect();
            let body = upstream_body(&relative);
            (relative, lines, body)
        })
        .collect()
}

/// The verbs of the `specforge` CLI that ported lines name.
const VERBS: &[&str] = &[
    "add", "check", "collect", "export", "format", "gsd", "init", "install", "new", "query",
    "schema", "set", "settings",
];

/// Commands of `@gsd/security` that core's text asks for when it is loaded (plan 16 T5, T6; `security-hooks` for the
/// security gate, plan 05 review R9).
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
    for (path, lines, body) in every_asset() {
        for (n, (mine, theirs)) in lines.iter().zip(&body).enumerate() {
            if mine == theirs {
                continue;
            }
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
    for (path, lines, _) in every_asset() {
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

/// Plan 03 (F9) V8: the `node` lines a shell block keeps, as `(asset, trimmed line)`. Each runs the user's own
/// JavaScript project, where node is that project's runtime and the prompt's reader is not SpecForge.
const KEEP: &[(&str, &str)] = &[
    // the user's dev server (`npm run dev` above it): its readiness probe, as upstream wrote it
    (
        "gsd-core/references/checkpoints.md",
        r#"for i in $(seq 1 30); do node -e "fetch(\"http://localhost:3000\").then(r=>{process.exit(r.ok?0:1)}).catch(()=>process.exit(1))" 2>/dev/null && break; sleep 1; done"#,
    ),
    // runs the user's own CLI to check it lists a subcommand
    (
        "agents/gsd-verifier.md",
        r#"node $CLI_PATH --help 2>&1 | grep -q "$EXPECTED_SUBCOMMAND""#,
    ),
    // loads a module of the user's project to check what it exports
    (
        "agents/gsd-verifier.md",
        r#"node -e "const m = require('$MODULE_PATH'); console.log(typeof m.$FUNCTION_NAME)" 2>/dev/null | grep -q "function""#,
    ),
];

/// The `node` lines plan 03 left, as `(asset, trimmed line)`, each with its reason. Each must match exactly one line,
/// as a KEEP row must, so the list cannot go stale. None reads SpecForge's JSON. They run upstream code that @gsd/core
/// does not install, and the step that holds them is optional; gsd/core/README.md names them.
const PENDING_NODE: &[(&str, &str)] = &[
    // the disposition-ledger builder: a ~200-line script that parses REVIEW.md and REVIEW-FIX.md Markdown and rewrites
    // the ledger. It reads no SpecForge JSON, has no readable jq form, and no `specforge gsd` command answers it.
    (
        "gsd-core/workflows/execute-phase/steps/code-review-disposition.md",
        r#"FIX_REPORT_FILE="${FIX_REPORT_FILE}" node -e ""#,
    ),
    // upstream's runtime library spec-section.cjs (found under `$_GSD_RT`, built with npm): it parses a SPEC.md's
    // sections, which no `specforge` command answers
    (
        "gsd-core/references/specless-probe-fallback.md",
        r#"EDGE_ABSENT=1;   node "$SPEC_SECTION_JS" "$SPEC_FILE" edges       2>/dev/null | grep -q '"supplied":true' && EDGE_ABSENT=0"#,
    ),
    (
        "gsd-core/references/specless-probe-fallback.md",
        r#"PROHIB_ABSENT=1; node "$SPEC_SECTION_JS" "$SPEC_FILE" prohibitions 2>/dev/null | grep -q '"supplied":true' && PROHIB_ABSENT=0"#,
    ),
    // upstream's edge-probe library (found by the candidates loop above it): it derives the edge-coverage answer
    // from the requirements JSON, which no `specforge` command answers
    (
        "gsd-core/references/specless-probe-fallback.md",
        r#"if ! COVERAGE=$(node "$EDGE_PROBE_JS" "$REQS_JSON"); then"#,
    ),
    (
        "gsd-core/workflows/spec-phase.md",
        r#"if ! COVERAGE=$(node "$EDGE_PROBE_JS" "$REQS_JSON"); then"#,
    ),
];

/// The code lines of `lines` (an asset's lines after its header) as `(index, trimmed line)`: the lines inside a fence
/// that are not a `#` comment. Prose about a command is not code.
fn fenced_code(lines: &[String]) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    // the open fence's character: a ``` fence is closed by ```, a ~~~ fence by ~~~
    let mut fence: Option<char> = None;
    for (n, line) in lines.iter().enumerate() {
        let code = line.trim();
        if code.starts_with("```") || code.starts_with("~~~") {
            let mark = code.chars().next();
            fence = match fence {
                None => mark,
                Some(open) if Some(open) == mark => None,
                open => open,
            };
            continue;
        }
        if fence.is_some() && !code.starts_with('#') {
            out.push((n, code));
        }
    }
    out
}

/// Plan 03 (F9): a shell block runs no node: a project that uses SpecForge may have no node at all. Only code counts:
/// a line inside a fence that is not a `#` comment. Prose about a node script is not code.
#[test]
fn a_shell_block_runs_no_node() {
    let mut found = Vec::new();
    let listed: Vec<&(&str, &str)> = KEEP.iter().chain(PENDING_NODE).collect();
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
    for (path, lines, _) in every_asset() {
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
        assert_eq!(
            *count, 1,
            "KEEP/PENDING_NODE ({path}, {line}) matches {count} lines"
        );
    }
}

/// G4 (plan 04 T1, plan 05 T4): no shell block and no inline-code run of the prose runs jq: a SpecForge answer is
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
            if command_word::runs_in_prose(run, "jq") {
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
    assert!(
        hits.len() > 500,
        "only {} hits: the scan finds nothing",
        hits.len()
    );
    specforge_only::assert_pending(
        &hits,
        specforge_only::pending(|f| !f.starts_with("prompt:")),
    );
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
            // `--path`, `--format` and `--pick` belong to the CLI, not to a command
            if matches!(long, "path" | "format" | "pick") {
                i += 2;
                continue;
            }
            match shape.options.iter().find(|(n, _)| n == long) {
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
    let mut files = Vec::new();
    walk(&root().join("assets"), &mut files);
    walk(&root().join("src"), &mut files);
    walk(&root().join("../kit/src"), &mut files);
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
