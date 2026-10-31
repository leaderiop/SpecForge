//! The commands core's ported text asks of a capability extension are the ones `@gsd/security` declares
//! (REVIEW-security-part1 N5): core's own tests cannot see this declaration, and the text's `|| echo` fallbacks would
//! hide a rename (the threat-ID hard stop would silently never fire).

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
fn every_command_core_asks_of_an_extension_is_declared_by_security() {
    let core: Vec<String> = gsd_core::build()
        .declaration()
        .surfaces
        .commands
        .iter()
        .map(|c| c.id.replace('_', "-"))
        .collect();
    let security: Vec<String> = gsd_security::build()
        .declaration()
        .surfaces
        .commands
        .iter()
        .map(|c| c.id.replace('_', "-"))
        .collect();
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
            if !core.contains(&cap[1].to_string()) {
                asked.insert(cap[1].to_string());
            }
        }
    }
    // the hook line builds `"$c-hooks"` for each capability; security is the one that exists
    assert!(
        security.contains(&"security-hooks".to_string()),
        "{security:?}"
    );
    for name in ["security-gate", "threat-ids"] {
        assert!(
            asked.contains(name),
            "core's text no longer asks for {name}: {asked:?}"
        );
    }
    for name in &asked {
        assert!(
            security.contains(name),
            "core's text asks for `specforge gsd {name}`, which neither extension declares"
        );
    }
}
