//! The line tables of the line-for-line rule, read as the families of the port lock's reviewed groups. Each own span
//! of the lock holds the family that lists the most of its body lines; ties go in [`ORDER`]. While the line counts
//! are still equal, a span's upstream and asset body lines share one numbering.

use super::port_map::{self, COMPUTED, Span};
use super::{agent_rewrites, rule, thread_rewrites};

#[path = "../common/answer_rewrites.rs"]
mod answer_rewrites;
#[path = "../common/args_rewrites.rs"]
mod args_rewrites;
#[path = "../common/files_rewrites.rs"]
mod files_rewrites;
#[path = "../common/git_rewrites.rs"]
mod git_rewrites;
#[path = "../common/jq_rewrites.rs"]
mod jq_rewrites;
#[path = "../common/prose_rewrites.rs"]
mod prose_rewrites;
#[path = "../common/quick_rewrites.rs"]
mod quick_rewrites;
#[allow(
    dead_code,
    reason = "the port rule's blanked-run assertion is not read here"
)]
#[path = "../common/relic_rewrites.rs"]
mod relic_rewrites;
#[path = "../common/review_rewrites.rs"]
mod review_rewrites;
#[path = "../common/wait_rewrites.rs"]
mod wait_rewrites;
#[path = "../common/workflow_rewrites.rs"]
mod workflow_rewrites;

/// The families in tie order.
const ORDER: [&str; 13] = [
    "relic", "thread", "quick", "args", "jq", "git", "files", "answer", "wait", "review", "prose",
    "agent", "workflow",
];

/// `@gsd/security`'s `REWRITTEN` body lines of `gsd-core/workflows/secure-phase.md`, with the group each row's comment
/// names.
const SECURITY: &[(usize, &str)] = &[
    (1, "relic"),
    (51, "relic"),
    (52, "relic"),
    (53, "relic"),
    (56, "relic"),
    (57, "relic"),
    (64, "relic"),
    (68, "relic"),
    (74, "relic"),
    (108, "relic"),
    (164, "relic"),
    (98, "answer"),
    (144, "answer"),
    (146, "answer"),
    (150, "answer"),
    (154, "answer"),
    (155, "answer"),
    (173, "git"),
    (18, "args"),
    (19, "args"),
    (21, "args"),
    (24, "args"),
];

/// Whether `family` lists body line `n` of the upstream path `path`, whose upstream body line is `theirs`.
fn lists(family: &str, path: &str, n: usize, theirs: Option<&str>) -> bool {
    let spans = |s: &[(&str, usize, usize)]| rule::in_spans(s, path, n);
    let security = SECURITY
        .iter()
        .any(|(l, f)| path == "gsd-core/workflows/secure-phase.md" && *l == n && *f == family);
    security
        || match family {
            "relic" => {
                relic_rewrites::LINES.contains(&(path, n))
                    || spans(relic_rewrites::SPANS)
                    || theirs.is_some_and(|t| relic_rewrites::blanked(path, n, t))
            }
            "thread" => spans(thread_rewrites::SPANS),
            "quick" => spans(quick_rewrites::SPANS),
            "args" => spans(args_rewrites::SPANS),
            "jq" => jq_rewrites::LINES.contains(&(path, n)) || spans(jq_rewrites::SPANS),
            "git" => spans(git_rewrites::SPANS),
            "files" => spans(files_rewrites::SPANS),
            "answer" => spans(answer_rewrites::SPANS),
            "wait" => spans(wait_rewrites::SPANS),
            "review" => review_rewrites::LINES.contains(&(path, n)),
            "prose" => spans(prose_rewrites::SPANS),
            "agent" => path.strip_prefix("agents/").is_some_and(|file| {
                agent_rewrites::ROWS.contains(&(file, n))
                    || agent_rewrites::REWRITTEN.contains(&(file, n))
            }),
            "workflow" => {
                workflow_rewrites::REWRITTEN.contains(&(path, n))
                    || spans(workflow_rewrites::REWRITTEN_SPANS)
            }
            _ => false,
        }
}

/// The family listing the most of `lines` (body lines of `path`), if any lists one.
fn family(path: &str, lines: &[usize], up: &[String]) -> Option<&'static str> {
    let count = |f: &str| {
        lines
            .iter()
            .filter(|&&n| lists(f, path, n, up.get(n).map(String::as_str)))
            .count()
    };
    let (best, most) = ORDER
        .iter()
        .map(|f| (*f, count(f)))
        .fold(("", 0), |b, c| if c.1 > b.1 { c } else { b });
    (most > 0).then_some(best)
}

/// Checks (or, under `GSD_BLESS=seed`, writes) every own row's reviewed group against the tables.
pub fn agree() {
    let seed = std::env::var("GSD_BLESS").as_deref() == Ok("seed");
    let mut lock = port_map::lock_rows();
    let mut at = 0;
    let (mut unlisted, mut wrong) = (Vec::new(), Vec::new());
    for asset in port_map::assets() {
        let path = port_map::upstream_path(&asset).to_string();
        let (up, _) = port_map::upstream_body(&path);
        for span in port_map::spans(&up, &port_map::asset_body(&asset)) {
            let row = &mut lock[at];
            at += 1;
            let Span::Own {
                asset: a, up: u, ..
            } = span
            else {
                continue;
            };
            if COMPUTED.contains(&row.group.as_str()) {
                continue;
            }
            let mut lines: Vec<usize> = u.clone().chain(a.clone()).collect();
            lines.sort_unstable();
            lines.dedup();
            let name = format!(
                "{asset}:{}+{} (asset body {}..{})",
                row.up, row.n, a.start, a.end
            );
            match family(&path, &lines, &up) {
                Some(f) if seed && row.group == "unreviewed" => row.group = f.to_string(),
                Some(f) if row.group != f && row.group != "unreviewed" => {
                    wrong.push(format!("{name}: `{}`, the tables say `{f}`", row.group));
                }
                None if row.group == "unreviewed" => unlisted.push(name),
                _ => {}
            }
        }
    }
    assert_eq!(at, lock.len(), "the lock is the span map");
    if seed {
        std::fs::write(port_map::lock_path(), port_map::render(&lock)).expect("write port.lock");
    }
    assert!(
        wrong.is_empty(),
        "groups the tables disagree with:\n{}",
        wrong.join("\n")
    );
    assert!(
        unlisted.is_empty(),
        "own spans no table lists (review each by hand):\n{}",
        unlisted.join("\n")
    );
}
