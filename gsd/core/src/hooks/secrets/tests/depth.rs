//! The span reader's depth cap (F7), a divergence from gsd-core: upstream's reader recurses without one, and a stack
//! overflow lands in its fail-open catch. Here a command nested past `MAX_SPAN_DEPTH` spans is denied as
//! `command-too-nested`, and one at the cap is judged as before.

use super::v1_19::bash;
use super::*;
use shell::MAX_SPAN_DEPTH;

/// A `$(` in an unquoted heredoc tag word inside a span (`paren_close` → `read_heredoc_tag` → `paren_close`).
const TAG: &str = "$(cat <<";
/// A `$(` in double quotes inside a span (`paren_close` → `paren_close`).
const QUOTE: &str = "$(\"";

/// `lead; : "` and `n` spans of `shape` (both shapes in turn for `None`), closed by one `)"`.
fn nested(lead: &str, shape: Option<&str>, n: usize) -> String {
    let spans: String = (0..n)
        .map(|k| shape.unwrap_or(if k % 2 == 0 { TAG } else { QUOTE }))
        .collect();
    format!("{lead}; : \"{spans})\"")
}

/// What the guard says of `command`: `too-nested`, `read .env`, `allow`, or the reason of another denial.
fn said(command: &str) -> String {
    let answer = bash(command);
    let reason = answer.reason.unwrap_or_default();
    if answer.decision != Some(Decision::Deny) {
        "allow".into()
    } else if reason.contains(&format!("deeper than {MAX_SPAN_DEPTH} levels")) {
        "too-nested".into()
    } else if reason.contains("would read '.env'") {
        "read .env".into()
    } else {
        reason
    }
}

#[test]
fn a_command_at_the_cap_is_judged_and_one_past_it_is_too_nested() {
    let mut differ = Vec::new();
    for shape in [Some(TAG), Some(QUOTE), None] {
        for (lead, at_cap) in [("cat .env", "read .env"), ("echo hi", "allow")] {
            for (n, want) in [
                (MAX_SPAN_DEPTH, at_cap),
                (MAX_SPAN_DEPTH + 1, "too-nested"),
                (5000, "too-nested"),
            ] {
                let got = said(&nested(lead, shape, n));
                if got != want {
                    differ.push(format!("{lead} {shape:?} x {n}: {got}, want {want}"));
                }
            }
        }
    }
    assert!(differ.is_empty(), "{}", differ.join("\n"));
}

/// The nested scripts the scan reads again (an `eval`'s, a `$( )` body's) are under the same cap.
#[test]
fn a_nested_script_is_under_the_same_cap() {
    let inner = nested("echo hi", Some(QUOTE), MAX_SPAN_DEPTH + 1);
    assert_eq!(said(&format!("eval '{inner}'")), "too-nested");
    assert_eq!(
        said(&format!(
            "eval '{}'",
            nested("cat .env", Some(QUOTE), MAX_SPAN_DEPTH)
        )),
        "read .env"
    );
}
