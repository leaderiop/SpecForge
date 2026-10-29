//! A ported line names only tools that exist (REVIEW-r40 M6): every `specforge <verb>` it adds is a verb of the CLI,
//! every `specforge gsd <command>` is a declared command, and no agent body keeps a blind rename.

use std::path::{Path, PathBuf};

use regex::Regex;

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
    let base = root().join("../parity/upstream/v1.18.0");
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
    "check", "collect", "format", "gsd", "install", "new", "query", "schema", "set", "settings",
];

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
            // a shell comment is prose
            if mine == theirs || mine.trim_start().starts_with('#') {
                continue;
            }
            for cap in verb.captures_iter(mine) {
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
                        name.is_empty() || commands.iter().any(|c| c == name),
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
