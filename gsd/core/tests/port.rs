//! The port against the vendored upstream files (gsd-core v1.19.0): every `@gsd/core` and `@gsd/security` asset is
//! mapped span by span, and `port.lock` is the map (ADR 0332); the agents' frontmatter and the commands their bodies
//! name.

use std::path::{Path, PathBuf};

use gsd_core::agents::roster::AGENTS;
use regex::Regex;

#[path = "common/asset_code.rs"]
mod asset_code;
#[path = "common/command_word.rs"]
mod command_word;
#[path = "common/files.rs"]
mod files;
#[path = "common/port_map.rs"]
mod port_map;
#[path = "common/specforge_only.rs"]
mod specforge_only;

pub fn upstream_dir() -> PathBuf {
    PathBuf::from(gsd_kit::snapshot!("agents"))
}

pub fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/agents")
}

fn ported() -> Regex {
    Regex::new(r"\.planning/|gsd-tools|gsd_run|gsd-sdk").expect("a regex")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// An upstream file as `(frontmatter lines, body lines)`.
fn upstream(file: &str) -> (Vec<String>, Vec<String>) {
    let text = read(&upstream_dir().join(file));
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    assert_eq!(lines[0], "---", "{file} has no frontmatter");
    let end = lines[1..]
        .iter()
        .position(|l| l == "---")
        .expect("a closing ---")
        + 1;
    (lines[1..end].to_vec(), lines[end + 1..].to_vec())
}

#[test]
fn no_asset_names_planning_or_gsd_tools() {
    let rx = Regex::new(port_map::PORTED).expect("a regex");
    let mut seen = 0;
    for asset in port_map::assets() {
        seen += 1;
        for (n, line) in port_map::asset_body(&asset).iter().enumerate() {
            assert!(!rx.is_match(line), "{asset}:{}: {line}", n + 2);
        }
    }
    assert!(seen >= 300, "{seen} assets");
}

/// Every asset is mapped span by span against its upstream copy, and `port.lock` is that map (ADR 0332).
#[test]
fn the_port_lock_maps_every_asset() {
    port_map::check_lock();
}

/// No span waits for its re-port. Ignored only between a snapshot move and its last re-port, never on main.
#[test]
fn no_span_waits_for_resync() {
    let waiting: Vec<String> = port_map::lock_rows()
        .into_iter()
        .filter(|r| r.group == "resync")
        .map(|r| format!("{}:{}+{}", r.asset, r.up, r.n))
        .collect();
    assert!(waiting.is_empty(), "spans to re-port: {waiting:#?}");
}

#[test]
fn a_span_map_tiles_both_bodies() {
    use port_map::Span;
    let up = ["a", "b", "", "```", "c", "d"];
    let own = ["a", "B", "", "```", "C", "d"];
    let spans = port_map::spans(&up, &own);
    let ranges: Vec<_> = spans
        .iter()
        .map(|s| match s {
            Span::Kept { asset, up, .. } => (true, up.clone(), asset.clone()),
            Span::Own { asset, up, .. } => (false, up.clone(), asset.clone()),
        })
        .collect();
    // the blank and fence-mark lines between two own spans fold into one
    assert_eq!(
        ranges,
        [(true, 0..1, 0..1), (false, 1..5, 1..5), (true, 5..6, 5..6)]
    );
    let up = ["x", "y", "z", "q"];
    let own = ["x", "z", "w", "q", "v"];
    let spans = port_map::spans(&up, &own);
    let (mut u, mut a) = (0, 0);
    for span in &spans {
        let (su, sa) = match span {
            Span::Kept {
                asset, up: range, ..
            } => {
                assert_eq!(up[range.clone()], own[asset.clone()], "{span:?}");
                (range.clone(), asset.clone())
            }
            Span::Own {
                asset, up: range, ..
            } => (range.clone(), asset.clone()),
        };
        assert_eq!(
            (su.start, sa.start),
            (u, a),
            "spans tile both sides in order"
        );
        (u, a) = (su.end, sa.end);
    }
    assert_eq!((u, a), (up.len(), own.len()));
    assert_eq!(
        spans
            .iter()
            .filter(|s| matches!(s, Span::Own { .. }))
            .count(),
        3
    );
}

#[test]
fn words_ignore_blank_lines_indentation_and_fence_width() {
    assert_eq!(
        port_map::words(&["  ````bash", "", "   x  ", "~~~~", "\t", "``a``"]),
        "```bash\nx\n~~~\n``a``"
    );
    assert_eq!(port_map::words(&["", "  "]), "");
}

#[test]
fn the_computed_groups_judge_both_sides() {
    use port_map::computed;
    // a ported name wins over a served call
    assert_eq!(
        computed(&["X=$(gsd-tools init x)"], &["X=$(specforge gsd init x)"]),
        Some("ported-name")
    );
    assert_eq!(
        computed(
            &["jq -r .phase \"$INIT\""],
            &["specforge gsd init --pick .phase"]
        ),
        Some("served")
    );
    // a program upstream's span does not run is not served
    assert_eq!(computed(&["jq -r .a f"], &["specforge x | sed 1d"]), None);
    assert_eq!(
        computed(&["```", "  x", ""], &["````", "x", "````"][..2]),
        Some("markdown")
    );
    assert_eq!(computed(&["Read the file."], &["Read the entity."]), None);
}

#[test]
fn a_bless_carries_a_reviewed_group_and_marks_a_new_span() {
    use port_map::{Bless, Row, carry};
    let row = |sha_own: &str, group: &str| Row {
        asset: "gsd-core/workflows/x.md".to_string(),
        kept: false,
        up: 3,
        n: 1,
        m: 1,
        sha_up: "aaaaaaaaaaaa".to_string(),
        sha_own: sha_own.to_string(),
        group: group.to_string(),
    };
    let old = [row("bbbbbbbbbbbb", "relic"), row("cccccccccccc", "jq")];
    let new = vec![
        row("bbbbbbbbbbbb", ""),
        row("bbbbbbbbbbbb", ""),
        row("dddddddddddd", "markdown"),
    ];
    let groups: Vec<String> = carry(&old, new, Bless::Review)
        .into_iter()
        .map(|r| r.group)
        .collect();
    // one old row carries once; a computed group stays
    assert_eq!(groups, ["relic", "unreviewed", "markdown"]);
    // after an upstream move, a span no old row carries waits for its re-port
    let new = vec![row("bbbbbbbbbbbb", ""), row("eeeeeeeeeeee", "")];
    let groups: Vec<String> = carry(&old, new, Bless::Resync)
        .into_iter()
        .map(|r| r.group)
        .collect();
    assert_eq!(groups, ["relic", "resync"]);
}

#[test]
fn the_lock_round_trips() {
    let lock = std::fs::read_to_string(port_map::lock_path()).expect("port.lock");
    assert!(lock.starts_with(&port_map::header()));
    let rows = port_map::rows(&lock);
    assert!(rows.iter().any(|r| r.kept) && rows.iter().any(|r| !r.kept));
    assert_eq!(port_map::rows(&port_map::render(&rows)), rows);
    assert_eq!(port_map::render(&rows), lock);
}

fn class(tool: &str) -> String {
    match tool {
        "Read" => "read",
        "Write" => "write",
        "Edit" => "edit",
        "Bash" => "shell",
        "Glob" | "Grep" => "search",
        "WebFetch" => "web_fetch",
        "WebSearch" => "web_search",
        "Skill" => "skill",
        "AskUserQuestion" => "ask_user",
        "Agent" => "agent",
        mcp => {
            let server = mcp
                .strip_prefix("mcp__")
                .and_then(|m| m.strip_suffix("__*"))
                .unwrap_or_else(|| panic!("unmapped tool {mcp}"));
            return format!("mcp:{server}/*");
        }
    }
    .to_string()
}

/// The tools of an upstream frontmatter: a comma list or a YAML list.
fn upstream_tools(front: &[String]) -> Vec<String> {
    let at = front
        .iter()
        .position(|l| l.starts_with("tools:"))
        .expect("tools");
    let rest = front[at]["tools:".len()..].trim();
    let names: Vec<String> = if rest.is_empty() {
        front[at + 1..]
            .iter()
            .map_while(|l| l.trim().strip_prefix("- ").map(|t| t.trim().to_string()))
            .collect()
    } else {
        rest.split(',').map(|t| t.trim().to_string()).collect()
    };
    let mut classes = Vec::new();
    for name in names {
        let c = class(&name);
        if !classes.contains(&c) {
            classes.push(c);
        }
    }
    classes
}

#[test]
fn tools_are_upstreams() {
    assert_eq!(AGENTS.len(), 15);
    let rx = ported();
    for agent in AGENTS {
        let (front, _) = upstream(&format!("{}.md", agent.id));
        let expected = upstream_tools(&front);
        assert!(!expected.is_empty(), "{}", agent.id);
        assert_eq!(agent.tools.to_vec(), expected, "{}", agent.id);
        let field = |key: &str| {
            front
                .iter()
                .find_map(|l| l.strip_prefix(key))
                .expect("a frontmatter key")
                .trim()
                .to_string()
        };
        assert_eq!(agent.color, field("color:"), "{}", agent.id);
        let theirs = field("description:");
        if rx.is_match(&theirs) {
            assert!(!rx.is_match(agent.description), "{}", agent.id);
        } else {
            assert_eq!(agent.description, theirs, "{}", agent.id);
        }
    }
}

/// The bodies that name only commands that exist: the agents' bodies the port rewrote first, and the workflows' as
/// each rewrites its own.
const RUNNABLE: [&str; 21] = [
    "gsd-executor.md",
    "gsd-verifier.md",
    "gsd-plan-checker.md",
    "gsd-phase-researcher.md",
    "gsd-integration-checker.md",
    "gsd-integration-checker.compact.md",
    "gsd-roadmapper.md",
    "gsd-roadmapper.compact.md",
    "gsd-project-researcher.md",
    "gsd-project-researcher.compact.md",
    "gsd-research-synthesizer.md",
    "gsd-research-synthesizer.compact.md",
    "gsd-planner.md",
    "gsd-pattern-mapper.md",
    "gsd-pattern-mapper.compact.md",
    "gsd-assumptions-analyzer.md",
    "gsd-assumptions-analyzer.compact.md",
    "gsd-doc-synthesizer.md",
    "gsd-doc-synthesizer.compact.md",
    "gsd-doc-classifier.md",
    "gsd-doc-classifier.compact.md",
];

#[test]
fn every_command_a_body_names_is_declared() {
    let declaration = gsd_core::build().declaration();
    let names: Vec<String> = declaration
        .surfaces
        .commands
        .iter()
        .map(|c| c.id.replace('_', "-"))
        .collect();
    let rx = Regex::new(r"specforge gsd ([a-z][a-z0-9-]*)").expect("a regex");
    let mut seen = 0;
    for file in RUNNABLE {
        for (n, line) in read(&assets_dir().join(file)).lines().enumerate() {
            for cap in rx.captures_iter(line) {
                seen += 1;
                assert!(
                    names.contains(&cap[1].to_string()),
                    "{file}:{}: `specforge gsd {}` is not a declared command",
                    n + 1,
                    &cap[1]
                );
            }
        }
    }
    assert!(seen >= 5, "only {seen} commands named");
}

/// A body's text with the quoted strings blanked: the commands' words and flags are left, a `|`, `)` or `;` inside a
/// quote does not end an invocation.
fn unquoted(line: &str) -> String {
    let quoted = Regex::new(r#""[^"]*"|'[^']*'"#).expect("a regex");
    quoted.replace_all(line, "X").into_owned()
}

/// Every `specforge <verb> ...` invocation of a body as the words after `specforge`, up to the end of the command
/// (a pipe, a closing quote or parenthesis, a comment, `;` or `&&`). A line ending in `\` goes on in the next.
fn invocations(file: &str, verb: &str) -> Vec<(usize, Vec<String>)> {
    let text = read(&assets_dir().join(file));
    let mut joined: Vec<(usize, String)> = Vec::new();
    let mut continued = false;
    for (n, line) in text.lines().enumerate() {
        match joined.last_mut().filter(|_| continued) {
            Some((_, last)) => {
                last.push(' ');
                last.push_str(line.trim_end_matches('\\'));
            }
            None => joined.push((n + 1, line.trim_end_matches('\\').to_string())),
        }
        continued = line.ends_with('\\');
    }
    let rx = Regex::new(&format!(r"specforge {verb}\b(?:\s+([^`|);&#]*))?")).expect("a regex");
    joined
        .into_iter()
        .flat_map(|(n, line)| {
            let line = unquoted(&line);
            rx.captures_iter(&line)
                .map(|c| {
                    let words = c.get(1).map_or("", |m| m.as_str());
                    (n, words.split_whitespace().map(String::from).collect())
                })
                .collect::<Vec<(usize, Vec<String>)>>()
        })
        .collect()
}

/// The `--flag` names among `words`, with `_` as `-` and without `=value`.
fn flags(words: &[String]) -> Vec<String> {
    words
        .iter()
        .filter_map(|w| w.strip_prefix("--"))
        .map(|w| w.split('=').next().unwrap_or(w).replace('_', "-"))
        .collect()
}

/// What every `specforge` command takes besides its own arguments.
const GLOBAL_FLAGS: [&str; 7] = ["format", "path", "dry-run", "help", "yes", "pick", "fields"];

#[test]
fn every_gsd_invocation_a_body_gives_uses_flags_the_command_declares() {
    let declaration = gsd_core::build().declaration();
    let mut seen = 0;
    for file in RUNNABLE {
        for (n, words) in invocations(file, "gsd") {
            let Some(name) = words.first() else { continue };
            let Some(command) = declaration
                .surfaces
                .commands
                .iter()
                .find(|c| c.id.replace('_', "-") == *name)
            else {
                continue; // `every_command_a_body_names_is_declared` names it
            };
            seen += 1;
            let declared: Vec<String> = command
                .args
                .iter()
                .map(|a| a.name.replace('_', "-"))
                .collect();
            for flag in flags(&words) {
                assert!(
                    declared.contains(&flag) || GLOBAL_FLAGS.contains(&flag.as_str()),
                    "{file}:{n}: `specforge gsd {name}` has no --{flag} (it takes {declared:?})"
                );
            }
        }
    }
    assert!(seen >= 10, "only {seen} invocations checked");
}

/// The kinds of the extension: name -> (field -> (required, writer)).
fn kinds() -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, (bool, String)>>
{
    let host = specforge_extension_sdk::testing::MockHost::new(gsd_core::build());
    let described: serde_json::Value =
        serde_json::from_str(&host.describe_json("entities").expect("entities")).expect("JSON");
    described["items"]
        .as_array()
        .expect("kinds")
        .iter()
        .map(|k| {
            let fields = k["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .map(|f| {
                    (
                        f["name"].as_str().unwrap_or_default().to_string(),
                        (
                            f["required"] == true,
                            f["writer"].as_str().unwrap_or("user").to_string(),
                        ),
                    )
                })
                .collect();
            (k["name"].as_str().unwrap_or_default().to_string(), fields)
        })
        .collect()
}

#[test]
fn every_specforge_new_and_set_a_body_gives_fits_the_kind() {
    let kinds = kinds();
    let mut complete = 0;
    for file in RUNNABLE {
        for (n, words) in invocations(file, "new") {
            let Some(kind) = words.first() else { continue };
            // `@gsd/security`'s kinds, which the planner names only when that extension is loaded.
            if ["trust_boundary", "threat"].contains(&kind.as_str()) {
                continue;
            }
            let fields = kinds
                .get(kind)
                .unwrap_or_else(|| panic!("{file}:{n}: `specforge new {kind}`: no such kind"));
            assert!(
                !words.iter().any(|w| w.contains('=') && !w.starts_with('-')),
                "{file}:{n}: `specforge new {kind}` takes `--field value`, not `field=value`: {words:?}"
            );
            let given = flags(&words);
            for flag in &given {
                let field = fields.get(&flag.replace('-', "_"));
                assert!(
                    [
                        "in", "verify", "revision", "dry-run", "format", "path", "help"
                    ]
                    .contains(&flag.as_str())
                        || field.is_some_and(|(_, writer)| writer == "user"),
                    "{file}:{n}: `specforge new {kind}` has no writable field --{flag}"
                );
            }
            // `kind id "title" ...` is a whole example; fewer words only mention the command.
            if words.len() >= 3 {
                complete += 1;
                assert!(
                    given.contains(&"in".to_string()),
                    "{file}:{n}: `specforge new {kind}` needs --in <file>"
                );
                for (name, (required, writer)) in fields {
                    assert!(
                        !required || writer != "user" || given.contains(&name.replace('_', "-")),
                        "{file}:{n}: `specforge new {kind}` needs the required --{name}"
                    );
                }
            }
        }
        for (n, words) in invocations(file, "set") {
            let (Some(_), Some(field)) = (words.first(), words.get(1)) else {
                continue;
            };
            assert!(
                kinds
                    .values()
                    .any(|f| f.get(field).is_some_and(|(_, w)| w == "user")),
                "{file}:{n}: `specforge set` names no user-written field '{field}'"
            );
        }
        for (n, words) in invocations(file, "settings set") {
            assert!(
                words.first().is_some_and(|e| e.starts_with('@')),
                "{file}:{n}: `specforge settings set` takes the extension first (`@gsd/core`): {words:?}"
            );
        }
        for (n, words) in invocations(file, "query") {
            let op = words.first().map_or("", String::as_str);
            assert!(
                !(op.contains('.') && !op.contains('/')) && !op.contains("legitimacy"),
                "{file}:{n}: `specforge query {op}` is an upstream operation, not a query"
            );
        }
    }
    assert!(
        complete >= 3,
        "only {complete} whole `specforge new` examples"
    );
}

/// The agents' threat-model lines apply when `@gsd/security` is loaded, and are off when it is absent.
#[test]
fn threat_model_lines_name_the_security_kinds() {
    let when = "when `@gsd/security` is loaded and `workflow.security_enforcement` is on";
    for file in ["gsd-planner.md", "gsd-phase-researcher.md"] {
        let text = read(&assets_dir().join(file));
        let mut seen = 0;
        for line in text.lines().filter(|l| l.contains("security_enforcement")) {
            assert!(line.contains(when), "{file}: {line}");
            assert!(!line.contains("absent = enabled"), "{file}: {line}");
            seen += 1;
        }
        assert!(seen >= 2, "{file}: only {seen} security_enforcement lines");
    }
    let planner = read(&assets_dir().join("gsd-planner.md"));
    let lines: Vec<&str> = planner.lines().collect();
    let close = lines
        .iter()
        .position(|l| *l == "</threat_model>")
        .expect("the template");
    let named = lines[close + 1];
    for word in ["`trust_boundary`", "`threat`", "`specforge new threat"] {
        assert!(named.contains(word), "{word} is not in: {named}");
    }
    let preamble = read(&assets_dir().join("../../../kit/src/prompt/preamble.rs"));
    assert!(preamble.contains("An agent this host does not have installed"));
    assert!(!preamble.contains("`gsd-security-auditor`"));
}
