//! One reader per open format (ADR 0332 D7): a Markdown table in gsd code is read through
//! `gsd_kit::markdown::tables`, never cut into cells at its pipes by hand.

#[path = "common/files.rs"]
mod files;

use std::path::Path;

/// The forms of a hand split of a table row. Built from pieces, so this file does not match itself.
const NEEDLES: &[&str] = &[
    concat!("split", "('|')"),
    concat!("split", "(\"|\")"),
    concat!("split", "(\" | \")"),
    concat!("split_terminator", "('|')"),
    concat!("rsplit", "('|')"),
    concat!("trim_matches", "('|')"),
];

/// The files that may split at a pipe, each with why.
const EXEMPT: &[(&str, &str)] = &[(
    "core/tests/common/command_word.rs",
    "a shell pipe operator, read until the SDK shell reader gives segment positions",
)];

/// The `path:line` of every line of `text` that holds a needle.
fn hits(path: &str, text: &str) -> Vec<String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| NEEDLES.iter().any(|n| line.contains(n)))
        .map(|(n, _)| format!("{path}:{}", n + 1))
        .collect()
}

#[test]
fn no_gsd_code_splits_a_markdown_table_row_by_hand() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    let mut exempt_hit = vec![false; EXEMPT.len()];
    let mut scanned = 0;
    for dir in ["kit", "core", "security"] {
        for path in files::under(&root.join(dir)) {
            let relative = path
                .strip_prefix(&root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/");
            if path.extension().is_none_or(|e| e != "rs") || relative.contains("/target/") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{relative}: {e}"));
            scanned += 1;
            let hits = hits(&relative, &text);
            match EXEMPT.iter().position(|(p, _)| *p == relative) {
                Some(k) => exempt_hit[k] = !hits.is_empty(),
                None => found.extend(hits),
            }
        }
    }
    assert!(scanned > 100, "{scanned} Rust files scanned");
    assert!(
        found.is_empty(),
        "lines that split a row at a pipe by hand (read the table through gsd_kit::markdown::tables):\n{}",
        found.join("\n")
    );
    for ((path, why), hit) in EXEMPT.iter().zip(exempt_hit) {
        assert!(
            hit,
            "{path} no longer splits at a pipe ({why}): take off its exemption"
        );
    }
}

#[test]
fn the_needles_match_every_hand_split_and_nothing_else() {
    for needle in NEEDLES {
        let sample = format!("let cells: Vec<&str> = line.{needle}.collect();");
        assert_eq!(hits("x.rs", &sample), ["x.rs:1"], "{needle}");
    }
    assert!(hits("x.rs", "let cells = line.split('\\t');").is_empty());
}
