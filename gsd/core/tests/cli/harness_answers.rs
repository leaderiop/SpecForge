//! The harness's check of every answer a test reads against `tests/common/answers.rs`.

#[path = "../common/answers.rs"]
pub mod answers;

/// Holds `tests/common/answers.rs` to the answer a test reads: a whole JSON answer of a listed `specforge gsd
/// <command>` prints only members its row lists (`answer_members` holds the other way). A writing command's JSON is
/// the host's envelope (`output`, `plan`, and `files_written` once applied), whose `output` is the answer a `--pick`
/// reads. A `--pick` of a path other than `.` may print an inner object, so it is not the answer.
pub(super) fn answers_listed(args: &[&str], stdout: &str) {
    let ["gsd", command, rest @ ..] = args else {
        return;
    };
    let Some(row) = answers::members(command) else {
        return;
    };
    let inner = rest
        .iter()
        .enumerate()
        .any(|(i, a)| match a.strip_prefix("--pick") {
            Some("") => rest.get(i + 1) != Some(&"."),
            Some(value) => value != "=.",
            None => false,
        });
    if inner {
        return;
    }
    let Ok(serde_json::Value::Object(mut answer)) = serde_json::from_str(stdout) else {
        return;
    };
    if answer.contains_key("plan")
        && let Some(serde_json::Value::Object(output)) = answer.remove("output")
    {
        answer = output;
    }
    for key in answer.keys() {
        assert!(
            row.contains(&key.as_str()),
            "{command} answered '{key}', which tests/common/answers.rs does not list"
        );
    }
}
