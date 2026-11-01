//! R0 of "SpecForge serves itself" (`.planning/serves-itself-2026-11-01`): a block calls only `specforge`, so a user
//! installs nothing else. The scan finds every command a markdown text runs: the command words of its shell fences, and
//! the runs of its inline code that read as runs (`super::command_word`). What is not `specforge`, the shell's own word
//! or a function a block defines is a hit, and every hit must be a row of `pending_tools.txt`, the programme's
//! worklist: a row with no hit fails, so the list only shrinks, and a hit with no row fails, so no new tool comes in.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

use regex::Regex;

use super::command_word;

/// What a block may run besides `specforge`: the shell's own keywords and builtins.
pub const SHELL_WORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do", "done",
    "in", "function", "break", "continue", "read", "printf", "echo", "test", "local", "export",
    "set", "unset", "shift", "return", "exit", "true", "false", "cd", "pwd", "shopt", "setopt",
    "let", "declare", "typeset", "readonly", "trap", "wait", "eval", "source", "getopts",
];

/// A hit or a row: `(file, tool, the line or inline run that runs it)`.
pub type Row = (String, String, String);

/// The worklist, as `<file>\t<tool>\t<line>` rows under `#` headings, one per building block.
pub const PENDING_TOOLS: &str = include_str!("pending_tools.txt");

/// The rows of [`PENDING_TOOLS`] whose file `keep` takes.
pub fn pending(keep: impl Fn(&str) -> bool) -> Vec<Row> {
    PENDING_TOOLS
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut cells = l.splitn(3, '\t').map(String::from);
            let mut cell = || {
                cells
                    .next()
                    .unwrap_or_else(|| panic!("a row of three cells: {l}"))
            };
            (cell(), cell(), cell())
        })
        .filter(|(file, _, _)| keep(file))
        .collect()
}

/// Fails with the hits not listed and the listed rows not hit, each as a row to add or take off.
pub fn assert_pending(hits: &[Row], listed: Vec<Row>) {
    let mut left = listed;
    let mut new = Vec::new();
    for hit in hits {
        match left.iter().position(|row| row == hit) {
            Some(k) => {
                left.swap_remove(k);
            }
            None => new.push(hit),
        }
    }
    let row = |(f, t, l): &Row| format!("{f}\t{t}\t{l}");
    assert!(
        new.is_empty() && left.is_empty(),
        "{} hits not in pending_tools.txt (ask specforge instead):\n{}\n{} rows no longer hit (take them off):\n{}",
        new.len(),
        new.iter().map(|h| row(h)).collect::<Vec<_>>().join("\n"),
        left.len(),
        left.iter().map(row).collect::<Vec<_>>().join("\n")
    );
}

/// Every asset of `@gsd/core`, and of `@gsd/security` under `../security/assets/`, as `(path, its lines after the
/// header)`.
pub fn shell_assets() -> Vec<(String, Vec<String>)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    for (dir, label) in [
        ("assets", ""),
        ("../security/assets", "../security/assets/"),
    ] {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let mut paths = Vec::new();
        walk(&base, &mut paths);
        paths.sort();
        for p in paths
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
        {
            let relative = p.strip_prefix(&base).expect("under its base");
            let text = std::fs::read_to_string(p).expect("an asset");
            let lines = text.split('\n').skip(1).map(String::from).collect();
            out.push((format!("{label}{}", relative.to_string_lossy()), lines));
        }
    }
    out
}

/// The shell fences (`bash`, `sh`, `shell`, `zsh`, `powershell`) of `lines`, each as its lines.
pub fn shell_blocks(lines: &[String]) -> Vec<Vec<&str>> {
    let mut out = Vec::new();
    let mut fence: Option<(char, bool)> = None;
    for line in lines {
        let code = line.trim();
        if code.starts_with("```") || code.starts_with("~~~") {
            let mark = code.chars().next().unwrap_or('`');
            fence = match fence {
                None => {
                    let shell = matches!(
                        code[3..].trim(),
                        "bash" | "sh" | "shell" | "zsh" | "powershell"
                    );
                    if shell {
                        out.push(Vec::new());
                    }
                    Some((mark, shell))
                }
                Some((open, _)) if open == mark => None,
                open => open,
            };
            continue;
        }
        if fence.is_some_and(|(_, shell)| shell) {
            out.last_mut().expect("an open block").push(line.as_str());
        }
    }
    out
}

/// The inline-code runs (`` `…` ``) of the lines outside a fence, with their line index.
pub fn inline_runs(lines: &[String]) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fenced = false;
    for (n, line) in lines.iter().enumerate() {
        let text = line.trim();
        if text.starts_with("```") || text.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if !fenced {
            out.extend(text.split('`').skip(1).step_by(2).map(|run| (n, run)));
        }
    }
    out
}

/// The functions the shell fences of `texts` define (`name() {`, `function name() {`): the shell's own words too.
pub fn functions<'a>(texts: impl IntoIterator<Item = &'a [String]>) -> Vec<String> {
    let define = Regex::new(r"(?:^|[\s;&|{(])(?:function\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\(\)\s*\{")
        .expect("a regex");
    texts
        .into_iter()
        .flat_map(|lines| shell_blocks(lines).into_iter().flatten())
        .flat_map(|line| {
            define
                .captures_iter(line)
                .map(|c| c[1].to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The hits of the markdown `lines` of `file`: each command its shell fences and inline runs run that is not
/// `specforge`, a [`SHELL_WORDS`] word or one of `functions`.
pub fn hits(file: &str, lines: &[String], functions: &[String]) -> Vec<Row> {
    let mut out = Vec::new();
    let mut add = |tool: String, text: &str| {
        if tool != "specforge"
            && !SHELL_WORDS.contains(&tool.as_str())
            && !functions.contains(&tool)
        {
            out.push((file.to_string(), tool, text.to_string()));
        }
    };
    for block in shell_blocks(lines) {
        for (n, tool) in command_word::block_commands(&block.join("\n")) {
            add(tool, block[n].trim());
        }
    }
    for (_, run) in inline_runs(lines) {
        for tool in command_word::commands_in_prose(run) {
            add(tool, run);
        }
    }
    out
}
