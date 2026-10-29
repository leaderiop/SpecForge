use super::repoint;

const EMBEDDED: &str = "references/verification-patterns.md";
const LATER_WORKFLOW: &str = "workflows/update.md";

#[test]
fn an_at_line_becomes_the_command_that_serves_the_file() {
    let line = format!("@~/.claude/gsd-core/{EMBEDDED}");
    assert_eq!(
        repoint(&line),
        format!("`specforge gsd reference gsd-core/{EMBEDDED}`")
    );
}

#[test]
fn a_backticked_path_keeps_one_pair_of_backticks() {
    let line = format!("Read `~/.claude/gsd-core/{EMBEDDED}` first.");
    assert_eq!(
        repoint(&line),
        format!("Read `specforge gsd reference gsd-core/{EMBEDDED}` first.")
    );
}

#[test]
fn a_home_path_and_a_sentence_end_are_read_apart() {
    let line = format!("See $HOME/.claude/gsd-core/{EMBEDDED}.");
    assert_eq!(
        repoint(&line),
        format!("See `specforge gsd reference gsd-core/{EMBEDDED}`.")
    );
}

#[test]
fn a_path_no_workflow_embeds_says_so() {
    let line = format!("@~/.claude/gsd-core/{LATER_WORKFLOW}");
    assert_eq!(
        repoint(&line),
        format!("`specforge gsd reference gsd-core/{LATER_WORKFLOW}` (not part of @gsd/core)")
    );
}

#[test]
fn a_line_with_two_paths_rewrites_both() {
    let line = format!("@~/.claude/gsd-core/{EMBEDDED} and ~/.claude/gsd-core/{LATER_WORKFLOW}");
    let out = repoint(&line);
    assert!(!out.contains(".claude"), "{out}");
    assert_eq!(out.matches("specforge gsd reference").count(), 2, "{out}");
}

#[test]
fn text_without_a_path_is_unchanged() {
    let text = "nothing to point at, not even .claude/ alone";
    assert_eq!(repoint(text), text);
}

#[test]
fn a_path_in_a_double_quoted_string_gets_no_backticks() {
    let out = repoint(r#"for d in "$HOME/.claude/gsd-core/bin/lib" "x"; do"#);
    assert_eq!(
        out,
        r#"for d in "specforge gsd reference gsd-core/bin/lib" "x"; do"#
    );
}

#[test]
fn the_users_own_profile_file_keeps_its_path() {
    let line = r#"P="$HOME/.claude/gsd-core/USER-PROFILE.md""#;
    assert_eq!(repoint(line), line);
}
