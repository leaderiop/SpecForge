//! `user_story_validate --story <text>`: whether a phase goal is a canonical user story ("As a [role], I want to
//! [capability], so that [outcome]."), the check `/gsd-mvp-phase`, the verifier and the MVP UAT framing run.
//!
//! A port of gsd-tools' `user-story validate` (gsd-core v1.19.0, MIT, `LICENSE-GSD`): the story is trimmed,
//! the per-clause guards give their targeted errors first (case-insensitive, as upstream's), and only when they all
//! pass does the canonical pattern run, which is case-sensitive, needs non-whitespace at the start of each slot and
//! one line (JavaScript's `.` stops at `\n`, `\r`, U+2028 and U+2029). The answer is flat (`role`, `capability`,
//! `outcome`), not upstream's `slots` object. The patterns are matched by hand, as the component links no regex engine.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, run};

const FORMAT: &str = "\"As a [role], I want to [capability], so that [outcome].\"";

/// JavaScript's `\s`, which `trim()` also strips: Rust's whitespace holds U+0085 and lacks U+FEFF.
fn js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// Whether `s` opens with a character that is not `\s`.
fn opens_solid(s: &str) -> bool {
    s.chars().next().is_some_and(|c| !js_space(c))
}

/// The byte offsets just past each `marker` in `s`, in order: ASCII case folded when `fold` (JavaScript's `/i`
/// without `u` maps no other character onto an ASCII one).
fn after(s: &str, marker: &str, fold: bool) -> Vec<usize> {
    let (s, m) = (s.as_bytes(), marker.as_bytes());
    (0..s.len().saturating_sub(m.len() - 1))
        .filter(|&i| {
            let w = &s[i..i + m.len()];
            if fold {
                w.eq_ignore_ascii_case(m)
            } else {
                w == m
            }
        })
        .map(|i| i + m.len())
        .collect()
}

/// A guard `/<marker>\S/i`: some `marker` is followed by a character that is not `\s`.
fn guarded(s: &str, marker: &str) -> bool {
    after(s, marker, true)
        .into_iter()
        .any(|i| opens_solid(&s[i..]))
}

/// `/^As a (\S.*?), I want to (\S.*?), so that (\S.*?)\.$/` with its backtracking: the leftmost role, then the
/// leftmost capability, that leave a well-formed rest.
fn canonical(s: &str) -> Option<(&str, &str, &str)> {
    const WANT: &str = ", I want to ";
    const SO: &str = ", so that ";
    if s.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    let rest = s.strip_prefix("As a ").filter(|r| opens_solid(r))?;
    for c in after(rest, WANT, false)
        .into_iter()
        .filter(|&c| c > WANT.len())
    {
        let (role, tail) = (&rest[..c - WANT.len()], &rest[c..]);
        if !opens_solid(tail) {
            continue;
        }
        for o in after(tail, SO, false).into_iter().filter(|&o| o > SO.len()) {
            let (capability, outcome) = (&tail[..o - SO.len()], &tail[o..]);
            if let Some(outcome) = outcome.strip_suffix('.').filter(|o| opens_solid(o)) {
                return Some((role, capability, outcome));
            }
        }
    }
    None
}

/// The verdict on `story`: `{valid, errors, role, capability, outcome}`, the slots `null` unless it is valid.
pub fn validate(story: &str) -> Value {
    let trimmed = story.trim_matches(js_space);
    let mut errors = Vec::new();
    let mut slots = None;
    if trimmed.is_empty() {
        errors.push(format!("Story is empty. Required format: {FORMAT}"));
    } else {
        if !trimmed
            .as_bytes()
            .get(..5)
            .is_some_and(|b| b.eq_ignore_ascii_case(b"as a "))
            || !opens_solid(&trimmed[5..])
        {
            errors.push(
                "Story must start with \"As a [user role],\" (role must be non-empty).".to_string(),
            );
        }
        if !guarded(trimmed, ", i want to ") {
            errors.push(
                "Story must include \", I want to [capability],\" (capability must be non-empty)."
                    .to_string(),
            );
        }
        if !guarded(trimmed, ", so that ") {
            errors.push(
                "Story must include \", so that [outcome].\" (outcome must be non-empty)."
                    .to_string(),
            );
        }
        if !trimmed.ends_with('.') {
            errors.push("Story must end with a period (.).".to_string());
        }
        if errors.is_empty() {
            match canonical(trimmed) {
                Some(found) => slots = Some(found),
                None => errors.push(format!(
                    "Story does not match the canonical format: {FORMAT}"
                )),
            }
        }
    }
    let (role, capability, outcome) = match slots {
        Some((r, c, o)) => (json!(r), json!(c), json!(o)),
        None => (Value::Null, Value::Null, Value::Null),
    };
    json!({
        "valid": errors.is_empty(),
        "errors": errors,
        "role": role,
        "capability": capability,
        "outcome": outcome,
    })
}

fn user_story_validate(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    // an absent story is an empty one, as upstream reads it: a verdict, not a refusal
    let verdict = validate(call.str("story").unwrap_or(""));
    let human = match verdict["errors"].as_array() {
        Some(errors) if !errors.is_empty() => errors
            .iter()
            .filter_map(Value::as_str)
            .map(|e| format!("{e}\n"))
            .collect(),
        _ => "valid\n".to_string(),
    };
    Ok(answer(call, verdict, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("user_story_validate", |cmd| {
        cmd.description("Check a phase goal against the canonical user story: \"As a [role], I want to [capability], so that [outcome].\" on one line; answers valid, the errors and the three slots")
            .arg("story", |a| {
                a.description("the story text, usually the phase's goal");
            })
            .handler(|call| run(call, user_story_validate));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errors(story: &str) -> Vec<String> {
        validate(story)["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn a_canonical_story_is_valid_and_its_slots_are_read() {
        let verdict = validate("  As a user, I want to add tasks, so that I remember them.\n");
        assert_eq!(
            verdict,
            json!({"valid": true, "errors": [], "role": "user", "capability": "add tasks", "outcome": "I remember them"})
        );
        // the slots are lazy: the first ", I want to " and ", so that " split them
        let verdict = validate("As a user, admin, I want to x, I want to y, so that a, so that b.");
        assert_eq!(verdict["role"], "user, admin");
        assert_eq!(verdict["capability"], "x, I want to y");
        assert_eq!(verdict["outcome"], "a, so that b");
    }

    #[test]
    fn a_slot_that_opens_blank_moves_the_split_on_as_the_pattern_backtracks() {
        // JavaScript gives these slots: the first split leaves a blank capability or outcome, the next one does not
        let slots = |s: &str| {
            let v = validate(s);
            (
                v["role"].clone(),
                v["capability"].clone(),
                v["outcome"].clone(),
            )
        };
        assert_eq!(
            slots("As a u, I want to  , I want to x, so that y."),
            (json!("u, I want to  "), json!("x"), json!("y"))
        );
        assert_eq!(
            slots("As a u, I want to x, so that  , so that z."),
            (json!("u"), json!("x, so that  "), json!("z"))
        );
    }

    #[test]
    fn whitespace_is_javascripts() {
        // U+0085 is Rust whitespace, not JavaScript's: trim() keeps it, so the story does not start with "As a"
        assert_eq!(
            errors("\u{85}As a u, I want to x, so that y."),
            ["Story must start with \"As a [user role],\" (role must be non-empty)."]
        );
        assert_eq!(
            validate("\u{feff}As a u, I want to x, so that y.\u{3000}")["valid"],
            true
        );
    }

    #[test]
    fn an_empty_story_has_one_error() {
        for story in ["", "   ", "\n\t", "\u{feff}"] {
            assert_eq!(
                errors(story),
                [format!("Story is empty. Required format: {FORMAT}")],
                "{story:?}"
            );
            assert_eq!(validate(story)["valid"], false);
        }
    }

    #[test]
    fn each_missing_clause_has_its_own_error() {
        assert_eq!(
            errors("I want to x"),
            [
                "Story must start with \"As a [user role],\" (role must be non-empty).",
                "Story must include \", I want to [capability],\" (capability must be non-empty).",
                "Story must include \", so that [outcome].\" (outcome must be non-empty).",
                "Story must end with a period (.).",
            ]
        );
        assert_eq!(
            errors("As a user, I want to x, so that y"),
            ["Story must end with a period (.)."]
        );
    }

    #[test]
    fn a_whitespace_slot_fails() {
        // the guards see `\S` after each marker, so a blank slot fails one of them
        assert_eq!(
            errors("As a  , I want to x, so that y."),
            ["Story must start with \"As a [user role],\" (role must be non-empty)."]
        );
        assert_eq!(
            errors("As a user, I want to  , so that y."),
            ["Story must include \", I want to [capability],\" (capability must be non-empty)."]
        );
        assert_eq!(
            errors("As a user, I want to x, so that  ."),
            ["Story must include \", so that [outcome].\" (outcome must be non-empty)."]
        );
    }

    #[test]
    fn a_story_on_two_lines_or_in_another_case_is_not_canonical() {
        let canonical = format!("Story does not match the canonical format: {FORMAT}");
        for story in [
            "As a user\nadmin, I want to x, so that y.",
            "As a user, I want to x\ny, so that z.",
            "As a user, I want to x, so that y\n z.",
            "As a user, I want to x, so that y\u{2028}z.",
            "as a user, I want to x, so that y.",
            "As a user, i want to x, so that y.",
        ] {
            assert_eq!(errors(story), std::slice::from_ref(&canonical), "{story:?}");
            assert_eq!(validate(story)["role"], Value::Null);
        }
    }
}
