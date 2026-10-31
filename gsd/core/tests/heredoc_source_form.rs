//! #5637: the render renames a capture heredoc's terminator only in the form it recognises, the quoted opener `<<'T'`
//! and an unindented closing line `T` (`gsd_kit::prompt::render`). Upstream's "the shipped sources open these heredocs
//! only in the one quoted form the stamp recognizes", over @gsd/core's and @gsd/security's assets.

use std::path::{Path, PathBuf};

use regex::Regex;

const TOKENS: [&str; 2] = ["GSD_ARGS_END", "GSD_QUICK_BATCH_ARGS_END"];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

#[test]
fn the_assets_open_and_close_the_capture_heredocs_only_in_the_stamped_form() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["assets", "../security/assets"] {
        let dir = root.join(dir);
        if dir.is_dir() {
            walk(&dir, &mut files);
        }
    }
    let opener =
        Regex::new(r#"<<[-~]?\s*["']?(GSD_(?:QUICK_BATCH_)?ARGS_END)\b"#).expect("a regex");
    let (mut opened, mut closed) = (0, 0);
    for file in &files {
        let text = std::fs::read_to_string(file).expect("an asset");
        for (i, line) in text.lines().enumerate() {
            let at = format!("{}:{}", file.display(), i + 1);
            if let Some(token) = opener.captures(line).map(|c| c[1].to_string()) {
                assert!(
                    line.contains(&format!("<<'{token}'")),
                    "{at} opens {token} in a form the render does not stamp"
                );
                opened += 1;
            }
            if TOKENS.contains(&line.trim()) {
                assert!(
                    TOKENS.contains(&line),
                    "{at} closes with an indented or padded line"
                );
                closed += 1;
            }
        }
    }
    assert!(opened >= 60, "vacuous: only {opened} openers");
    assert_eq!(opened, closed, "every opener has its closing line");
}
