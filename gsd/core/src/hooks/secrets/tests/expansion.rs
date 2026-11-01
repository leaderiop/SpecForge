//! The command substitutions inside a parameter expansion (F7 review), a divergence from gsd-core: upstream's lexer
//! jumps from `${` to the next `}` and reads nothing inside, so `echo ${x:-$(cat .env)}` is allowed there.

use super::v1_19::bash;
use super::*;

fn denied_as_read(command: &str) -> bool {
    let answer = bash(command);
    answer.decision == Some(Decision::Deny)
        && answer
            .reason
            .unwrap_or_default()
            .contains("would read '.env'")
}

#[test]
fn a_substitution_inside_a_parameter_expansion_is_read() {
    let read = [
        "echo ${x:-$(cat .env)}",
        "echo ${x:-`cat .env`}",
        "echo \"${x:-$(cat .env)}\"",
        "echo ${a:-${b:-$(cat .env)}}",
        "echo ${x:-$(echo })$(cat .env)}",
        "echo ${x:-\"$(cat .env)\"}",
        "cat <<E\n${x:-$(cat .env)}\nE",
    ];
    let missed: Vec<&str> = read.into_iter().filter(|c| !denied_as_read(c)).collect();
    assert!(missed.is_empty(), "allowed: {missed:?}");
    for benign in [
        "echo ${x:-default}",
        "echo ${x:-$(date)} ${HOME}",
        "echo ${x:-'}'}",
    ] {
        assert_eq!(bash(benign).decision, None, "{benign}");
    }
}

/// A `${` inside a `${` is a span: past `MAX_SPAN_DEPTH` (the `$( )` inside counting one) it is too nested.
#[test]
fn nested_parameter_expansions_are_under_the_span_cap() {
    let deep = format!("echo {}x{}", "${a:-".repeat(5000), "}".repeat(5000));
    let reason = bash(&deep).reason.unwrap_or_default();
    assert!(reason.contains("deeper than 128 levels"), "{reason}");
    let at_cap = format!(
        "echo {}$(cat .env){}",
        "${a:-".repeat(shell::MAX_SPAN_DEPTH - 1),
        "}".repeat(shell::MAX_SPAN_DEPTH - 1)
    );
    assert!(denied_as_read(&at_cap));
}
