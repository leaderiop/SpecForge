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

use serde_json::{Value, json};
use specforge_extension_sdk::host::{fs, git};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, given, node, required, run};
use crate::relocation::{Event, dynamic, events, literal_prefix, missing_sentinel};

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
    match common_dir {
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
    let mut cwd = Cwd {
        path: roots.top.to_string(),
        read: true,
    };
    let mut saved = Vec::new();
    for event in events(command)? {
        match event {
            Event::Enter => saved.push(cwd.clone()),
            Event::Leave => {
                if let Some(before) = saved.pop() {
                    cwd = before;
                }
            }
            Event::Cd(target) => {
                let reached = relocated(&target, &cwd, roots, leaves, &mut judged);
                cwd = reached;
            }
            Event::Flag { target, .. } => {
                relocated(&target, &cwd, roots, leaves, &mut judged);
            }
            Event::Absolute(word) => absolute_word(word, roots, leaves, &mut judged),
            Event::Run(_) => {}
        }
    }
    Ok(judged)
}

/// Where the walk stands. Past a `cd` the shell expands, `read` is false: a relative target from there is named under
/// `dynamic`, and is still folded with the expansion as one directory, as far as that shows it climbing out.
#[derive(Clone)]
struct Cwd {
    path: String,
    read: bool,
}

/// An absolute word is judged by all it spells and by what it spells before an expansion: `/main/*` names the main checkout.
fn absolute_word(
    word: String,
    roots: &Roots<'_>,
    leaves: &dyn Fn(&str) -> bool,
    judged: &mut Judged,
) {
    if dynamic(&word) {
        judged.dynamic.push(word.clone());
    }
    let reaches = |path: &str| inside(path, roots.main) && !inside(path, roots.top);
    let whole = resolved("/", &word);
    let prefix = resolved("/", literal_prefix(&word));
    if reaches(&whole) || reaches(&prefix) {
        judged.outside.push(Finding {
            word,
            path: whole,
            why: "an absolute path inside the main checkout",
        });
    } else if inside(&prefix, roots.top) && leaves(&prefix) {
        judged.outside.push(Finding {
            word,
            path: whole,
            why: "a link out of the worktree",
        });
    }
}

/// Judges a relocation to `target`, and answers where it stands after it. What the shell expands is folded as one more
/// directory, and judged by that and by what it spells before the expansion, so only the part no reading can fix passes.
fn relocated(
    target: &str,
    cwd: &Cwd,
    roots: &Roots<'_>,
    leaves: &dyn Fn(&str) -> bool,
    judged: &mut Judged,
) -> Cwd {
    let rooted = target.starts_with('/') || drive(target).is_some();
    let read = !dynamic(target) && (rooted || cwd.read);
    if !read {
        judged.dynamic.push(target.to_string());
    }
    let path = resolved(&cwd.path, target);
    let prefix = resolved(&cwd.path, literal_prefix(target));
    let why = if !inside(&path, roots.top) || !inside(&prefix, roots.top) {
        Some("a relocation outside the worktree")
    } else if (rooted || cwd.read) && leaves(&prefix) {
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
    Cwd { path, read }
}

fn fatal(command: &str, why: &str) -> String {
    format!(
        "FATAL: <automated> command {command:?}: {why} — it would verify the wrong checkout. Rewrite the plan's command \
         root-relative (cwd is the checkout root); do not rewrite it in place."
    )
}

/// The `<automated>` commands of a task, as the plan wrote them.
fn commands_of(task: &GraphNode) -> Vec<String> {
    task.fields
        .get("verify")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("automated")?.as_str().map(str::to_string))
        .collect()
}

fn automated_guard(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let given_text = required(call, "command")?;
    let commands = if call.flag("task") {
        commands_of(node(call, "task", given_text)?)
    } else {
        vec![given_text.to_string()]
    };
    let common_dir = given(call, "common_dir")?;
    let refused = |error: String| fatal(given_text, &error);
    let located = git::locate().map_err(|error| refused(format!("{error:?}")))?;
    let top = located.map_or_else(|| call.cwd().to_string(), |repo| repo.toplevel);
    let main = main_checkout(common_dir, &top);
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
    let mut human = Vec::new();
    let mut outside = Vec::new();
    let mut dynamics = Vec::new();
    for command in &commands {
        let judged = judge(command, &roots, &leaves).map_err(|why| fatal(command, &why))?;
        for finding in &judged.outside {
            human.push(fatal(
                command,
                &format!("{} ({} -> {})", finding.why, finding.word, finding.path),
            ));
            outside.push(json!({
                "command": command,
                "word": finding.word,
                "path": finding.path,
                "why": finding.why,
            }));
        }
        dynamics.extend(judged.dynamic);
    }
    let ok = outside.is_empty();
    let human = if ok {
        format!("ok: {}", commands.join("; "))
    } else {
        human.join("\n")
    };
    let mut out = answer(
        call,
        json!({
            "ok": ok,
            "top": top,
            "main": main,
            "commands": commands,
            "outside": outside,
            "dynamic": dynamics,
        }),
        human,
        None,
    );
    if !ok {
        out.exit_code = 1;
    }
    Ok(out)
}

/// `worktree_contains <target>`: whether `target` is the worktree or under it, read as the guard reads a target, so a
/// name that only starts like the worktree's and a `..` out of it are outside.
fn worktree_contains(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let path = required(call, "target")?;
    let located = git::locate()
        .map_err(|error| format!("the worktree cannot be read: {error:?}"))?
        .ok_or("this is no git repository")?;
    let top = resolved(&located.toplevel, ".");
    let is_inside = inside(&resolved(&top, path), &top);
    Ok(answer(
        call,
        json!({"path": path, "top": top, "inside": is_inside}),
        format!("{}: {path}", if is_inside { "inside" } else { "outside" }),
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("worktree_contains", |cmd| {
        cmd.description("Answer whether a path is inside this worktree, read lexically as the guard reads a target")
            .arg("target", |a| {
                a.required().description("the path; a relative one is from the worktree's top");
            })
            .handler(|call| run(call, worktree_contains));
    });
    c.capability(
        Capability::fs("project")
            .ops(&[FsAccess::Stat])
            .why("see whether an <automated> command's directory leads out through a link, and what a gate's manifest is"),
    );
    c.command("automated_guard", |cmd| {
        cmd.description(
            "Halt when a plan's <automated> command would leave the worktree: its words are read, never run",
        )
        .arg("command", |a| {
            a.required().description(
                "the <automated> command, as the plan wrote it; with --task, the task whose commands are read",
            );
        })
        .arg("task", |a| {
            a.flag()
                .description("the argument names a task: read each of its `<automated>` commands");
        })
        .arg("common_dir", |a| {
            a.description(
                "the repository's common directory (`specforge git repo --pick .common_dir`): the main checkout is it without its `.git`. Empty is refused: it means that read failed",
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
        // `$VAR` counts as one directory: this stays in `/wt`, read as dynamic.
        let judged = verdict("cd $VAR && cd ../main");
        assert!(judged.outside.is_empty(), "{judged:?}");
        assert_eq!(judged.dynamic, ["$VAR", "../main"]);
        assert!(halts("cd $VAR && cd /main"), "an absolute target is read");
        assert!(!halts("(cd $VAR); cd src"), "the subshell's cwd is gone");
    }

    /// The guard before the words halted on every row. Two kinds of row it halted pass, as the words tell them apart:
    /// quoted text (`echo '; cd ../main'` runs nothing) and another tool's flag (`git status -C ../main`).
    #[test]
    fn nothing_the_old_guard_halted_passes() {
        for command in [
            "cd ../main",
            "cd /main",
            "cat /main/x",
            "cd /main/$X",
            "cd /main/*",
            "cd /main/a?",
            "cat /main/$X",
            "cat /main/*",
            "cd ../main/*",
            "cd ../main/$X",
            "cd ../ma*",
            "cd $X/../..",
            "cd a*/../..",
            "cd $X && cd ../../main",
            "cd /tmp/$X",
            "PATH=/main/bin:$PATH make",
            "FOO=/main/x make",
            "--x=/main/y",
            "tool --x=/main/$Y",
            "MISSING ; cd ../main && make",
            "MISSING && cd ../main",
            "MISSING x; cd /main",
            "MISSING x\ncd /main",
            r#"echo "$(cd ../main && make)""#,
            "echo `cat /main/x`",
            "echo $(cat /main/x)",
            "cd `pwd`/../..",
            "cd $(pwd)/../..",
            "x=`cd ../main`",
            "function f { cd ../main; }; f",
            "f() { cd ../main; }; f",
            "{ cd ../main; }",
            "(cd ../main)",
            "cat x 2>&1; cd ../main",
            "cat x &>y; cd ../main",
            "cat x 2>&1 && cd ../main",
            "pushd ../main",
            "npm --prefix ../main test",
            "yarn --cwd=../main test",
            "pnpm -C ../main test",
            "make -C ../main",
            "git -C ../main status",
            "git --directory=../main status",
            "env -C ../main make",
            "env --chdir=../main make",
            "eval 'cd ../main'",
            "bash -c 'cd ../main'",
            "/bin/sh -c 'cd /main'",
            "timeout 5 bash -c 'cd ../main'",
            // Its readable flags, which the guard before the words did not read either.
            "cd -LP ../main",
            "env -iC ../main make",
            "time -p cd ../main",
            "make -wC ../main",
            "bash --rcfile x -c 'cd ../main'",
            "bash -eo pipefail -c 'cd ../main'",
            "`cd ../main`",
        ] {
            assert!(halts(command), "{command}: {:?}", verdict(command));
        }
        // What only the shell can name passes, named.
        for command in ["cd $X", "cd $X/y", "cd ~/x", "cat /etc/*", "cat /$X"] {
            let judged = verdict(command);
            assert!(judged.outside.is_empty(), "{command}: {judged:?}");
            assert!(!judged.dynamic.is_empty(), "{command}: {judged:?}");
        }
        assert!(!halts("cat $X/y"), "a relative word is not read");
        assert!(!halts("MISSING - Wave 0, can't yet"));
        assert!(!halts("echo '; cd ../main'"), "quoted text runs nothing");
        assert!(!halts("git log -C ../main"), "log's own -C moves nothing");
    }

    #[test]
    fn a_link_is_seen_under_an_expansion_and_in_an_absolute_word() {
        let leaves = |path: &str| path.starts_with("/wt/link");
        for command in [
            "cd link/$X",
            "cd link/*",
            "cat /wt/link/x",
            "cat /wt/link/*",
        ] {
            let judged = judge(command, &ROOTS, &leaves).unwrap();
            assert_eq!(
                judged.outside[0].why, "a link out of the worktree",
                "{command}"
            );
        }
        assert!(
            judge("cat /wt/src/*", &ROOTS, &leaves)
                .unwrap()
                .outside
                .is_empty()
        );
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
