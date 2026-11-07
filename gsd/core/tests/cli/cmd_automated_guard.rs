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
