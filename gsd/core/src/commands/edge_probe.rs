//! `edge_probe_requirements --requirements <json>`: whether the requirements an agent wrote for the edge probe are
//! fit to run it on, the guard `/gsd-spec-phase` and the specless probe fallback run before the probe.
//!
//! A port of the guard's inline node check (`gsd-core/workflows/spec-phase.md`, gsd-core v1.19.0, MIT,
//! `LICENSE-GSD`): the text is a non-empty JSON array, and every item has a string `text` that is not blank
//! (JavaScript's `trim()`) and holds no `<replace:` placeholder. Upstream's node also failed on a `null` item (reading
//! its `text` throws); each failure is named here, where node only exited 1.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, run};
use super::user_story::js_space;

/// The verdict on `requirements`: `{valid, errors, count}`, `count` the items of an array.
pub fn verdict(requirements: &str) -> Value {
    let mut errors = Vec::new();
    let mut count = 0;
    match serde_json::from_str::<Value>(requirements) {
        Err(e) => errors.push(format!("the requirements are not JSON: {e}")),
        Ok(Value::Array(items)) if items.is_empty() => {
            errors.push("the requirements are an empty array".to_string())
        }
        Ok(Value::Array(items)) => {
            count = items.len();
            for (at, item) in items.iter().enumerate() {
                match item.get("text") {
                    Some(Value::String(text)) if text.trim_matches(js_space).is_empty() => {
                        errors.push(format!("requirement {at}: its text is blank"));
                    }
                    Some(Value::String(text)) if text.contains("<replace:") => {
                        errors.push(format!(
                            "requirement {at}: its text still holds the <replace: …> placeholder"
                        ));
                    }
                    Some(Value::String(_)) => {}
                    _ => errors.push(format!("requirement {at}: it has no string text")),
                }
            }
        }
        Ok(_) => errors.push("the requirements are not an array".to_string()),
    }
    json!({ "valid": errors.is_empty(), "errors": errors, "count": count })
}

fn edge_probe_requirements(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    // shortcut: the requirements come as an argument (a command reads no runtime path), so a list over the platform's
    // argument size (128 KiB on Linux) cannot be checked; read the file through the host when one exceeds it
    let verdict = verdict(call.str("requirements").unwrap_or(""));
    let human = match verdict["errors"].as_array() {
        Some(errors) if !errors.is_empty() => errors
            .iter()
            .filter_map(Value::as_str)
            .map(|e| format!("{e}\n"))
            .collect(),
        _ => format!("{} requirement(s), each with a text\n", verdict["count"]),
    };
    Ok(answer(call, verdict, human, None))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("edge_probe_requirements", |cmd| {
        cmd.description("Check the requirements written for the edge probe: a non-empty JSON array whose every item has a string text that is not blank and holds no <replace: …> placeholder; answers valid, the errors and the count")
            .arg("requirements", |a| {
                a.description("the requirements' JSON text, `[{\"id\", \"text\", …}]`");
            })
            .handler(|call| run(call, edge_probe_requirements));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(requirements: &str) -> bool {
        verdict(requirements)["valid"] == true
    }

    #[test]
    fn a_list_of_texts_is_valid() {
        assert_eq!(
            verdict(
                r#"[{"id":"R1","text":"a user signs in","shapes":["auth"]},{"id":"R2","text":"x"}]"#
            ),
            json!({"valid": true, "errors": [], "count": 2})
        );
    }

    #[test]
    fn each_failure_of_upstreams_check_is_refused() {
        for bad in [
            "",
            "not json",
            "[]",
            r#"{"text":"a"}"#,
            r#""a""#,
            "[null]",
            r#"["a"]"#,
            r#"[{"id":"R1"}]"#,
            r#"[{"text":1}]"#,
            r#"[{"text":"  \n\t"}]"#,
            "[{\"text\":\"\u{feff}\u{3000}\"}]",
            r#"[{"text":"<replace: requirement text from the SPEC>"}]"#,
            r#"[{"text":"ok"},{"text":"see <replace: x>"}]"#,
        ] {
            assert!(!valid(bad), "{bad}");
        }
    }

    #[test]
    fn each_bad_item_is_named() {
        assert_eq!(
            verdict(r#"[{"text":"ok"},{"text":" "},{"id":"R3"}]"#)["errors"],
            json!([
                "requirement 1: its text is blank",
                "requirement 2: it has no string text"
            ])
        );
    }
}
