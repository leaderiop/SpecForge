//! The shell a ported workflow shows. The port rule checks which lines changed, not whether the new
//! line is valid, so a mechanical `.planning/x` -> "the spec" swap left `rm -f the spec` in a bash fence. For each fenced
//! bash block that differs from upstream's: it still parses where upstream's did (in each bash: `shells`), and none of its code lines carries
//! prose standing in for a path.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use regex::Regex;

#[path = "common/asset_code.rs"]
mod asset_code;
#[path = "common/command_word.rs"]
mod command_word;
#[path = "common/files.rs"]
mod files;
#[path = "common/ledger.rs"]
mod ledger;
#[path = "common/port_map.rs"]
mod port_map;
#[path = "common/specforge_only.rs"]
mod specforge_only;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The files under `dir` with the extension `ext`.
fn with_extension(dir: &Path, ext: &str) -> Vec<PathBuf> {
    files::under(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect()
}

/// An asset's port: its label (the lock key), its body lines, upstream's body lines and its spans (ADR 0332).
struct Pair {
    label: String,
    mine: Vec<String>,
    theirs: Vec<String>,
    spans: Vec<port_map::Span>,
    /// Whether each upstream line is a fence's code line.
    theirs_code: Vec<bool>,
}

impl Pair {
    fn new(
        label: String,
        mine: Vec<String>,
        theirs: Vec<String>,
        spans: Vec<port_map::Span>,
    ) -> Self {
        let mut theirs_code = vec![false; theirs.len()];
        for f in gsd_kit::markdown::fences(&theirs.join("\n")) {
            theirs_code[f.code].fill(true);
        }
        Pair {
            label,
            mine,
            theirs,
            spans,
            theirs_code,
        }
    }

    /// Whether line `n` of ours lies in an own span: text that is not upstream's.
    fn own(&self, n: usize) -> bool {
        self.spans
            .iter()
            .any(|s| matches!(s, port_map::Span::Own { asset, .. } if asset.contains(&n)))
    }

    /// The fence code lines of upstream that ours `s..e` stands for: the upstream range of the spans over it, without
    /// the marks and prose an own span's range also holds.
    fn upstream_of(&self, s: usize, e: usize) -> Vec<&str> {
        let (mut from, mut to) = (usize::MAX, 0);
        for span in &self.spans {
            let (asset, up) = match span {
                port_map::Span::Kept { asset, up, .. } | port_map::Span::Own { asset, up, .. } => {
                    (asset, up)
                }
            };
            if asset.start < e && s < asset.end.max(asset.start + 1) {
                // a kept span maps line for line; an own span stands whole for its upstream range
                let (a, b) = match span {
                    port_map::Span::Kept { .. } => (
                        up.start + s.max(asset.start) - asset.start,
                        up.start + e.min(asset.end) - asset.start,
                    ),
                    port_map::Span::Own { .. } => (up.start, up.end),
                };
                from = from.min(a);
                to = to.max(b);
            }
        }
        (from..to)
            .filter(|&n| self.theirs_code[n])
            .map(|n| self.theirs[n].as_str())
            .collect()
    }
}

/// Every asset of `@gsd/core` but the compact bodies, with its port.
fn pairs() -> Vec<Pair> {
    port_map::assets()
        .into_iter()
        .filter(|a| !a.starts_with("../") && !a.ends_with(".compact.md"))
        .map(|label| {
            let (theirs, _) = port_map::upstream_body(&label);
            let (mine, spans) = (port_map::asset_body(&label), port_map::map(&label));
            Pair::new(label, mine, theirs, spans)
        })
        .collect()
}

/// The bash and sh fences of `lines` as `(first code line, end)` (0-based, end exclusive), as gsd-kit's Markdown
/// reader reads the lines joined with `\n`.
fn fences(lines: &[String]) -> Vec<(usize, usize)> {
    gsd_kit::markdown::fences(&lines.join("\n"))
        .into_iter()
        .filter(|f| matches!(f.language.as_str(), "bash" | "sh"))
        .map(|f| (f.code.start, f.code.end))
        .collect()
}

/// The shells a block must parse in: `bash` on the `PATH`, and `/bin/bash` where it exists (macOS's is bash 3.2, which
/// cannot parse a bare `case` pattern inside `$(…)`; a `(pattern)` form parses in both).
fn shells() -> Vec<&'static str> {
    let mut out = vec!["bash"];
    if Path::new("/bin/bash").exists() {
        out.push("/bin/bash");
    }
    out
}

fn parses(bash: &str, code: &str) -> bool {
    let mut child = Command::new(bash)
        .arg("-n")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("bash");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(code.as_bytes())
        .expect("write");
    child.wait().expect("bash -n").success()
}

/// The bash fences of `pair` that hold an own line, and those of them that no longer parse where upstream's code did.
fn changed_blocks(pair: &Pair) -> (usize, Vec<String>) {
    let (mut checked, mut broken) = (0, Vec::new());
    for (s, e) in fences(&pair.mine) {
        if !(s..e).any(|n| pair.own(n)) {
            continue;
        }
        checked += 1;
        let theirs = pair.upstream_of(s, e).join("\n");
        let mine = pair.mine[s..e].join("\n");
        for bash in shells() {
            if parses(bash, &theirs) && !parses(bash, &mine) {
                broken.push(format!("{}:{} ({bash})", pair.label, s + 2));
            }
        }
    }
    (checked, broken)
}

#[test]
fn a_broken_own_fence_over_a_parsing_one_is_found() {
    let lines = |text: &str| text.split('\n').map(String::from).collect::<Vec<_>>();
    // the own span folds the closing mark and the prose after it in: upstream's side must still be code only
    let theirs = lines("Run it:\n\n```bash\necho one\n```\nThen read the answer.");
    let mine = lines("Run it:\n\n```bash\nif then\n```\nThen read the answer twice.");
    let spans = port_map::spans(&theirs, &mine);
    assert!(
        matches!(&spans[1], port_map::Span::Own { asset, .. } if *asset == (3..6)),
        "{spans:?}"
    );
    let (checked, broken) = changed_blocks(&Pair::new("x.md".into(), mine, theirs, spans));
    assert_eq!((checked, broken.len()), (1, shells().len()), "{broken:?}");
}

#[test]
fn a_changed_bash_block_still_parses() {
    let mut checked = 0;
    let mut broken = Vec::new();
    for pair in pairs() {
        let (c, b) = changed_blocks(&pair);
        checked += c;
        broken.extend(b);
    }
    assert!(checked > 100, "{checked} changed blocks");
    assert!(
        broken.is_empty(),
        "blocks that no longer parse:\n{}",
        broken.join("\n")
    );
}

#[test]
fn no_bash_code_line_carries_prose_for_a_path() {
    let prose = Regex::new(
        r"\bthe spec\b|\bthe pending todos\b|\bthe computed\b|\bthe `\w+` (and `\w+` )?entit(y|ies)\b",
    )
    .expect("a regex");
    let mut found = Vec::new();
    for pair in pairs() {
        let (label, mine) = (&pair.label, &pair.mine);
        for (s, e) in fences(mine) {
            for (n, line) in mine.iter().enumerate().take(e).skip(s) {
                let code = line.trim_start();
                // a `#` or `: #` line is a comment: it may say why there is nothing to run
                if pair.own(n)
                    && !code.starts_with('#')
                    && !code.starts_with(": #")
                    && prose.is_match(code)
                {
                    found.push(format!("{label}:{}: {code}", n + 2));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "prose where a path or command belongs:\n{}",
        found.join("\n")
    );
}

/// `the spec` joined to the word or path after it (`the specquick/`, `the spec.continue-here.md`,
/// `the `phase` entities03-auth`), in prose and in inline code as well as in a fence. The pattern is not `\bthe spec\b`:
/// the join has no word boundary after `spec`.
#[test]
fn no_text_joins_the_spec_to_a_path() {
    let joined = Regex::new(
        r"\bthe spec(?:[A-Za-z]+/|[A-Za-z-]*\.md|\.[A-Za-z])|\)the spec|\bthe spec\]|entit(?:y|ies)[0-9]",
    )
    .expect("a regex");
    let mut found = Vec::new();
    for pair in pairs() {
        for (n, line) in pair.mine.iter().enumerate() {
            if joined.is_match(line) {
                found.push(format!("{}:{}: {line}", pair.label, n + 2));
            }
        }
    }
    // the Rust side writes text too: its string literals
    let mut sources = with_extension(&root().join("src"), "rs");
    sources.extend(with_extension(&root().join("../kit/src"), "rs"));
    for path in sources {
        let text = std::fs::read_to_string(&path).expect("a source");
        for (n, line) in text.lines().enumerate() {
            if joined.is_match(line) {
                found.push(format!("{}:{}: {line}", path.display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "joined words:\n{}", found.join("\n"));
}

/// Text the model wrote into a quoted heredoc is redirected only into a file a `specforge files temp` answer named in
/// the same block (the file, or a path under its directory): the one write ADR 0343 D2 counts as glue.
#[test]
fn a_heredoc_is_redirected_only_into_a_temp_file() {
    let heredoc = Regex::new(r#"read -rd '' (\w+) <<'"#).expect("a regex");
    let redirect = Regex::new(r#"^\s*printf '%s(?:\\n)?' "\$\{?(\w+)\}?" >>? "\$\{?(\w+)\}?"#)
        .expect("a regex");
    let mut checked = 0;
    let mut wrong = Vec::new();
    for path in with_extension(&root().join("assets"), "md") {
        let text = std::fs::read_to_string(&path).expect("an asset");
        let lines: Vec<String> = text.split('\n').map(String::from).collect();
        for (s, e) in fences(&lines) {
            let block = &lines[s..e];
            let read: Vec<&str> = block
                .iter()
                .filter_map(|l| Some(heredoc.captures(l)?.get(1)?.as_str()))
                .collect();
            for (n, line) in block.iter().enumerate() {
                let Some(c) = redirect.captures(line) else {
                    continue;
                };
                if !read.contains(&&c[1]) {
                    continue;
                }
                checked += 1;
                let temp = format!("{}=$(specforge files temp ", &c[2]);
                if !block.iter().any(|l| l.trim_start().starts_with(&temp)) {
                    wrong.push(format!("{}:{}: {line}", path.display(), s + n + 1));
                }
            }
        }
    }
    assert!(
        checked >= 2,
        "specless-probe-fallback and regression-gate redirect a heredoc"
    );
    assert!(
        wrong.is_empty(),
        "a heredoc redirected outside a temp file: {wrong:#?}"
    );
}

/// One wording per situation: an asset quotes its display text one way (ADR 0343 D7's prose quotation marks), a
/// phase placeholder is `{phase_id}` in the researcher and chunked planning, and the review router's note names the
/// declared command D7 gives as its example.
#[test]
fn a_situation_has_one_wording() {
    let backticked = Regex::new(r"[Dd]isplay:? `[^`]").expect("a regex");
    let quoted = Regex::new(r#"[Dd]isplay:? ""#).expect("a regex");
    let mut mixed = Vec::new();
    for path in with_extension(&root().join("assets"), "md") {
        let text = std::fs::read_to_string(&path).expect("an asset");
        if backticked.is_match(&text) && quoted.is_match(&text) {
            mixed.push(path.display().to_string());
        }
    }
    assert!(mixed.is_empty(), "display text quoted two ways: {mixed:#?}");
    let read = |rel: &str| std::fs::read_to_string(root().join("assets").join(rel)).expect(rel);
    // `${PHASE_ID}` in a block is the shell's variable, not a placeholder
    let upper = Regex::new(r"(^|[^$])\{PHASE_ID\}").expect("a regex");
    for rel in [
        "agents/gsd-phase-researcher.md",
        "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
    ] {
        let text = read(rel);
        assert!(
            text.contains("{phase_id}") && !upper.is_match(&text),
            "{rel}"
        );
    }
    let review = read("commands/gsd/ns-review.md");
    // the route table keeps upstream's skill names, which the router reads verbatim
    assert!(
        review.contains("absorbed by `/gsd-code-review --fix`") && !review.contains("flag of"),
        "{review}"
    );
}

/// The graph-context gate (the prompt's `**Graph context:**` line, and when to skip) is written once, in the
/// reference the planner's step reads.
#[test]
fn the_graph_context_gate_lives_in_its_reference() {
    let read = |rel: &str| std::fs::read_to_string(root().join("assets").join(rel)).expect(rel);
    let planner = read("agents/gsd-planner.md");
    let step = planner
        .split_once("<step name=\"load_graph_context\">")
        .and_then(|(_, rest)| rest.split_once("</step>"))
        .expect("the step")
        .0;
    assert!(step.contains("planner-load-graph-context.md"), "{step}");
    assert!(!step.contains("not served on this project"), "{step}");
    let reference = read("gsd-core/references/planner-load-graph-context.md");
    assert!(reference.contains("not served on this project"));
}

/// The decision step's block holds no prose behind `:`: what the model records is a comment.
#[test]
fn the_decision_step_runs_no_prose() {
    let plan = std::fs::read_to_string(root().join("assets/gsd-core/workflows/execute-plan.md"))
        .expect("execute-plan");
    let step = plan
        .split_once("<step name=\"extract_decisions_and_issues\">")
        .and_then(|(_, rest)| rest.split_once("</step>"))
        .expect("the step")
        .0;
    assert!(
        !step.lines().any(|l| l.trim_start().starts_with(": \"")),
        "{step}"
    );
}

/// Every asset fence reads as written to the gsd family's Markdown reader (ADR 0332 D7): each fence is closed, no
/// code line opens with a run of its fence's mark as long as the fence's, and no line opening with three backticks or
/// three tildes lies outside every fence.
#[test]
fn every_fence_is_read_as_written() {
    let mut offences = Vec::new();
    for asset in port_map::assets() {
        let lines = port_map::asset_body(&asset);
        let fences = gsd_kit::markdown::fences(&lines.join("\n"));
        let mut covered = vec![false; lines.len()];
        for f in &fences {
            let end = if f.closed { f.code.end + 1 } else { f.code.end };
            covered[f.open..end.min(lines.len())].fill(true);
            if !f.closed {
                offences.push(format!(
                    "{asset}:{}: not closed (code ends at {})",
                    f.open + 2,
                    f.code.end + 1
                ));
            }
            for n in f.code.clone() {
                let run = lines[n]
                    .trim_start()
                    .chars()
                    .take_while(|c| *c == f.mark)
                    .count();
                if run >= f.width {
                    offences.push(format!(
                        "{asset}:{}: a mark as long as its fence's (opened at {})",
                        n + 2,
                        f.open + 2
                    ));
                }
            }
        }
        // the raw lines are what the reader is checked against
        for (n, line) in lines.iter().enumerate() {
            let t = line.trim_start();
            if ["```", "~~~"].iter().any(|m| t.starts_with(m)) && !covered[n] {
                offences.push(format!("{asset}:{}: a mark outside every fence", n + 2));
            }
        }
    }
    assert!(
        offences.is_empty(),
        "{} fence offences:\n{}",
        offences.len(),
        offences.join("\n")
    );
}
