//! The state store never follows a link, never prunes through one, never trusts an oversized document, and a refused
//! state patch never drops the files a command's plan wrote (ADR 0292, review K9 H2/M2/M7).

use std::os::unix::fs::symlink;
use std::time::{Duration, SystemTime};

use serde_json::json;
use specforge_ops::host::state;
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GsdHosts};
use specforge_protocol_types::StatePatch;
use specforge_test_macros::test as specforge_test;

use crate::host_support::{hosted, hosts_project};

fn project_patch(value: serde_json::Value) -> StatePatch {
    StatePatch {
        project: Some(value),
        session: None,
    }
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "state is never written, read or pruned through a link, and an oversized document is ignored"
)]
fn state_never_follows_a_link() {
    let h = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let view = ProjectView::of(&h.compiled);
    let root = view.root().unwrap().to_path_buf();
    let outside = tempfile::tempdir().unwrap();
    // A linked extension directory: a patch is refused and nothing lands outside.
    let dir = root.join(".specforge/state/t__core");
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    symlink(outside.path(), &dir).unwrap();
    let refused = state::apply(&view, CORE, None, &project_patch(json!({"a": 1})));
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    std::fs::remove_file(&dir).unwrap();

    // A linked `sessions` directory is neither pruned nor followed.
    std::fs::create_dir_all(&dir).unwrap();
    let victim = outside.path().join("old.json");
    std::fs::write(&victim, "{}").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&victim)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(90 * 24 * 3600))
        .unwrap();
    symlink(outside.path(), dir.join("sessions")).unwrap();
    state::apply(&view, CORE, None, &project_patch(json!({"a": 1}))).unwrap();
    assert!(victim.exists(), "pruned through a linked directory");
    std::fs::remove_file(dir.join("sessions")).unwrap();

    // An oversized document on disk reads as empty.
    std::fs::write(
        dir.join("project.json"),
        format!("{{\"blob\":\"{}\"}}", "x".repeat(70 * 1024)),
    )
    .unwrap();
    assert!(state::read(&view, CORE, None).project.is_empty());
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "a refused state patch is reported and the files its command's plan wrote stay written"
)]
fn a_refused_state_patch_keeps_the_plan() {
    use crate::plans::{compiled, project, revision, run_command, set};
    use serde_json::{Value, json};
    use specforge_project::testing::{GsdWrites, gsd_writes_answer};
    use std::sync::Arc;

    let p = project();
    let ops = json!([set(
        "v1.0/03",
        "goal",
        json!("new"),
        &revision(&p, "m.spec")
    )]);
    let mut answer = match gsd_writes_answer(0, ops) {
        specforge_wasm::WasmCallResult::Ok(bytes) => {
            serde_json::from_slice::<Value>(&bytes).unwrap()
        }
        other => panic!("{other:?}"),
    };
    answer["state"] = json!({"project": {"blob": "x".repeat(70 * 1024)}});
    let runtime = Arc::new(
        GsdWrites::new()
            .answering(
                CORE,
                "cmd__record",
                specforge_wasm::WasmCallResult::Ok(answer.to_string().into_bytes()),
            )
            .runtime(),
    );
    let ran = run_command(&p, &compiled(&p, &runtime), &runtime, "core", "record");
    assert_eq!(ran.wrote, ["m.spec"], "the plan was dropped");
    assert!(ran.state_refused, "the refusal was not reported");
}
