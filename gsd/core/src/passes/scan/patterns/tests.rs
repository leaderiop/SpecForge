use super::*;

fn severities(text: &str) -> Vec<(usize, Severity)> {
    anti_patterns(text)
        .into_iter()
        .map(|h| (h.line, h.severity))
        .collect()
}

#[test]
fn a_bare_marker_is_a_blocker_and_a_referenced_one_is_not() {
    let text =
        "a\n// FIXME later\n// TBD see #42\n// XXX tracked in DEF-12\n// XXX\nlet x; // TODO\n";
    assert_eq!(
        severities(text),
        [
            (2, Severity::Blocker),
            (5, Severity::Blocker),
            (6, Severity::Warning)
        ]
    );
}

#[test]
fn a_marker_is_a_whole_word() {
    assert_eq!(severities("size XXXL\nFIXMEs\nTODO_X\n"), []);
}

#[test]
fn a_placeholder_phrase_is_info() {
    assert_eq!(
        severities("coming soon\nNot Yet Implemented\n"),
        [(1, Severity::Info), (2, Severity::Info)]
    );
}

#[test]
fn exports_match_the_three_forms() {
    assert!(exports("export function add() {}", "add"));
    assert!(exports("export { a, add }", "add"));
    assert!(exports("module.exports.add = add", "add"));
    assert!(exports("exports.add = add", "add"));
    assert!(!exports("export function addAll() {}", "add"));
    assert!(!exports("function add() {}", "add"));
    assert!(!exports("exports.addAll = 1", "add"));
}

#[test]
fn only_a_plain_pattern_is_literal() {
    assert!(is_literal("require"));
    assert!(!is_literal("import.*store"));
}
