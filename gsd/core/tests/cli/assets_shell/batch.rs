//! The quick batch's list is rows of one `quick-init` answer (`quick_id`, `slug`, `description`), its merge-eligible
//! prefix a bash array and its verification route a `case`: no jq on the `PATH`.

use super::{Shell, asset, block, line, q};
use crate::commands_support::twin;

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
        line(&text, "IFS=$'\\x1f' read -r BATCH_ID"),
        line(&text, "BATCH_MANIFEST_JSON="),
    ]
    .join("\n");
    let run = |list: &str| {
        sh.run_in(
            p.root(),
            &format!(
                "INLINE_TASK_LIST={}\n{lines}\n\
                 printf '%s|%s|%s\\n' \"$QB_CREATE_RC\" \"$ITEM_COUNT\" \"$BATCH_ID\"; printf '%s\\n' \"$BATCH_MANIFEST_JSON\"",
                q(list)
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
    let items: Vec<Vec<&str>> = lines[1..]
        .iter()
        .map(|l| l.split('\u{1f}').collect())
        .collect();
    let descriptions: Vec<&str> = items.iter().map(|row| row[2]).collect();
    assert_eq!(descriptions, ["Add a task", "Remove a task", "List tasks"]);
    let slugs: Vec<&str> = items.iter().map(|row| row[1]).collect();
    assert_eq!(slugs, ["add-a-task", "remove-a-task", "list-tasks"]);
    for row in &items {
        let id = row[0];
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
    // one item is no batch: the create step refuses, saying why, and no row is kept
    let (out, err, _) = run("- Only this\n");
    assert!(out.starts_with('1'), "{out}{err}");
    assert!(err.contains("a batch needs at least 2 tasks"), "{err}");
    assert_eq!(out.lines().nth(1).unwrap_or_default(), "", "{out}");
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
