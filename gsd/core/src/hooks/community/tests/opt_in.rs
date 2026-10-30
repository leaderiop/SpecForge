//! Ported from gsd-core v1.18.0 tests/hooks-opt-in.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The suite spawns the three bash hooks (`validate-commit`, `phase-boundary`, `session-state`) in a temp project. Here
//! `validate_commit` is a function of the message the kernel extracts and the raw command, so a case is ported when
//! what it decides is that verdict, and skipped, with its reason, when it tests the shell script, the installer, the
//! opt-in (the hook's `when`, tested black-box in `tests/cli/hooks_community.rs`) or how bash hands words to git (the
//! kernel's `facts::commit` reads them). Each ported case is asserted where its upstream title is written.

use super::{denied, reason, spec_write, with_command};
use crate::hooks::accounting;
use crate::hooks::community::{phase_boundary, validate_commit};
use crate::hooks::facts::Facts;
use specforge_extension_sdk::prelude::*;

const SUITE: &str =
    include_str!("../../../../../parity/upstream/v1.18.0/tests/hooks-opt-in.test.cjs");

fn heredoc(body: &str, open: &str, close: &str) -> String {
    format!("$(cat {open}\n{body}\n{close}\n)")
}

fn quoted(body: &str) -> String {
    heredoc(body, "<<'EOF'", "EOF")
}

fn with_types(message: &str, types: &[&str]) -> bool {
    denied(message, types)
}

fn ported() -> Vec<String> {
    let mut ported = Vec::new();
    let mut port = |title: &str| ported.push(title.to_string());

    assert!(!denied("fix(core): add locking mechanism", &[]));
    port("validate-commit allows valid conventional commit");
    assert!(reason("WIP save").starts_with("CONVENTIONAL_COMMITS_VIOLATION"));
    port("validate-commit blocks non-conventional commit");
    assert!(!denied(&quoted("feat(auth): add login flow"), &[]));
    port("validate-commit allows a CONFORMING heredoc-form message");
    assert!(
        reason(&quoted("wibble wobble no type here")).starts_with("CONVENTIONAL_COMMITS_VIOLATION")
    );
    port("validate-commit still BLOCKS a non-conforming heredoc-form message");

    // #3811: hooks.commit_types extends the ten built-in types.
    assert!(with_types("enhance(core): x", &[]));
    port("blocks a non-built-in type when commit_types is absent (default list unchanged)");
    port("treats an empty commit_types array as the default list");
    assert!(!with_types("enhance(core): x", &["enhance"]));
    port("accepts a single configured type");
    assert!(with_types("wip: x", &["enhance"]));
    port("still rejects an unconfigured type when only one type is configured");
    let many: Vec<String> = (0..50).map(|i| format!("zz{i}type")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    assert!(
        many.iter()
            .all(|t| !with_types(&format!("{t}(core): x"), &many))
    );
    port("accepts every type in a many-entry commit_types array");
    let listed = super::verdict("wip: x", &["feat", "enhance", "enhance"])
        .reason
        .unwrap();
    assert!(listed.contains("chore, enhance."), "{listed}");
    assert_eq!(listed.matches("feat").count(), 1, "{listed}");
    assert_eq!(listed.matches("enhance").count(), 1, "{listed}");
    port("dedupes a configured type that duplicates a built-in");
    port("dedupes a configured type repeated in its own array");
    let unsafe_types = ["feat|rm -rf /", "a)(b", ".*", "UPPER", ""];
    assert!(!with_types("feat(core): x", &unsafe_types));
    assert!(with_types("UPPER(core): x", &unsafe_types));
    port("rejects unsafe commit_types entries without corrupting the built-in regex");
    assert!(reason("WIP save").contains("chore."));
    assert!(
        super::verdict("WIP save", &["enhance"])
            .reason
            .unwrap()
            .contains("feat, fix, docs, style, refactor, perf, test, build, ci, chore, enhance.")
    );
    port("valid_types includes configured types alongside the built-ins");

    // #3802: the heredoc form, and every shape upstream refuses to resolve.
    let at = |n: usize| format!("feat(auth): {}", "x".repeat(n - 12));
    for (n, deny) in [(71, false), (72, false), (73, true)] {
        assert_eq!(denied(&quoted(&at(n)), &[]), deny, "{n}");
    }
    assert!(
        !denied(&quoted(&format!("{}   ", at(72))), &[]),
        "git strips the trailing blanks"
    );
    assert_eq!(
        reason(&quoted(&format!("{} ", at(73)))),
        "COMMIT_SUBJECT_TOO_LONG: Commit subject must be 72 characters or less."
    );
    port("validate-commit measures subject length against the RESOLVED heredoc subject");
    assert!(denied("$(cat <<'EOF'\nfeat: aaaa", &[]));
    port("validate-commit does not resolve a TRUNCATED capture past its own limit");
    assert!(!denied(
        &quoted("\nfeat(auth): real subject after a blank line"),
        &[]
    ));
    port("validate-commit skips leading blank lines in the heredoc body, as git does");
    for (open, close, indent) in [
        ("<<-'MSG'", "\tMSG", "\t"),
        ("<<'END-MSG'", "END-MSG", ""),
        ("<<\\EOF", "EOF", ""),
    ] {
        assert!(
            !denied(
                &heredoc(
                    &format!("{indent}fix(api): correct status code"),
                    open,
                    close
                ),
                &[]
            ),
            "{open}"
        );
        assert!(
            denied(
                &heredoc(&format!("{indent}wibble wobble"), open, close),
                &[]
            ),
            "{open}"
        );
    }
    port("validate-commit resolves the QUOTED heredoc opener spellings, both directions");
    for open in ["<<EOF", "<< EOF", "<<-EOF"] {
        let r = reason(&heredoc("fix(api): correct status code", open, "EOF"));
        assert!(r.starts_with("CONVENTIONAL_COMMITS_VIOLATION"), "{open}");
    }
    port(
        "validate-commit BLOCKS a bare heredoc delimiter — bash expands that body (round-4 BLOCKER)",
    );
    for body in ["feat: $UNSET_VAR", "feat: ${LONG}"] {
        assert!(denied(&heredoc(body, "<<EOF", "EOF"), &[]), "{body}");
    }
    port("validate-commit BLOCKS an expansion inside a bare-delimiter body (round-4 BLOCKER)");
    for open in ["<<\"EOF\"", "<<'EOF'", "<<\\EOF", "<<EOF", "<< EOF"] {
        let message = heredoc("feat(auth): looks conforming", open, "EOF");
        let command = format!("git commit -m '{message}'");
        assert!(with_command(&message, &command), "{open}");
    }
    port(
        "validate-commit does not resolve a heredoc in the SINGLE-quoted -m arm (round-4 BLOCKER)",
    );
    assert!(!denied(
        &quoted("feat: stop passing -m 'foo'bar to git"),
        &[]
    ));
    port("the adjacency guard is scoped to the arm that matched (round-4 Minor 1)");
    assert!(denied(
        &format!("{} {}", quoted("feat: ok").trim_end(), "a".repeat(200)),
        &[]
    ));
    port("validate-commit blocks a substitution composed with more text (round-3 BLOCKER)");
    assert!(denied(
        &format!("{}{}", quoted("feat: ok").trim_end(), "a".repeat(200)),
        &[]
    ));
    port("validate-commit blocks a suffix glued OUTSIDE the closing quote (adjacency guard)");
    assert!(!denied("feat: okzzz", &[]));
    port("adjacency on a PLAIN single-line message keeps base behavior (pre-existing, unchanged)");
    assert!(denied("$(id;/bin/cat <<'EOF'\nfix: smuggled\nEOF\n)", &[]));
    port("validate-commit blocks a command smuggled before the cat");
    assert!(denied("WIP notes <<EOF\nfix: smuggled subject", &[]));
    port("validate-commit does not treat a message ENDING in <<WORD as a heredoc");
    let body = quoted("feat: accepted body");
    assert!(denied(&format!("WIP first\n\n{body}"), &[]));
    port("validate-commit resolves only git's FIRST message argument (round-4 Codex BLOCKER)");

    // the cleanup mode changes the subject git records
    let padded = format!("feat: {}   ", "x".repeat(66));
    let msg = quoted(&padded);
    let cmd =
        |flags: &str, after: &str| format!("git commit --allow-empty {flags} -m \"{msg}\"{after}");
    assert!(with_command(&msg, &cmd("--cleanup=verbatim", "")));
    assert!(with_command(
        &msg,
        &format!("git -c commit.cleanup=verbatim commit -m \"{msg}\"")
    ));
    assert!(!with_command(&msg, &cmd("--cleanup=whitespace", "")));
    assert!(!with_command(&msg, &cmd("", "")));
    for subject in [
        "fix: document commit.cleanup=strip behavior",
        "docs: explain --cleanup=verbatim in the hook guide",
        "fix: correct the guard scope\n\nIt scanned --cleanup=verbatim in the body.",
    ] {
        let m = quoted(subject);
        assert!(
            !with_command(&m, &format!("git commit -m \"{m}\"")),
            "{subject}"
        );
    }
    assert!(with_command(&msg, &cmd("", " --cleanup=verbatim")));
    port(
        "validate-commit refuses to resolve under a non-default cleanup mode (round-4 Codex BLOCKER)",
    );

    // the options are read the way bash hands them to git (hooks-opt-in.test.cjs "option scans read the command")
    let long72 = format!("feat: {}   ", "x".repeat(66));
    let hd = quoted(&long72);
    let hd_ok = quoted("fix: a perfectly ordinary conforming subject");
    for spelling in ["--cle", "--clea", "--clean", "--cleanu", "--cleanup"] {
        assert!(
            with_command(
                &hd,
                &format!("git commit --allow-empty -m \"{hd}\" {spelling}=verbatim")
            ),
            "{spelling}"
        );
    }
    assert!(!with_command(
        &hd,
        &format!("git commit --allow-empty -m \"{hd}\" --clean=whitespace")
    ));
    assert!(with_command(
        &hd,
        &format!("git commit --allow-empty -m \"{hd}\" --clean\"\"up=verbatim")
    ));
    port("option scans read the command the way bash hands it to git (round-6 accept direction)");
    // `-m A --cleanup=verbatim -m B`: the flag sits between two heredocs
    let two = format!("{hd}\n\n{hd_ok}");
    assert!(with_command(
        &two,
        &format!("git commit -m \"{hd}\" --cleanup=verbatim -m \"{hd_ok}\"")
    ));
    // the wide scan: a separator inside an argument hides nothing
    for author in ["\"a&b\"", "\"a|b\"", "\"a;b\"", "a\\&b", "a\\|b", "a\\;b"] {
        assert!(
            with_command(
                &hd,
                &format!(
                    "git commit --allow-empty -m \"{hd}\" --author {author} --cleanup=verbatim"
                )
            ),
            "{author}"
        );
    }
    assert!(with_command(
        &quoted("fix: a perfectly ordinary conforming subject"),
        &format!("git commit -m \"{hd_ok}\" && echo --cleanup=verbatim")
    ));
    port("the cleanup guard scans wide on purpose — narrowing it reopened a length hole");
    // the last directive is not told from the first: any non-default one refuses
    assert!(with_command(
        &hd,
        &format!("git commit --allow-empty --cleanup=whitespace -m \"{hd}\" --cleanup=verbatim")
    ));
    assert!(!with_command(
        &hd_ok,
        &format!("git commit --cleanup=whitespace -m \"{hd_ok}\"")
    ));
    assert!(with_command(
        &hd,
        &format!("git commit --allow-empty --cleanup=verbatim -m \"{hd}\"")
    ));
    port("more than one cleanup directive is unresolvable — git applies the LAST (round 7)");

    // a commit with no message on the command line, and a command that is no commit, have none to judge
    assert_eq!(validate_commit(&Facts::default(), &[]), HookOutput::allow());
    port("validate-commit allows non-commit commands");

    assert!(denied("$(rm -rf /)", &[]));
    port("validate-commit blocks message with shell metacharacters");
    assert!(denied("`whoami`", &[]));
    port("validate-commit blocks message with backtick injection");
    assert!(!denied("fix(api/v2): handle edge case", &[]));
    port("validate-commit allows commit with scope containing special chars");

    // #4492: a 112 KB message is judged on its subject in time (the subject is read to the first newline).
    let big = "z".repeat(112_000);
    assert!(denied(&format!("wibble {big}"), &[]));
    port("a 112KB message is blocked on its subject, not stalled by the scan");
    assert!(!denied(&format!("feat: ok\n\n{big}"), &[]));
    port("a 112KB conforming message is allowed, not stalled by the scan");

    let out = phase_boundary(&spec_write("write", true));
    assert!(out.context.unwrap().contains("specs/roadmap.spec"));
    port("phase-boundary detects .planning/ writes when enabled");
    ported
}

#[test]
fn hooks_opt_in_ported_cases_hold() {
    assert!(ported().len() > 30);
}

#[test]
fn hooks_opt_in_accounts_for_every_upstream_case() {
    accounting::account(SUITE, &ported(), SKIPPED);
}

const SKIPPED: &[(&str, &str)] = &[
    (
        "hook file validation",
        "installer: the upstream hook files, their modes and the installer's lists; `specforge install` pins what it writes (tests/cli/install_hosts.rs)",
    ),
    (
        "installer hook registration",
        "installer: the upstream hook files, their modes and the installer's lists; `specforge install` pins what it writes (tests/cli/install_hosts.rs)",
    ),
    (
        "opt-in gating behavior",
        "runtime: the opt-in is the hook's `when` (hooks.community), tested black-box in tests/cli/hooks_community.rs",
    ),
    (
        "hook execution when enabled > hooks.commit_types config surface (#3811) > drops non-string",
        "runtime: hooks.commit_types is a typed list of text; the kernel refuses another shape (E086) before a hook runs",
    ),
    (
        "hook execution when enabled > hooks.commit_types config surface (#3811) > ignores a non-array",
        "runtime: hooks.commit_types is a typed list of text; the kernel refuses another shape (E086) before a hook runs",
    ),
    (
        "hook execution when enabled > hooks.commit_types config surface (#3811) > a backslash-glued",
        "runtime: a bash-version loop over the script's `-m` capture; this port has no capture regex",
    ),
    (
        "hook execution when enabled > hooks.commit_types config surface (#3811) > a letter-glued",
        "runtime: a bash-version loop over the script's `-m` capture; this port has no capture regex",
    ),
    (
        "hook execution when enabled > hooks.commit_types config surface (#3811) > a backslash before",
        "runtime: a bash-version loop over the script's `-m` capture; this port has no capture regex",
    ),
    (
        "hook execution when enabled > hooks.commit_types config surface (#3811) > a non-conforming subject",
        "runtime: a bash-version loop over the script's `-m` capture; this port has no capture regex",
    ),
    (
        "hook execution when enabled > KNOWN LIMIT",
        "diverges: upstream's capture cannot deliver the <<\"TAG\" body and blocks it; the kernel's words can, so this port resolves it (more permissive, and what git records)",
    ),
    (
        "hook execution when enabled > validate-commit leaves every non-heredoc form",
        "diverges: upstream's regex needs a quoted -m, so an unquoted `-m WIP` and `-m \"\" -m \"fix: x\"` pass; the kernel hands a message for both and this port judges it (stricter)",
    ),
    (
        "hook execution when enabled > the adjacency guard is scoped to the span",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > the two reported chained-before",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > the first-message guard recognises",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > option-name scans",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > an option NAME carrying",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > the round-9 class",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > a backslash-newline continuation",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > a newline is a command separator",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > a git-GENERATED subject",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > subject extraction uses no pipe-to-head",
        "runtime: the script's `head -1` SIGPIPE race; this port has no pipe",
    ),
    (
        "hook execution when enabled > validate-commit does not trust a relative path ending in cat",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > an ABSOLUTE path is not an identity",
        "runtime: how bash hands words to git (option scans, continuations, expansions in option names); the kernel's `facts::commit` reads the words, and the hook is a function of the message and the raw command",
    ),
    (
        "hook execution when enabled > session-state",
        "runtime: session-state is a hook of the computed state, tested black-box (tests/cli/hooks_community.rs); upstream's case reads STATE.md",
    ),
    (
        "hook execution when enabled > phase-boundary detects .planning/ writes from Kimi",
        "runtime: Kimi has no renderer here",
    ),
    (
        "hook execution when enabled > phase-boundary prefers Kimi",
        "runtime: Kimi has no renderer here",
    ),
    (
        "hook execution when enabled > phase-boundary negative control: Kimi",
        "runtime: Kimi has no renderer here",
    ),
    (
        "hook security tests > phase-boundary handles malformed",
        "runtime: a malformed payload or config never reaches a hook (the kernel reads the event and the settings, K9)",
    ),
    (
        "hook security tests > hooks handle config.json with broken JSON",
        "runtime: a malformed payload or config never reaches a hook (the kernel reads the event and the settings, K9)",
    ),
    (
        "validate-commit: a large -m message is validated in bounded time (#4492) > the suffix window",
        "runtime: the script's bounded scan window for options after a large command; this port has no scan window",
    ),
];
