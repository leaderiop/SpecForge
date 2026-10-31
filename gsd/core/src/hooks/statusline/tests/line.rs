//! Ported from gsd-core v1.18.0 tests/gsd-statusline.test.cjs, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The meter, the token count, the model badge and the layout.

use serde_json::{Value, json};

use super::*;

/// The meter, the token count, the model badge and the layout.
pub(super) fn line_ported(port: &mut dyn FnMut(&str)) {
    assert_eq!(used_at(50.0), 52);
    port(
        "context meter: 100% is the auto-compact threshold (#2219, #4985) > no window configured: 100% is the model window minus the 33k auto-compact buffer",
    );
    let m = context_meter(&json!({"context_window_size": 1_000_000, "remaining_percentage": 50}));
    assert_eq!(m.map(|m| m.used), Some(52));
    assert_eq!(used_at(50.0), 52);
    port(
        "context meter boundary: acw at/near totalCtx does not pin used at 100% (#1194) > acw = 0 (default path): 52% for 50% remaining",
    );
    assert!((7..=10).contains(&used_at(93.0)), "{}", used_at(93.0));
    port(
        "context meter boundary: acw at/near totalCtx does not pin used at 100% (#1194) > normal partial value: 93% remaining → ~7% used",
    );
    // used + remaining = 100, both whole and in 0..=100, against the threshold; never falling as tokens grow
    for size in [1_u64, 50_000, 200_000, 1_000_000, 3_000_000] {
        let mut last = 0;
        for tokens in [0_u64, 1, 999, 100_000, 500_000, 967_000, 5_000_000] {
            let usage = json!({"input_tokens": tokens, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0});
            let m = context_meter(&json!({"context_window_size": size, "current_usage": usage}))
                .expect("a meter");
            let limit = m.limit_tokens.expect("a limit");
            let threshold = size as f64 - 33_000.0;
            assert_eq!(
                limit,
                if threshold > 0.0 {
                    threshold
                } else {
                    size as f64
                }
            );
            assert!((0..=100).contains(&m.used) && m.used + m.remaining == 100);
            let pct = tokens as f64 / limit * 100.0;
            assert!(if pct >= 100.0 {
                m.used == 100
            } else {
                (m.used as f64 - pct).abs() <= 0.5
            });
            assert_eq!(m.used_tokens, Some(tokens as f64));
            assert!(m.used >= last);
            last = m.used;
        }
    }
    port(
        "auto-compact window and context meter: properties (#4985) > contextMeter: used + remaining = 100, both integers in 0–100, against the threshold or the model window",
    );
    // no window size: Claude Code's own percentage, unscaled; nothing at all: no meter
    let m = context_meter(&json!({"remaining_percentage": 35})).expect("a meter");
    assert_eq!(
        (m.used, m.remaining, m.used_tokens, m.limit_tokens),
        (65, 35, None, None)
    );
    assert!(context_meter(&json!({"context_window_size": 1_000_000, "current_usage": null, "remaining_percentage": null})).is_none());

    // the bridge: the line writes the meter to the session for the monitor
    let f = Facts {
        host: "claude-code".into(),
        session: Some("s1".into()),
        now: "2026-10-30T10:00:00Z".into(),
        status: Some(status(
            json!("Claude"),
            Some(1_000_000),
            Some(50.0),
            Value::Null,
        )),
        ..Facts::default()
    };
    let patch = statusline(&f, &opts("end", false, false))
        .state
        .and_then(|s| s.session)
        .expect("a patch");
    assert_eq!(
        patch["context"],
        json!({"remaining_percentage": 48, "used_pct": 52, "used_tokens": 500_000.0, "threshold_tokens": 967_000.0,
               "at": "2026-10-30T10:00:00Z"})
    );
    port(
        "context meter: 100% is the auto-compact threshold (#2219, #4985) > bridge is on the bar's scale and carries the /context token counts (#2451, #4985)",
    );

    // the layout
    let ctx = " \x1b[32m████░░░░░░ 40%\x1b[0m";
    let l = |position: &'static str,
             ctx: &'static str,
             middle: Option<&'static str>,
             dirname: &'static str| {
        compose(&Line {
            model: "Claude",
            ctx,
            middle,
            dirname,
            position,
            ..Line::default()
        })
    };
    let out = l("", ctx, None, "myproject");
    assert!(out.find(ctx) > out.find("myproject"));
    port("default (no position arg) renders ctx at tail — end layout");
    assert_eq!(
        l("end", ctx, None, "myproject"),
        l("", ctx, None, "myproject")
    );
    port("explicit \"end\" is byte-identical to default");
    let out = l("end", ctx, Some("doing work"), "proj");
    assert!(out.find(ctx) > out.find("proj"));
    port("\"end\" with middle segment places ctx after dirname");
    let out = l("front", ctx, None, "myproject");
    assert!(out.find(ctx).is_some() && out.find(ctx) < out.find(" │ "));
    port("\"front\" puts ctx after model name, before first │");
    let out = l("front", ctx, Some("doing work"), "proj");
    assert!(out.find(ctx) < out.find(" │ "));
    port("\"front\" with middle segment: ctx after model, before first │");
    let out = l("front", "", None, "myproject");
    assert!(!out.contains(" │  │ ") && out.contains(" │ "));
    port("empty ctx + \"front\" renders no stray separator");
    assert!(!l("end", "", None, "myproject").contains(" │  │ "));
    port("empty ctx + \"end\" renders no stray separator");
    assert_eq!(
        l("middle", ctx, None, "myproject"),
        l("end", ctx, None, "myproject")
    );
    port("invalid position value silently falls back to \"end\" layout");
    assert_eq!(l("banana", ctx, None, "proj"), l("end", ctx, None, "proj"));
    port("invalid position \"banana\" silently falls back to \"end\"");
    let update = "\x1b[33m⬆ /gsd:update\x1b[0m │ ";
    let mut line = Line {
        update,
        model: "Claude",
        dirname: "proj",
        position: "end",
        ..Line::default()
    };
    assert!(compose(&line).starts_with(update));
    port("gsdUpdate warning is leftmost in \"end\" mode");
    line.position = "front";
    line.ctx = ctx;
    assert!(compose(&line).starts_with(update));
    port("gsdUpdate warning is leftmost in \"front\" mode");
    let git = " │ main✓";
    let end = compose(&Line {
        model: "Claude",
        dirname: "proj",
        git,
        ctx,
        position: "end",
        ..Line::default()
    });
    assert!(end.find(git) > end.find("proj"));
    port(
        "composeStatusline gitSuffix placement > git segment renders after the directory in end layout",
    );
    let front = compose(&Line {
        model: "Claude",
        dirname: "proj",
        git,
        ctx,
        position: "front",
        ..Line::default()
    });
    assert!(front.find(git) > front.find("proj"));
    port(
        "composeStatusline gitSuffix placement > git segment renders after the directory in front layout",
    );
    let none = compose(&Line {
        model: "Claude",
        dirname: "proj",
        ctx,
        ..Line::default()
    });
    assert!(!none.contains("main"));
    port("composeStatusline gitSuffix placement > default (no gitSuffix) output is unchanged");
    let st = status(
        json!("Claude"),
        Some(200_000),
        Some(70.0),
        json!({"input_tokens": 1000, "cache_read_input_tokens": 150_000, "output_tokens": 5000}),
    );
    assert!(!line_of(st, &opts("end", false, false)).contains("main"));
    port("show_git e2e through the hook > default (flag absent) has no git segment");
    git_ported(port);

    // the keys the settings declare
    let keys: Vec<&str> = crate::settings::key_names().collect();
    for (key, title) in [
        (
            "statusline.context_position",
            "config schema registers statusline.context_position",
        ),
        (
            "statusline.show_context_tokens",
            "config schema: statusline.show_context_tokens > registers statusline.show_context_tokens",
        ),
        (
            "statusline.state_format",
            "config schema: statusline.state_format > registers statusline.state_format",
        ),
        (
            "statusline.show_git",
            "config schema: statusline.show_git > registers statusline.show_git",
        ),
        (
            "statusline.show_last_command",
            "config schema registers statusline.show_last_command",
        ),
    ] {
        assert!(keys.contains(&key), "{key}");
        port(title);
    }

    // the token count
    assert_eq!(
        (format_tokens(0.0), format_tokens(999.0)),
        ("0".into(), "999".into())
    );
    port("formatTokens > passes small counts through");
    assert_eq!(
        (
            format_tokens(1000.0),
            format_tokens(156_342.0),
            format_tokens(156_700.0)
        ),
        ("1k".into(), "156k".into(), "157k".into())
    );
    port("formatTokens > rounds thousands to k");
    assert_eq!(
        (format_tokens(1_000_000.0), format_tokens(1_234_567.0)),
        ("1.0M".into(), "1.2M".into())
    );
    port("formatTokens > formats millions with one decimal");
    for (n, want) in [
        (999_999.0, "1.0M"),
        (1_000_000.0, "1.0M"),
        (1_000_001.0, "1.0M"),
        (999_499.0, "999k"),
        (999_500.0, "1.0M"),
    ] {
        assert_eq!(format_tokens(n), want, "{n}");
    }
    port("formatTokens > k-to-M threshold boundary: limit-1 / limit / limit+1");
    for u in [None, Some(&json!("nope")), Some(&json!({}))] {
        assert_eq!(token_suffix(u), "");
    }
    port("contextTokenSuffix > returns empty string for absent/malformed usage");
    let all = json!({"input_tokens": 1000, "cache_creation_input_tokens": 2000, "cache_read_input_tokens": 150_000, "output_tokens": 3000});
    assert_eq!(token_suffix(Some(&all)), " (156k)");
    port("contextTokenSuffix > sums all four token dimensions");
    assert_eq!(token_suffix(Some(&json!({"input_tokens": 500}))), " (500)");
    port("contextTokenSuffix > tolerates missing dimensions");
    let st = status(
        json!("Claude"),
        Some(200_000),
        Some(70.0),
        json!({"input_tokens": 1000, "cache_read_input_tokens": 150_000, "output_tokens": 5000}),
    );
    assert!(line_of(st.clone(), &opts("end", false, true)).contains("% (156k)"));
    port(
        "statusline output token suffix (e2e) > flag=true appends the token count after the percentage",
    );
    let no = line_of(st.clone(), &opts("end", false, false));
    assert!(!no.contains("(156k)") && !no.contains("k)"), "{no}");
    port(
        "statusline output token suffix (e2e) > default (flag absent) meter is unchanged — no token count",
    );
    port("statusline output token suffix (e2e) > flag=false meter is unchanged — no token count");

    // the model badge
    assert_eq!(compact_model("Sonnet 4.5 (1M context)"), "Sonnet 4.5 (1M)");
    port("compactModelName > collapses \"(1M context)\" to \"(1M)\"");
    assert_eq!(compact_model("Opus 4.6  (1m CONTEXT)"), "Opus 4.6 (1m)");
    port("compactModelName > is case-insensitive on \"context\" and preserves the token case");
    assert_eq!(compact_model("Sonnet 5 (500K context)"), "Sonnet 5 (500K)");
    assert_eq!(compact_model("Opus 5 (2M context)"), "Opus 5 (2M)");
    port("compactModelName > tolerates future window sizes (#2160 approval condition)");
    assert_eq!(compact_model("Sonnet 4.5 (1M ctx)"), "Sonnet 4.5 (1M)");
    assert_eq!(compact_model("Opus 5 (2M CTX)"), "Opus 5 (2M)");
    port("compactModelName > handles the abbreviated \"ctx\" variant (#2160 approval condition)");
    assert_eq!(compact_model("Sonnet 5 (beta)"), "Sonnet 5 (beta)");
    assert_eq!(compact_model("Opus 4 (deprecated)"), "Opus 4 (deprecated)");
    port("compactModelName > leaves non-context parentheticals untouched");
    assert_eq!(compact_model("Claude"), "Claude");
    assert_eq!(compact_model("Sonnet 4.5"), "Sonnet 4.5");
    port("compactModelName > leaves ordinary names unchanged");
    assert_eq!(
        compact_model("(1M context) Special"),
        "(1M context) Special"
    );
    port("compactModelName > only matches the suffix position");
    // a model that is not a string is the default name
    let out = line_of(
        status(Value::Null, None, None, Value::Null),
        &opts("end", false, false),
    );
    assert!(out.starts_with("Claude │ "), "{out}");
    port("compactModelName > passes non-strings through");
    let out = line_of(
        status(json!("Sonnet 4.5 (1M context)"), None, None, Value::Null),
        &opts("end", false, false),
    );
    assert!(
        out.contains("Sonnet 4.5 (1M)") && !out.contains("(1M context)"),
        "{out}"
    );
    port(
        "renderStatusline uses the compact badge > rendered output carries (1M), not (1M context)",
    );
}

/// `parseGitStatus` and `buildGitSegment` over the raw status text, and the flag through the hook.
fn git_ported(port: &mut dyn FnMut(&str)) {
    use crate::hooks::facts::Git;
    use crate::hooks::statusline::format::{GitInfo, git_segment, parse_git_status};

    let info = |branch: &str, ahead, behind, staged, unstaged, untracked| GitInfo {
        branch: branch.into(),
        ahead,
        behind,
        staged,
        unstaged,
        untracked,
    };
    assert_eq!(parse_git_status(""), None);
    assert_eq!(parse_git_status("? some-file\n"), None);
    port("parseGitStatus > returns null for non-string / missing branch header");
    let clean = "# branch.oid abc123\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +0 -0\n";
    assert_eq!(parse_git_status(clean), Some(info("main", 0, 0, 0, 0, 0)));
    port("parseGitStatus > parses a clean, in-sync branch");
    let dirty = "# branch.oid abc123\n# branch.head feat/x\n# branch.upstream origin/feat/x\n# branch.ab +2 -1\n\
        1 M. N... 100644 100644 100644 aaa bbb staged-only.txt\n1 .M N... 100644 100644 100644 aaa bbb unstaged-only.txt\n\
        1 MM N... 100644 100644 100644 aaa bbb both.txt\n2 R. N... 100644 100644 100644 aaa bbb R100 new.txt\told.txt\n\
        ? untracked-1.txt\n? untracked-2.txt\n";
    assert_eq!(parse_git_status(dirty), Some(info("feat/x", 2, 1, 3, 2, 2)));
    port("parseGitStatus > counts staged, unstaged, untracked, ahead, behind");
    let conflict = parse_git_status(
        "# branch.head main\nu UU N... 100644 100644 100644 100644 aaa bbb ccc conflict.txt\n",
    )
    .expect("a branch");
    assert_eq!((conflict.unstaged, conflict.staged), (1, 0));
    port("parseGitStatus > counts unmerged (conflict) entries as unstaged");
    assert_eq!(
        parse_git_status("# branch.head (detached)\n").map(|i| i.branch),
        Some("(detached)".to_string())
    );
    port("parseGitStatus > detached HEAD passes through as \"(detached)\"");
    let local = parse_git_status("# branch.head local-only\n? new.txt\n").expect("a branch");
    assert_eq!((local.ahead, local.behind, local.untracked), (0, 0, 1));
    port("parseGitStatus > no upstream (no branch.ab line) leaves ahead/behind at 0");

    assert_eq!(git_segment(None), "");
    assert_eq!(git_segment(Some(&GitInfo::default())), "");
    port("buildGitSegment > returns empty string for null info");
    assert_eq!(
        git_segment(Some(&info("main", 0, 0, 0, 0, 0))),
        " \u{2502} main\u{2713}"
    );
    port("buildGitSegment > clean repo renders branch with a check mark");
    assert_eq!(
        git_segment(Some(&info("feat/x", 2, 1, 3, 2, 4))),
        " \u{2502} feat/x+3~2?4\u{2191}2\u{2193}1"
    );
    port("buildGitSegment > dirty repo renders each nonzero marker in order");
    assert_eq!(
        git_segment(Some(&info("main", 1, 0, 0, 0, 0))),
        " \u{2502} main\u{2191}1"
    );
    port("buildGitSegment > omits zero markers");

    // the flag through the hook: the segment follows the directory, and only while `show_git` is on
    let facts = |text: &str| Facts {
        host: "claude-code".into(),
        status: Some(status(json!("Claude"), None, None, Value::Null)),
        git: Some(Git {
            toplevel: "/t/myproject".into(),
            git_dir: "/t/myproject/.git".into(),
            branch: "main".into(),
            status: Some(text.into()),
        }),
        ..Facts::default()
    };
    let on = Options {
        show_git: true,
        ..opts("end", false, false)
    };
    let line = |f: &Facts, o: &Options| plain(&statusline(f, o).statusline.expect("a line"));
    let f = facts("# branch.head main\n1 A. N... 0 0 0 a b x\n? y\n");
    assert!(
        line(&f, &on).contains("myproject \u{2502} main+1?1"),
        "{}",
        line(&f, &on)
    );
    assert!(!line(&f, &opts("end", false, false)).contains("main"));
    assert!(
        !line(
            &Facts {
                git: None,
                ..facts("")
            },
            &on
        )
        .contains(" \u{2502} main")
    );
    port("show_git e2e through the hook > flag=true renders the branch segment");
}
