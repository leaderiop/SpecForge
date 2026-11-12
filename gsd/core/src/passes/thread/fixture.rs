//! Test facts in a pass input, as `@code/core` and the host hand them to a pass: shared by the unit tests of every
//! pass that reads the thread.

use specforge_extension_sdk::prelude::*;

/// Pushes onto `input` the `@code/core` test fact `id` (its `name`, `file` and `status`) and, for each of `names`,
/// the edge labelled `names` from it to that entity: what the host sends for a test whose name tags them.
pub(crate) fn add_test(
    input: &mut PassInput,
    id: &str,
    name: &str,
    file: &str,
    status: TestStatus,
    names: &[&str],
) {
    input.entities.push(
        serde_json::from_value(serde_json::json!({
            "id": id, "kind": "test",
            "fields": { "name": name, "file": file, "status": status.as_str() },
        }))
        .expect("a test fact"),
    );
    for target in names {
        input.edges.push(PassEdge {
            source: id.to_string(),
            target: target.to_string(),
            label: super::NAMES.to_string(),
        });
    }
}
