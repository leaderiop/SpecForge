//! `specforge gsd automated-guard <command> [--common-dir D]`: the words of a plan's command against the worktree it
//! verifies, in a real linked worktree.

use super::commands_support::{gsd, json};
use crate::harness::Project;
use crate::hooks_isolation::linked_worktree;

fn guard(p: &Project, command: &str, common: &str) -> (i32, serde_json::Value) {
    let common = format!("--common-dir={common}");
    let out = gsd(
        p,
        "automated-guard",
        &[command, &common, "--format", "json"],
    );
    (out.code, json(&out))
}

/// An empty command, or an empty common directory (its read failed), never reads as a pass.
#[test]
fn an_empty_command_or_common_dir_refuses() {
    let p = crate::harness::project("execute");
    p.commit();
    for args in [
        vec!["", "--format", "json"],
        vec!["   ", "--format", "json"],
        vec!["cat /etc/hosts", "--common-dir=", "--format", "json"],
        vec!["cat /etc/hosts", "--common-dir", "", "--format", "json"],
        vec!["", "--task", "--format", "json"],
    ] {
        let out = gsd(&p, "automated-guard", &args);
        assert_ne!(out.code, 0, "{args:?}: {}", out.all());
    }
    let out = gsd(&p, "automated-guard", &["cat /etc/hosts"]);
    assert_eq!(out.code, 0, "no common dir given: {}", out.all());
}

/// The task's commands, all of them, from the graph.
#[test]
fn a_task_is_read_for_each_of_its_commands() {
    let p = crate::harness::project("execute");
    p.commit();
    let clean = guard_task(&p, "v1.0/01/01-01/T1");
    assert_eq!(clean.0, 0, "{}", clean.1);
    assert_eq!(clean.1["commands"][0], "node --test test/tasks.test.js");
    let spec = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &spec.replacen(
            "{ automated \"node --test test/tasks.test.js\" fails_when \"a test fails or none runs\" }",
            "{ automated \"node -v\" fails_when \"x\" }\n    { automated \"cd ../elsewhere && make\" fails_when \"x\" }",
            1,
        ),
    );
    let halted = guard_task(&p, "v1.0/01/01-01/T2");
    assert_eq!(halted.0, 1, "{}", halted.1);
    assert_eq!(halted.1["outside"][0]["command"], "cd ../elsewhere && make");
    assert_ne!(guard_task(&p, "v1.0/01/01-01/T9").0, 0, "no such task");
}

fn guard_task(p: &Project, task: &str) -> (i32, serde_json::Value) {
    let out = gsd(p, "automated-guard", &[task, "--task", "--format", "json"]);
    let answer = serde_json::from_str(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.code, answer)
}

#[test]
fn the_guard_halts_on_the_main_checkout_from_a_linked_worktree() {
    let Some((wt, main)) = linked_worktree() else {
        return;
    };
    let main = main.canonicalize().unwrap();
    let top = wt.root().canonicalize().unwrap();
    let common = main.join(".git");
    let common = common.to_str().unwrap();
    std::fs::create_dir_all(top.join("src")).unwrap();

    let (code, answer) = guard(&wt, "cd ../../../x && make", common);
    assert_eq!(code, 1, "{answer}");
    assert_eq!(answer["ok"], false, "{answer}");
    assert_eq!(answer["outside"][0]["word"], "../../../x", "{answer}");

    let (code, answer) = guard(
        &wt,
        &format!("cat {}", main.join("README").display()),
        common,
    );
    assert_eq!(code, 1, "{answer}");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&main, top.join("back")).unwrap();
        let (code, answer) = guard(&wt, "cd back/x && make", common);
        assert_eq!(code, 1, "a link into the main checkout: {answer}");
        assert_eq!(
            answer["outside"][0]["why"], "a link out of the worktree",
            "{answer}"
        );
    }

    // What the shell expands: the part written out is judged.
    let main_text = main.display().to_string();
    let top_text = top.display().to_string();
    for command in [
        format!("cd {main_text}/$X"),
        format!("cd {main_text}/*"),
        format!("cat {main_text}/*"),
        format!("cat {main_text}/$X"),
        format!("PATH={main_text}/bin:$PATH make"),
        format!("MISSING ; cd {main_text} && make"),
        format!("MISSING x; cat {main_text}/x"),
        format!("echo \"$(cat {main_text}/x)\""),
        format!("echo `cat {main_text}/x`"),
        format!("function f {{ cd {main_text}; }}; f"),
        format!("make&>{main_text}/log"),
    ] {
        let (code, answer) = guard(&wt, &command, common);
        assert_eq!(code, 1, "{command}: {answer}");
    }
    #[cfg(unix)]
    for command in [
        "cd back/$X".to_string(),
        format!("cat {top_text}/back/x"),
        format!("cat {top_text}/back/*"),
    ] {
        let (code, answer) = guard(&wt, &command, common);
        assert_eq!(code, 1, "{command}: {answer}");
    }
    let (code, answer) = guard(&wt, &format!("cat {top_text}/src/*"), common);
    assert_eq!(code, 0, "{answer}");
    let (code, answer) = guard(&wt, "MISSING - Wave 0, can't yet", common);
    assert_eq!(code, 0, "{answer}");

    let (code, answer) = guard(&wt, "cd src && ls", common);
    assert_eq!(code, 0, "{answer}");
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(answer["top"], top.to_str().unwrap(), "{answer}");
    assert_eq!(answer["main"], main.to_str().unwrap(), "{answer}");

    let refused = gsd(
        &wt,
        "automated-guard",
        &["cat 'a", &format!("--common-dir={common}")],
    );
    assert_ne!(refused.code, 0, "an unreadable command: {}", refused.all());
    assert!(refused.all().contains("FATAL"), "{}", refused.all());
}
