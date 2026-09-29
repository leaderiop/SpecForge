//! The formatter against the repository's own specs: formatting every
//! corpus (in a copy) leaves the entity graph and the diagnostics exactly
//! as they were, keeps every comment, and is idempotent. This gates
//! reformatting the corpus, and stays as a regression test.

use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Each corpus is checked by this path from the repository root; the root
/// `specforge.json` is the project config of `spec` and
/// `integrations/rust/spec`.
const CORPORA: &[&str] = &[
    "spec",
    "examples/todo-app",
    "examples/shop",
    "integrations/rust/spec",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn specforge(cwd: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_specforge"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().filter_map(Result::ok) {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            if entry.file_name() != "target" && entry.file_name() != "tests" {
                copy_tree(&path, &target);
            }
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

fn spec_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            spec_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "spec") {
            out.push(path);
        }
    }
}

/// Drop source positions: formatting moves lines, and nothing else may change.
fn without_positions(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for key in [
                "line",
                "start_line",
                "end_line",
                "start_col",
                "end_col",
                "column",
                "span",
            ] {
                map.remove(key);
            }
            map.values_mut().for_each(without_positions);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(without_positions),
        _ => {}
    }
}

/// `// ...` comments outside string literals, with `//text` read as
/// `// text` (the one rewrite the formatter makes).
fn comments(source: &str) -> BTreeMap<String, usize> {
    let mut found = BTreeMap::new();
    let bytes = source.as_bytes();
    let (mut i, mut in_string, mut in_triple) = (0, false, false);
    while i < bytes.len() {
        if in_triple {
            if bytes[i..].starts_with(b"\"\"\"") {
                in_triple = false;
                i += 3;
            } else {
                i += 1;
            }
        } else if in_string {
            match bytes[i] {
                b'\\' => i += 2,
                b'"' => {
                    in_string = false;
                    i += 1;
                }
                _ => i += 1,
            }
        } else if bytes[i..].starts_with(b"\"\"\"") {
            in_triple = true;
            i += 3;
        } else if bytes[i] == b'"' {
            in_string = true;
            i += 1;
        } else if bytes[i..].starts_with(b"//") {
            let end = bytes[i..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |n| i + n);
            let text = source[i..end].trim_end();
            let text = match text.strip_prefix("//") {
                Some(rest)
                    if rest.starts_with(|c: char| !c.is_whitespace() && c != '/' && c != '!') =>
                {
                    format!("// {rest}")
                }
                _ => text.to_string(),
            };
            *found.entry(text).or_insert(0) += 1;
            i = end;
        } else {
            i += 1;
        }
    }
    found
}

#[specforge_test(
    invariant = "formatting_semantic_preservation",
    verify = "format(spec) parses to an identical entity graph as spec"
)]
#[specforge_test(
    invariant = "formatting_semantic_preservation",
    verify = "formatting does not alter entity IDs, field values, or reference lists"
)]
#[specforge_test(
    invariant = "formatting_idempotency",
    verify = "formatting an already-formatted file produces identical output"
)]
#[specforge_test(
    invariant = "comment_preservation",
    verify = "every comment in input appears in formatted output"
)]
fn formatting_the_corpus_changes_nothing_but_layout() {
    let root = repo_root();
    let copy = tempfile::tempdir().unwrap();
    std::fs::copy(
        root.join("specforge.json"),
        copy.path().join("specforge.json"),
    )
    .unwrap();
    for corpus in CORPORA {
        copy_tree(&root.join(corpus), &copy.path().join(corpus));
    }

    for corpus in CORPORA {
        let mut originals = Vec::new();
        spec_files(&root.join(corpus), &mut originals);
        let before: BTreeMap<PathBuf, String> = originals
            .iter()
            .map(|p| {
                (
                    p.strip_prefix(&root).unwrap().to_path_buf(),
                    std::fs::read_to_string(p).unwrap(),
                )
            })
            .collect();

        specforge(copy.path(), &["format", corpus]);

        let mut problems = Vec::new();
        for (relative, original) in &before {
            let formatted = std::fs::read_to_string(copy.path().join(relative)).unwrap();
            if comments(original) != comments(&formatted) {
                problems.push(format!("{}: comments changed", relative.display()));
            }
        }
        let check = Command::new(env!("CARGO_BIN_EXE_specforge"))
            .args(["format", "--check", corpus])
            .current_dir(copy.path())
            .output()
            .unwrap();
        if !check.status.success() {
            problems.push(format!(
                "{corpus}: not idempotent: {}",
                String::from_utf8_lossy(&check.stdout)
            ));
        }
        assert!(problems.is_empty(), "{problems:#?}");

        for args in [
            &["export", "--format", "graph", corpus][..],
            &["check", "--format", "json", corpus][..],
        ] {
            let mut original: serde_json::Value =
                serde_json::from_str(&specforge(&root, args)).unwrap();
            let mut formatted: serde_json::Value =
                serde_json::from_str(&specforge(copy.path(), args)).unwrap();
            without_positions(&mut original);
            without_positions(&mut formatted);
            assert!(
                original == formatted,
                "`specforge {}` differs after formatting {corpus}",
                args.join(" ")
            );
        }
    }
}
