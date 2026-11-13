//! The quick batch's list is rows of one `quick-init` answer (`quick_id`, `slug`, `description`), its merge-eligible
//! prefix a bash array and its verification route a `case`: no jq on the `PATH`.

use super::{Shell, asset, block, fence_after, fence_from, line, q};
use crate::commands_support::{is_quick_id, rows, twin};

#[test]
fn a_task_list_becomes_one_line_per_quick_task() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = twin();
    let text = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let lines = [
        line(&text, "QB_TASKS=$INLINE_TASK_LIST"),
        line(&text, "QB_CREATE_LIST="),
        line(&text, "IFS=$'\\x1f' read -r _ _ _ BATCH_ID"),
        line(&text, "BATCH_MANIFEST_JSON="),
    ]
    .join("\n");
    let run = |list: &str| {
        sh.run_in(
            p.root(),
            &format!(
                "INLINE_TASK_LIST={}\n{lines}\n\
                 printf '%s\\t%s\\t%s\\n' \"$QB_CREATE_RC\" \"$ITEM_COUNT\" \"$BATCH_ID\"; printf '%s\\n' \"$BATCH_MANIFEST_JSON\"",
                q(list)
            ),
        )
    };
    let (out, err, _) = run("- Add a task\n\n2. Remove a task\n* List tasks\n");
    let lines: Vec<&str> = out.lines().collect();
    let head: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(&head[..2], ["0", "3"], "{out}{err}");
    assert!(
        head[2].len() == 12 && head[2].ends_with("-batch"),
        "{}",
        head[2]
    );
    let items = rows(out.split_once('\n').expect("a head line").1);
    let descriptions: Vec<&str> = items.iter().map(|row| row[2]).collect();
    assert_eq!(descriptions, ["Add a task", "Remove a task", "List tasks"]);
    let slugs: Vec<&str> = items.iter().map(|row| row[1]).collect();
    assert_eq!(slugs, ["add-a-task", "remove-a-task", "list-tasks"]);
    for row in &items {
        assert!(is_quick_id(row[0]), "{}", row[0]);
    }
    // one item is no batch: the create step refuses, saying why, and the read after it stops
    let (out, err, code) = run("- Only this\n");
    assert_eq!((out.as_str(), code), ("", 1), "{err}");
    assert!(err.contains("a batch needs at least 2 tasks"), "{err}");
}

/// Fences are separate shells on some hosts, so a plan is written in the fence whose answer minted its id: its slug
/// has no fence to get lost in. Another shell then finds it by that slug, and a slug read empty writes nothing.
#[test]
fn a_quick_plan_is_written_in_the_fence_that_mints_its_id() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = twin();
    let text = asset("gsd-core/workflows/quick.md");
    let init = line(&text, "IFS=$'\\x1f' read -r quick_id slug")
        .replace("{{setting:git.quick_branch_template|}}", "");
    let (out, err, code) = sh.run_in(p.root(), &format!("DESCRIPTION='Trim titles'\n{init}"));
    assert_eq!(code, 0, "{out}{err}");
    let commands = asset("commands/gsd/quick.md");
    let find = line(&commands, "dir=$(").replace("{SLUG}", "trim-titles");
    let (found, err, _) = sh.run_in(p.root(), &format!("{find}\nprintf '%s' \"$dir\""));
    assert_eq!(found.len(), 10, "{found}{err}");
    // the model reads the id off the fence, for the fences that follow
    assert!(out.contains(&format!("quick_id={found}")), "{out}");
    assert_eq!(p.fields_of(&found)["slug"], "trim-titles");

    let stub = "specforge() { case \"$2\" in quick-init) printf 'x\\037\\037\\037t\\n' ;; *) echo WROTE ;; esac; }";
    let (out, _, code) = sh.run_in(
        p.root(),
        &format!("{stub}\nDESCRIPTION=d\n{init}\necho after"),
    );
    assert_eq!(
        (out.as_str(), code),
        ("", 1),
        "an empty slug writes nothing"
    );

    // the batch's: one fence mints every id and writes every plan
    let text = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let fence = block(&text, "QB_CREATE_LIST=", "specforge new --in");
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!(
            "QB_TASKS='- Add a task\n- List tasks'\nQB_TASKS=$(printf '%b' \"$QB_TASKS\")\n{fence}"
        ),
    );
    assert_eq!(code, 0, "{out}{err}");
    for slug in ["add-a-task", "list-tasks"] {
        let find = line(&commands, "dir=$(").replace("{SLUG}", slug);
        let (found, err, _) = sh.run_in(p.root(), &format!("{find}\nprintf '%s' \"$dir\""));
        assert_eq!(found.len(), 10, "{slug}: {found}{err}");
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

/// A quick-batch item is its quick plan, named by its id: nothing sets a `quick_dir`, so a path built from it is
/// `/<id>-<slug>`, at the filesystem's root. No prompt lists the project state or the project as a file: both are
/// computed or entities, and the values that named them were always empty.
#[test]
fn a_quick_batch_item_is_its_quick_plan() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    for path in [
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
    ] {
        let text = asset(path);
        let item = line(&text, "ITEM_DIR=");
        let out = sh.run(&format!(
            "quick_id=260101-abc SLUG=x description=x\n{item}\nprintf '%s' \"$ITEM_DIR\""
        ));
        assert_eq!(out, "260101-abc", "{path}");
    }
    let base = format!("{}/assets/gsd-core/workflows", env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![std::path::PathBuf::from(format!("{base}/quick-batch.md"))];
    let mut dirs = vec![std::path::PathBuf::from(format!("{base}/quick-batch"))];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("a directory").flatten() {
            let p = entry.path();
            if p.is_dir() {
                dirs.push(p);
            } else {
                files.push(p);
            }
        }
    }
    for file in files {
        let text = std::fs::read_to_string(&file).expect("an asset");
        for unsourced in ["quick_dir", "STATE_PATH", "PROJECT_PATH"] {
            assert!(
                !text.contains(unsourced),
                "{} reads {unsourced}",
                file.display()
            );
        }
    }
}

/// A task may start with a hyphen, and no parser reads it as a flag: the plan is written and holds it, as one task and
/// as an entry of a batch's list.
#[test]
fn a_task_starting_with_a_hyphen_is_a_task() {
    let Some(sh) = Shell::only(&[]) else { return };
    let p = twin();
    let quick = asset("gsd-core/workflows/quick.md");
    let init = line(&quick, "IFS=$'\\x1f' read -r quick_id slug")
        .replace("{{setting:git.quick_branch_template|}}", "");
    for task in ["--upload-pack=x", "-no space marker", "--help"] {
        let (out, err, code) = sh.run_in(
            p.root(),
            &format!(
                "DESCRIPTION={}\n{init}\nprintf '[%s]' \"$quick_id\"",
                q(task)
            ),
        );
        assert_eq!(code, 0, "{task}: {out}{err}");
        let id = out.rsplit('[').next().unwrap().trim_end_matches(']');
        assert_eq!(p.fields_of(id)["objective"], task, "{task}");
        let discussion = asset("gsd-core/workflows/quick/steps/discussion-phase.md");
        let new = discussion
            .split("(`")
            .nth(1)
            .and_then(|rest| rest.split("`, adding").next())
            .expect("the context line");
        let script = format!(
            "quick_id={id}; date=2026-01-01; DESCRIPTION={}
{new}",
            q(task)
        );
        let (out, err, code) = sh.run_in(p.root(), &script);
        assert_eq!(code, 0, "{task}: {out}{err}");
    }
    let text = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let fence = block(&text, "QB_CREATE_LIST=", "specforge new --in");
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!(
            "QB_TASKS={}\n{fence}\nprintf '%s' \"$QB_CREATE_LIST\"",
            q("--upload-pack=x\n-no space marker")
        ),
    );
    assert_eq!(code, 0, "{out}{err}");
    let items = rows(&out);
    assert_eq!(items.len(), 2, "{out}");
    for (row, task) in items.iter().zip(["--upload-pack=x", "-no space marker"]) {
        assert_eq!(p.fields_of(row[0])["objective"], task);
    }
}

/// A `--file` that reads as a flag names a file: it is refused as one that isn't there, and no batch is minted from
/// a help text.
#[test]
fn a_task_file_starting_with_a_hyphen_names_a_file() {
    let Some(sh) = Shell::only(&[]) else { return };
    let p = twin();
    let text = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let read = line(&text, "QB_TASKS=$(specforge files read");
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("TASK_FILE=--help\n{read}\necho read: \"$QB_TASKS\""),
    );
    assert_eq!(code, 1, "{out}{err}");
    assert_eq!(out, "", "{err}");
    p.write("-tasks.txt", "- a\n- b\n");
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("TASK_FILE=-tasks.txt\n{read}\nprintf '%s' \"$QB_TASKS\""),
    );
    assert_eq!((code, out.as_str()), (0, "- a\n- b"), "{err}");
}

/// An id the graph already holds is refused, so two tasks never share a plan, and no init line skips the write.
#[test]
fn a_colliding_quick_id_writes_nothing() {
    let Some(sh) = Shell::only(&[]) else { return };
    let p = twin();
    let quick = asset("gsd-core/workflows/quick.md");
    let init = line(&quick, "IFS=$'\\x1f' read -r quick_id slug")
        .replace("{{setting:git.quick_branch_template|}}", "");
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("DESCRIPTION='the first'\n{init}\nprintf '[%s]' \"$quick_id\""),
    );
    assert_eq!(code, 0, "{out}{err}");
    let id = out
        .rsplit('[')
        .next()
        .unwrap()
        .trim_end_matches(']')
        .to_string();
    let written = p.read("spec/quick.spec");
    // the host's mint is what would never answer a held id: a stub stands in for it
    let stub = format!(
        "specforge() {{ if [ \"$2\" = quick-init ]; then printf '{id}\\037second\\037\\037second\\n'; \
         else command specforge \"$@\"; fi; }}"
    );
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("{stub}\nDESCRIPTION=second\n{init}\necho after"),
    );
    assert_eq!(code, 1, "{out}{err}");
    assert!(!out.contains("after"), "{out}");
    assert!(err.contains("entity_exists"), "{err}");
    assert_eq!(p.read("spec/quick.spec"), written);
    assert_eq!(p.fields_of(&id)["objective"], "the first");
    for path in [
        "gsd-core/workflows/quick/steps/research-phase.md",
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        "gsd-core/workflows/quick.md",
        "gsd-core/workflows/quick-batch/steps/planner-wave.md",
    ] {
        assert!(!asset(path).contains("|| specforge new"), "{path}");
    }
}

/// One call mints the ids and tells the day, the batch and the count, so a second can't differ from it across midnight.
#[test]
fn one_call_of_quick_init_reads_everything() {
    for path in [
        "gsd-core/workflows/quick.md",
        "gsd-core/workflows/quick-batch/steps/batch-init.md",
    ] {
        let calls = asset(path).matches("specforge gsd quick-init").count();
        assert_eq!(calls, 1, "{path}");
    }
    let Some(sh) = Shell::only(&[]) else { return };
    let p = twin();
    let quick = asset("gsd-core/workflows/quick.md");
    let init = line(&quick, "IFS=$'\\x1f' read -r quick_id slug")
        .replace("{{setting:git.quick_branch_template|}}", "");
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("DESCRIPTION='Trim titles'\n{init}\nprintf '[%s]' \"$date\""),
    );
    assert_eq!(code, 0, "{out}{err}");
    let date = out.rsplit('[').next().unwrap().trim_end_matches(']');
    assert_eq!(date.len(), 10, "{out}");
    // the fences that follow read it off this one
    assert!(out.contains(&format!("date={date}")), "{out}");
    // The batch's second read stops when the list holds no row.
    let text = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let read = line(&text, "IFS=$'\\x1f' read -r _ _ _ BATCH_ID");
    let (out, err, code) = sh.run_in(p.root(), &format!("QB_CREATE_LIST=\n{read}\necho after"));
    assert_eq!((out.as_str(), code), ("", 1), "{err}");
}

/// Resume refuses closed: a batch id nothing recorded, and a batch whose base is no longer an ancestor of `HEAD`.
#[test]
fn resuming_an_unknown_or_diverged_batch_stops() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = twin();
    p.commit();
    let resume = fence_after(
        &asset("gsd-core/workflows/quick-batch/steps/resume-mode.md"),
        "**Step 2a",
    );
    let rc = |id: &str| {
        sh.run_in(
            p.root(),
            &format!("RESUME_BATCH_ID={id}\n{resume}\nprintf '%s' \"$QB_RESUME_RC\""),
        )
    };
    let (out, err, _) = rc("261112-batch");
    assert_ne!(out, "0", "an unknown batch resumes: {err}");
    // a base no commit of this history holds
    quick_plans(&p);
    let base = "0123456789abcdef0123456789abcdef01234567";
    let items = r#"["261112-abc", "261112-def"]"#;
    let run = p.run(&[
        "gsd",
        "start-batch",
        "261112-batch",
        "--items",
        items,
        "--base",
        base,
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let (out, err, _) = rc("261112-batch");
    assert_eq!(out, "1", "{err}");
    assert!(err.contains("diverged"), "{err}");
}

/// Two quick plans of the twin, as batch-init leaves them before it records the batch.
fn quick_plans(p: &crate::harness::Project) {
    crate::commands_support::plant(
        p,
        "spec/quick.spec",
        "plan 261112-abc \"Trim titles\" {\n  type           execute\n  files_modified []\n  objective      \"Trim titles\"\n  slug           \"trim-titles\"\n}\n\nplan 261112-def \"Sort tasks\" {\n  type           execute\n  files_modified []\n  objective      \"Sort tasks\"\n  slug           \"sort-tasks\"\n}\n",
    );
}

/// batch-init records the batch on the commit it starts from, and resume reads it back from the graph: its fields,
/// its manifest and its eligible items.
#[test]
fn a_batch_is_recorded_and_resumed_from_the_graph() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = twin();
    p.commit();
    let init = asset("gsd-core/workflows/quick-batch/steps/batch-init.md");
    let create = block(&init, "QB_CREATE_LIST=", "specforge new --in");
    let record = fence_after(&init, "**Otherwise:** `$QB_CREATE_LIST` holds");
    let (id, err, code) = sh.run_in(
        p.root(),
        &format!(
            "QB_TASKS=$(printf '%b' '- Trim titles\\n- Sort tasks')\n{create}\n{record}\nprintf '%s' \"$BATCH_ID\""
        ),
    );
    assert_eq!(code, 0, "{id}{err}");
    let batch = p.fields_of(&id);
    let items = batch["items"].as_array().expect("items").clone();
    assert_eq!(items.len(), 2, "{batch}");
    assert_eq!(batch["status"], "open");
    let head = p.run(&["git", "head", "--pick", ".sha"]);
    assert_eq!(batch["base"], head.stdout.trim_end(), "{}", head.all());

    let resume_text = asset("gsd-core/workflows/quick-batch/steps/resume-mode.md");
    let resume = fence_after(&resume_text, "**Step 2a");
    let manifest = fence_after(&resume_text, "**Otherwise:**");
    let (out, err, _) = sh.run_in(
        p.root(),
        &format!(
            "RESUME_BATCH_ID={id}\n{resume}\n[ \"$QB_RESUME_RC\" -eq 0 ] || exit 9\n{manifest}\nprintf '%s' \"$BATCH_MANIFEST_JSON\""
        ),
    );
    let rows: Vec<&str> = out.lines().collect();
    assert_eq!(rows.len(), 2, "{out}{err}");
    for (row, item) in rows.iter().zip(&items) {
        assert!(row.starts_with(item.as_str().unwrap_or_default()), "{row}");
    }
    let dispatch = fence_from(
        &asset("gsd-core/workflows/quick-batch/steps/worktree-dispatch.md"),
        "QB_ELIGIBLE=",
    );
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!("BATCH_ID={id}\n{dispatch}\nprintf '%s' \"$QB_ELIGIBLE\""),
    );
    assert_eq!(code, 0, "{out}{err}");
    let eligible: Vec<&str> = out.lines().collect();
    let expected: Vec<&str> = items.iter().filter_map(|i| i.as_str()).collect();
    assert_eq!(eligible, expected, "{err}");
}
