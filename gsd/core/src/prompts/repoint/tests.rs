use super::repoint;

const EMBEDDED: &str = "references/verification-patterns.md";
const LATER_WORKFLOW: &str = "workflows/ingest-docs.md";

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
        format!(
            "`specforge gsd reference gsd-core/{LATER_WORKFLOW}` (not installed by @gsd/core yet)"
        )
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
