//! `automated_guard <command> [--common-dir D]`: whether a plan's `<automated>` command leaves the worktree it is meant
//! to verify, read as the shell's words and never run. Read-only.
//!
//! - D-s1-07-7 (where the words come from): the reader lives in `specforge-protocol-types`, which host and guest both
//!   link, so there is one reader and no wire change, and a nested `bash -c '…'` is read too, which a host input of one
//!   arg would not reach. ADR 0292 D4 still holds: hooks get the words from the host.
//! - D-s1-07-8 (fail closed, `[guard-reads-words]`): an unreadable command halts; upstream's regexes read it anyway. The
//!   words also close upstream's stated limits: quoted boundaries, keywords, `command`/`exec`, `--` before a flag.
//!
//! shortcut: a target is resolved lexically and a link is seen only inside the project, through the file port; a link
//! outside it, or a non-canonical spelling of the main checkout, is not. Upgrade when the file port reads outside roots.

use serde_json::json;
use specforge_extension_sdk::host::{fs, git};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, run};
use crate::relocation::{Event, dynamic, events, missing_sentinel};

/// Where a command must stay: the worktree's top, and the main checkout it must not reach.
struct Roots<'a> {
    top: &'a str,
    main: &'a str,
}

/// A word that leaves the worktree.
#[derive(Debug, PartialEq)]
struct Finding {
    word: String,
    path: String,
    why: &'static str,
}

#[derive(Debug, Default, PartialEq)]
struct Judged {
    outside: Vec<Finding>,
    dynamic: Vec<String>,
}

/// `C:/` of a path that starts with a Windows drive.
fn drive(path: &str) -> Option<&str> {
    let (letter, _) = path.split_once(":/")?;
    (letter.len() == 1).then(|| &path[..letter.len() + 2])
}

/// `target` from `from`, with `.` and `..` folded and no link followed.
fn resolved(from: &str, target: &str) -> String {
    let joined = if target.starts_with('/') || drive(target).is_some() {
        target.to_string()
    } else {
        format!("{from}/{target}")
    };
    let root = drive(&joined).unwrap_or("/");
    let mut parts: Vec<&str> = Vec::new();
    for part in joined[if root == "/" { 0 } else { root.len() }..].split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    format!("{root}{}", parts.join("/"))
}

fn inside(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root.trim_end_matches('/'))
            .is_some_and(|rest| rest.starts_with('/'))
}

/// The main checkout: `D` without its last `.git`, the top when none is given. A relative `D` is from the top.
fn main_checkout(common_dir: Option<&str>, top: &str) -> String {
    match common_dir.filter(|dir| !dir.is_empty()) {
        Some(dir) => {
            let dir = resolved(top, dir);
            match dir.strip_suffix("/.git") {
                Some("") => "/".to_string(),
                Some(main) => main.to_string(),
                None => dir,
            }
        }
        None => top.to_string(),
    }
}

/// The walk's verdict. `leaves` says whether a path inside the worktree leads out of it through a link.
fn judge(
    command: &str,
    roots: &Roots<'_>,
    leaves: &dyn Fn(&str) -> bool,
) -> Result<Judged, String> {
    let mut judged = Judged::default();
    if missing_sentinel(command.trim()) {
        return Ok(judged);
    }
    // `None`: a dynamic `cd` was passed, and a relative target from there cannot be read.
    let mut cwd = Some(roots.top.to_string());
    let mut saved: Vec<Option<String>> = Vec::new();
    for event in events(command)? {
        match event {
            Event::Enter => saved.push(cwd.clone()),
            Event::Leave => cwd = saved.pop().flatten(),
            Event::Cd(target) => {
                let reached = relocated(&target, cwd.as_deref(), roots, leaves, &mut judged);
                cwd = reached;
            }
            Event::Flag { target, .. } => {
                relocated(&target, cwd.as_deref(), roots, leaves, &mut judged);
            }
            Event::Absolute(word) if dynamic(&word) => judged.dynamic.push(word),
            Event::Absolute(word) => {
                let path = resolved("/", &word);
                if inside(&path, roots.main) && !inside(&path, roots.top) {
                    judged.outside.push(Finding {
                        word,
                        path,
                        why: "an absolute path inside the main checkout",
                    });
                }
            }
            Event::Run(_) => {}
        }
    }
    Ok(judged)
}

/// Judges a relocation to `target`, and answers the directory it reaches, `None` when that cannot be read.
fn relocated(
    target: &str,
    cwd: Option<&str>,
    roots: &Roots<'_>,
    leaves: &dyn Fn(&str) -> bool,
    judged: &mut Judged,
) -> Option<String> {
    if dynamic(target) {
        judged.dynamic.push(target.to_string());
        return None;
    }
    let from = match cwd {
        Some(from) => from,
        None if target.starts_with('/') || drive(target).is_some() => "/",
        None => {
            judged.dynamic.push(target.to_string());
            return None;
        }
    };
    let path = resolved(from, target);
    let why = if !inside(&path, roots.top) {
        Some("a relocation outside the worktree")
    } else if leaves(&path) {
        Some("a link out of the worktree")
    } else {
        None
    };
    if let Some(why) = why {
        judged.outside.push(Finding {
            word: target.to_string(),
            path: path.clone(),
            why,
        });
    }
    Some(path)
}

fn fatal(command: &str, why: &str) -> String {
    format!(
        "FATAL: <automated> command {command:?}: {why} — it would verify the wrong checkout. Rewrite the plan's command \
         root-relative (cwd is the checkout root); do not rewrite it in place."
    )
}

fn automated_guard(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let command = call.str("command").unwrap_or_default();
    let located = git::locate().map_err(|error| fatal(command, &format!("{error:?}")))?;
    let top = located.map_or_else(|| call.cwd().to_string(), |repo| repo.toplevel);
    let main = main_checkout(call.str("common_dir"), &top);
    let roots = Roots {
        top: &top,
        main: &main,
    };
    let project = call.cwd();
    // A path the file port refuses to reach is outside; one it answers for, present or not, is not.
    let leaves = |path: &str| match path.strip_prefix(project.trim_end_matches('/')) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            fs::stat("project", rest.trim_start_matches('/')).is_err()
        }
        _ => false,
    };
    let judged = judge(command, &roots, &leaves).map_err(|why| fatal(command, &why))?;
    let ok = judged.outside.is_empty();
    let human = if ok {
        format!("ok: {command}")
    } else {
        judged
            .outside
            .iter()
            .map(|f| fatal(command, &format!("{} ({} -> {})", f.why, f.word, f.path)))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let outside: Vec<_> = judged
        .outside
        .iter()
        .map(|f| json!({"word": f.word, "path": f.path, "why": f.why}))
        .collect();
    let mut out = answer(
        call,
        json!({
            "ok": ok,
            "top": top,
            "main": main,
            "outside": outside,
            "dynamic": judged.dynamic,
        }),
        human,
        None,
    );
    if !ok {
        out.exit_code = 1;
    }
    Ok(out)
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.capability(
        Capability::fs("project")
            .ops(&[FsAccess::Stat])
            .why("see whether an <automated> command's directory leads out through a link"),
    );
    c.command("automated_guard", |cmd| {
        cmd.description(
            "Halt when a plan's <automated> command would leave the worktree: its words are read, never run",
        )
        .arg("command", |a| {
            a.required()
                .description("the <automated> command, as the plan wrote it");
        })
        .arg("common_dir", |a| {
            a.description(
                "the repository's common directory (`specforge git repo --pick .common_dir`): the main checkout is it without its `.git`",
            );
        })
        .handler(|call| run(call, automated_guard));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOTS: Roots<'static> = Roots {
        top: "/wt",
        main: "/main",
    };

    fn verdict(command: &str) -> Judged {
        judge(command, &ROOTS, &|_| false).expect("readable")
    }

    fn halts(command: &str) -> bool {
        !verdict(command).outside.is_empty()
    }

    #[test]
    fn upstreams_boundary_cases() {
        for command in [
            "cd /main && npm test",
            "cd scripts && cd ../..",
            "env -u FOO --debug -u BAR -C ../main make",
            "bash -c 'cd ../main'",
            "command bash -c 'cd ../main'",
            "if cd ../main; then make; fi",
            r#"python3 -c 'import os; os.chdir("/main")'"#,
        ] {
            assert!(halts(command), "{command}");
        }
        for command in [
            "grep -F 'if cd ../main; then' README.md",
            r#"echo 'note; bash -c "cd ../main"'"#,
            "command -v bash",
            "cd scripts && cd ..",
            "npm run package -- --prefix=../main/",
            "cat /dev/null",
            "cd $VAR",
            "MISSING - Wave 0",
        ] {
            assert!(!halts(command), "{command}: {:?}", verdict(command));
        }
        assert!(judge("echo 'a", &ROOTS, &|_| false).is_err(), "unreadable");
    }

    #[test]
    fn a_subshell_keeps_its_move_and_a_dynamic_cd_makes_what_follows_dynamic() {
        // `(cd src)` leaves the cwd at the top, so `../main` from there is outside; from src it would be inside `/wt`.
        assert!(halts("(cd src) && cd ../main"));
        assert!(!halts("cd src && cd .."));
        let judged = verdict("cd $VAR && cd ../main");
        assert!(judged.outside.is_empty(), "{judged:?}");
        assert_eq!(judged.dynamic, ["$VAR", "../main"]);
        assert!(halts("cd $VAR && cd /main"), "an absolute target is read");
        assert!(!halts("(cd $VAR); cd src"), "the subshell's cwd is gone");
    }

    #[test]
    fn an_absolute_word_counts_inside_the_main_checkout_only() {
        let finding = &verdict("cat /main/README").outside[0];
        assert_eq!(finding.path, "/main/README");
        assert!(!halts("cat /wt/README"));
        assert!(!halts("cat /etc/hosts"));
        assert!(halts("cat /wt/../main/README"), "folded first");
        // A worktree inside the main checkout is its own.
        let nested = Roots {
            top: "/main/.claude/wt",
            main: "/main",
        };
        let judged = judge("cat /main/.claude/wt/x", &nested, &|_| false).unwrap();
        assert!(judged.outside.is_empty());
        let judged = judge("cat /main/x", &nested, &|_| false).unwrap();
        assert_eq!(judged.outside.len(), 1);
    }

    #[test]
    fn a_link_out_of_the_worktree_is_outside() {
        let judged = judge("cd link/x && make", &ROOTS, &|path| {
            path.starts_with("/wt/link")
        })
        .unwrap();
        assert_eq!(judged.outside[0].why, "a link out of the worktree");
        assert_eq!(judged.outside[0].path, "/wt/link/x");
    }

    #[test]
    fn the_main_checkout_is_the_common_dir_without_its_git() {
        assert_eq!(main_checkout(Some("/main/.git"), "/wt"), "/main");
        assert_eq!(main_checkout(Some("/bare.git"), "/wt"), "/bare.git");
        assert_eq!(main_checkout(Some(".git"), "/wt"), "/wt");
        assert_eq!(main_checkout(Some(""), "/wt"), "/wt");
        assert_eq!(main_checkout(None, "/wt"), "/wt");
    }

    #[test]
    fn a_target_is_folded_and_a_prefix_is_no_parent() {
        assert_eq!(resolved("/wt/a", "../b/./c"), "/wt/b/c");
        assert_eq!(resolved("/wt", "/x/../y"), "/y");
        assert!(inside("/wt/a", "/wt"));
        assert!(inside("/wt", "/wt"));
        assert!(!inside("/wt-other", "/wt"), "a sibling with the same start");
        assert_eq!(resolved("C:/wt/a", "../b"), "C:/wt/b");
        assert_eq!(resolved("C:/wt", "D:/x"), "D:/x");
        assert!(inside(&resolved("C:/wt", "src"), "C:/wt"));
    }
}
