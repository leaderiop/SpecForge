//! G4 (plan 05 T3): the quick batch's list is tab-separated lines (`quick_id<TAB>description`), its merge-eligible
//! prefix a bash array and its verification route a `case`: no jq on the `PATH`.

use super::{Shell, asset, line, q};

#[test]
fn a_task_list_becomes_one_line_per_quick_task() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let tmp = tempfile::tempdir().expect("a dir");
    let text = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let build = line(&text, "QB_ITEMS=$(");
    let take = [
        line(&text, "BATCH_ID="),
        line(&text, "BATCH_MANIFEST_JSON="),
        line(&text, "ITEM_COUNT="),
    ]
    .join("\n");
    let run = |list: &str| {
        std::fs::write(tmp.path().join("tasks.md"), list).expect("a list");
        sh.run_in(
            tmp.path(),
            &format!(
                "TASK_FILE={}\n{build}\nQB_CREATE_RC=$?\n{take}\n\
                 printf '%s|%s|%s\\n' \"$QB_CREATE_RC\" \"$ITEM_COUNT\" \"$BATCH_ID\"; printf '%s\\n' \"$BATCH_MANIFEST_JSON\"",
                q(&tmp.path().join("tasks.md").display().to_string())
            ),
        )
    };
    let (out, err, _) = run("- Add a task\n\n2. Remove a task\n* List tasks\n");
    let lines: Vec<&str> = out.lines().collect();
    let head: Vec<&str> = lines[0].split('|').collect();
    assert_eq!(&head[..2], ["0", "3"], "{out}{err}");
    assert!(
        head[2].len() == 12 && head[2].ends_with("-batch"),
        "{}",
        head[2]
    );
    let items: Vec<(&str, &str)> = lines[1..]
        .iter()
        .map(|l| l.split_once('\t').expect("a tab"))
        .collect();
    let descriptions: Vec<&str> = items.iter().map(|(_, d)| *d).collect();
    assert_eq!(descriptions, ["Add a task", "Remove a task", "List tasks"]);
    for (id, _) in &items {
        let (day, tail) = id.split_once('-').expect("<yymmdd>-<3 base-36>");
        assert!(
            day.len() == 6 && day.bytes().all(|b| b.is_ascii_digit()),
            "{id}"
        );
        assert!(
            tail.len() == 3
                && tail
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
            "{id}"
        );
    }
    // one item is no batch: the create step refuses
    let (out, err, _) = run("- Only this\n");
    assert!(out.starts_with('1'), "{out}{err}");
}

#[test]
fn the_merge_eligible_items_are_the_ready_prefix_of_the_wave() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/quick-batch/steps/merge-wave.md");
    let eligible = line(&text, "mergeable=()");
    for (ready, want) in [
        ("a\nb\nc", "[a][b][c]"),
        ("a\nc", "[a]"),
        ("b\nc", ""),
        ("", ""),
    ] {
        let (out, err, _) = sh.run_in(
            sh.bin.path(),
            &format!(
                "WAVE_ORDER={}\nREADY={}\n{eligible}\nfor q in \"${{mergeable[@]}}\"; do printf '[%s]' \"$q\"; done",
                q("a\nb\nc"),
                q(ready)
            ),
        );
        assert_eq!(out, want, "{ready:?}: {err}");
    }
}

#[test]
fn a_verification_status_routes_the_batch_item() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/quick-batch/steps/verification-wave.md");
    let route = line(&text, "case \"$STATUS\" in");
    for (status, want) in [
        ("passed", "complete"),
        ("human_needed", "human_needed"),
        ("gaps_found", "failed"),
        ("", "failed"),
    ] {
        let out = sh.run(&format!(
            "STATUS={}\n{route}\nprintf '%s' \"$QB_VERIFY_ACTION\"",
            q(status)
        ));
        assert_eq!(out, want, "{status}");
    }
}
