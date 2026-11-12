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

use super::asset_code::{At, assets_under, walk};
use super::command_word;
use super::ledger;

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
    ledger::rows(PENDING_TOOLS, 3)
        .into_iter()
        .map(|row| {
            let [file, tool, line]: [String; 3] = row.try_into().expect("three cells");
            (file, tool, line)
        })
        .filter(|(file, _, _)| keep(file))
        .collect()
}

/// Fails with the hits not listed and the listed rows not hit, each as a row to add or take off.
pub fn assert_pending(hits: &[Row], listed: Vec<Row>) {
    let cells = |(f, t, l): &Row| vec![f.clone(), t.clone(), l.clone()];
    ledger::assert_rows(
        "pending_tools.txt",
        "ask specforge instead",
        &hits.iter().map(cells).collect::<Vec<_>>(),
        listed.iter().map(cells).collect(),
    );
}

/// Every asset of `@gsd/core`, and of `@gsd/security` under `../security/assets/`, as `(path, its lines after the
/// header)`.
pub fn shell_assets() -> Vec<(String, Vec<String>)> {
    let mut out = assets_under("assets");
    out.extend(
        assets_under("../security/assets")
            .into_iter()
            .map(|(path, lines)| (format!("../security/assets/{path}"), lines)),
    );
    out
}

/// The shell fences (`bash`, `sh`, `shell`, `zsh`, `powershell`) of `lines`, each as its lines.
pub fn shell_blocks(lines: &[String]) -> Vec<Vec<&str>> {
    shell_fences(lines)
        .into_iter()
        .map(|(_, block)| block)
        .collect()
}

/// [`shell_blocks`] with each fence's language (`bash`).
pub fn shell_fences(lines: &[String]) -> Vec<(&str, Vec<&str>)> {
    let mut out: Vec<(usize, &str, Vec<&str>)> = Vec::new();
    for (_, line, at) in walk(lines) {
        let At::Fence { n, language } = at else {
            continue;
        };
        if !matches!(language, "bash" | "sh" | "shell" | "zsh" | "powershell") {
            continue;
        }
        match out.last_mut() {
            Some((open, _, block)) if *open == n => block.push(line),
            _ => out.push((n, language, vec![line])),
        }
    }
    out.into_iter()
        .map(|(_, language, block)| (language, block))
        .collect()
}

/// The inline-code runs (`` `…` ``) outside a fence, with the line each starts on, as gsd-kit's Markdown reader reads
/// them: a run may cross lines, and a line break in it reads as a space.
pub fn inline_runs(lines: &[String]) -> Vec<(usize, String)> {
    gsd_kit::markdown::inline_code(&lines.join("\n"))
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
        for tool in command_word::commands_in_prose(&run) {
            add(tool, &run);
        }
    }
    out
}
