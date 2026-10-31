//! The commands core's ported text asks of a capability extension are ones a project loading `@gsd/core` and
//! `@gsd/security` answers (REVIEW-security-part1 N5): core's own tests cannot see security's declaration, and the text's
//! `|| echo` fallbacks would hide a rename (the threat-ID hard stop would silently never fire). Black-box through the
//! binary: a test that links both guests gets two `call` exports, which Linux's linker refuses.

use regex::Regex;

fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

#[test]
fn every_command_core_asks_of_an_extension_is_answered() {
    let rx = Regex::new(r"specforge gsd (\w[\w-]*)").expect("a regex");
    let mut all = Vec::new();
    files(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/assets"),
        &mut all,
    );
    let mut asked = std::collections::BTreeSet::new();
    for path in all {
        let text = std::fs::read_to_string(&path).expect("an asset");
        for cap in rx.captures_iter(&text) {
            asked.insert(cap[1].to_string());
        }
    }
    for name in ["security-gate", "threat-ids"] {
        assert!(
            asked.contains(name),
            "core's text no longer asks for {name}: {asked:?}"
        );
    }
    let p = super::both();
    let unknown = p.run(&["gsd", "no-such-command", "--help"]);
    assert_ne!(unknown.code, 0, "the control is refused: {}", unknown.all());
    for name in &asked {
        let run = p.run(&["gsd", name, "--help"]);
        assert_eq!(
            run.code,
            0,
            "core's text asks for `specforge gsd {name}`, which neither extension answers: {}",
            run.all()
        );
    }
}
