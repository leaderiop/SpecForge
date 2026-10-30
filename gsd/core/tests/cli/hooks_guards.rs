//! The read, write and secret-read guards through the command the install wrote (D-r44-16): P1 (an extension hook
//! answers through `specforge hook`, timed against `core.spec_guard`) and each guard on the hosts that render it.

use std::time::Instant;

use serde_json::json;

use super::harness::{Project, taskboard};
use super::hooks_support::{
    HOSTS, Home, all_commands, has_command, install_in, installed_command, parses, run_hook,
    run_installed, tool_event, tool_name, verdict,
};

/// The twin with every host installed.
fn installed() -> (Project, Home) {
    let p = taskboard();
    let home = Home::new();
    for host in HOSTS {
        install_in(&p, host, &home);
    }
    (p, home)
}

/// A Write of `spec/plans.spec` (8,424 bytes on the twin) with this `content`.
fn write_event(p: &Project, host: &str, content: &str) -> String {
    let input = json!({"file_path": p.root().join("spec/plans.spec"), "content": content});
    tool_event(p, host, "pre", tool_name(host, "write"), &input, None)
}

/// P1: an extension's hook answers through `specforge hook gsd.<id>`, the line the install wrote.
#[test]
fn a_gsd_hook_answers_through_specforge_hook() {
    let (p, home) = installed();
    for host in HOSTS {
        let run = run_hook(
            &p,
            host,
            "gsd.write_guard",
            &write_event(&p, host, ""),
            &home,
        );
        let (denied, reason, _) = verdict(host, &run);
        assert!(denied, "{host}: {}", run.all());
        assert!(reason.contains("gsd.write_guard"), "{host}: {reason}");
        assert!(reason.contains("plans.spec"), "{host}: {reason}");
    }
}

fn percentile(sorted: &[f64], pct: usize) -> f64 {
    sorted[(sorted.len() * pct / 100).min(sorted.len() - 1)]
}

/// D-r44-6: a gsd hook costs at most twice `core.spec_guard` (and at most a second). Interleaved, so load moves both.
#[test]
fn a_gsd_hook_is_within_the_latency_budget() {
    let (p, home) = installed();
    let event = write_event(&p, "claude-code", "");
    let gsd = installed_command(&p, "claude-code", "gsd.write_guard");
    let spec = installed_command(&p, "claude-code", "core.spec_guard");
    let (mut a, mut b) = (Vec::new(), Vec::new());
    run_installed(&p, &gsd, &event, &home);
    run_installed(&p, &spec, &event, &home);
    for _ in 0..20 {
        let t = Instant::now();
        let run = run_installed(&p, &gsd, &event, &home);
        a.push(t.elapsed().as_secs_f64());
        assert_eq!(run.code, 0, "{}", run.all());
        let t = Instant::now();
        run_installed(&p, &spec, &event, &home);
        b.push(t.elapsed().as_secs_f64());
    }
    a.sort_by(f64::total_cmp);
    b.sort_by(f64::total_cmp);
    eprintln!(
        "gsd.write_guard p50 {:.3}s p95 {:.3}s; core.spec_guard p50 {:.3}s p95 {:.3}s",
        percentile(&a, 50),
        percentile(&a, 95),
        percentile(&b, 50),
        percentile(&b, 95)
    );
    assert!(percentile(&a, 50) <= 1.0, "p50 over a second: {a:?}");
    assert!(
        percentile(&a, 50) <= 2.0 * percentile(&b, 50),
        "over twice core.spec_guard: {a:?} against {b:?}"
    );
}

const GUARDS: [&str; 3] = ["gsd.read_guard", "gsd.write_guard", "gsd.secret_read_guard"];

#[test]
fn the_guards_are_installed_on_three_hosts_beside_the_spec_guard() {
    let (p, _home) = installed();
    for host in HOSTS {
        for route in GUARDS.into_iter().chain(["core.spec_guard"]) {
            assert!(has_command(&p, host, route), "{host}: {route}");
        }
        for line in all_commands(&p, host) {
            assert!(parses(&line), "{host}: {line}");
            assert!(line.ends_with("# specforge-install"), "{host}: {line}");
        }
    }
    let check = p.run(&["check", "--format", "json"]);
    assert!(
        !check.stdout.contains("W021"),
        "a hook the registry refused: {}",
        check.stdout
    );
}

/// A Write of a spec file that keeps most of its bytes is allowed, on every host.
#[test]
fn a_write_that_keeps_the_file_is_allowed() {
    let (p, home) = installed();
    let kept = p.read("spec/plans.spec");
    for host in HOSTS {
        let run = run_hook(
            &p,
            host,
            "gsd.write_guard",
            &write_event(&p, host, &kept),
            &home,
        );
        assert_eq!(
            verdict(host, &run),
            (false, String::new(), String::new()),
            "{host}: {}",
            run.all()
        );
    }
}

#[test]
fn read_guard_is_silent_on_claude_code() {
    let (p, home) = installed();
    let input = json!({"file_path": p.root().join("spec/plans.spec"), "content": "x"});
    let event = tool_event(&p, "claude-code", "pre", "Write", &input, None);
    let run = run_hook(&p, "claude-code", "gsd.read_guard", &event, &home);
    assert_eq!(run.stdout, "", "{}", run.all());
}

#[test]
fn read_guard_warns_on_codex_and_cursor() {
    let (p, home) = installed();
    for host in ["codex", "cursor"] {
        let input = json!({"file_path": p.root().join("spec/plans.spec"), "content": "x"});
        let event = tool_event(&p, host, "pre", tool_name(host, "write"), &input, None);
        let run = run_hook(&p, host, "gsd.read_guard", &event, &home);
        let (denied, _, context) = verdict(host, &run);
        assert!(!denied, "{host}: advice never blocks");
        assert!(
            context.starts_with("READ-BEFORE-EDIT REMINDER"),
            "{host}: {context}"
        );
        assert!(context.contains("\"plans.spec\""), "{host}: {context}");
    }
    let new = json!({"file_path": p.root().join("spec/new.spec"), "content": "x"});
    let event = tool_event(&p, "codex", "pre", "Write", &new, None);
    assert_eq!(
        run_hook(&p, "codex", "gsd.read_guard", &event, &home).stdout,
        "",
        "a new file"
    );
}

fn shell_event(p: &Project, host: &str, command: &str) -> String {
    tool_event(
        p,
        host,
        "pre",
        tool_name(host, "shell"),
        &json!({"command": command}),
        None,
    )
}

#[test]
fn secret_read_guard_denies_cat_env_on_three_hosts() {
    let (p, home) = installed();
    for host in HOSTS {
        let run = run_hook(
            &p,
            host,
            "gsd.secret_read_guard",
            &shell_event(&p, host, "cd app && cat .env"),
            &home,
        );
        let (denied, reason, _) = verdict(host, &run);
        assert!(denied, "{host}: {}", run.all());
        assert!(
            reason.contains("gsd.secret_read_guard") && reason.contains("would read '.env'"),
            "{host}: {reason}"
        );
        assert!(
            reason.contains("(.env.example)"),
            "{host}: the template is named"
        );
    }
}

#[test]
fn an_example_env_file_is_read() {
    let (p, home) = installed();
    for host in HOSTS {
        let run = run_hook(
            &p,
            host,
            "gsd.secret_read_guard",
            &shell_event(&p, host, "cat .env.example"),
            &home,
        );
        assert_eq!(run.stdout, "", "{host}");
    }
    let read = |path: &str| {
        let event = tool_event(
            &p,
            "claude-code",
            "pre",
            "Read",
            &json!({"file_path": path}),
            None,
        );
        run_hook(&p, "claude-code", "gsd.secret_read_guard", &event, &home)
    };
    assert_eq!(read(".env.example").stdout, "");
    assert!(verdict("claude-code", &read("config/.env.production")).0);
}

#[test]
fn secret_read_guard_judges_a_grep_by_path_and_glob() {
    let (p, home) = installed();
    let grep = |input: serde_json::Value| {
        let event = tool_event(&p, "claude-code", "pre", "Grep", &input, None);
        verdict(
            "claude-code",
            &run_hook(&p, "claude-code", "gsd.secret_read_guard", &event, &home),
        )
    };
    assert!(grep(json!({"pattern": "KEY", "path": ".secrets"})).0);
    assert!(grep(json!({"pattern": "KEY", "glob": "{*.rs,.env*}"})).0);
    assert!(!grep(json!({"pattern": "KEY", "glob": "*.rs"})).0);
    let event = tool_event(
        &p,
        "cursor",
        "pre",
        "Grep",
        &json!({"pattern": "KEY", "path": ".env"}),
        None,
    );
    assert!(
        verdict(
            "cursor",
            &run_hook(&p, "cursor", "gsd.secret_read_guard", &event, &home)
        )
        .0
    );
}

/// Each of the three guards on each host, once, as the install wired it: exit 0 and the host's answer shape.
#[test]
fn every_guard_answers_on_every_host() {
    let (p, home) = installed();
    for host in HOSTS {
        for route in GUARDS {
            let event = write_event(&p, host, "x");
            let run = run_hook(&p, host, route, &event, &home);
            assert_eq!(run.code, 0, "{host} {route}: {}", run.all());
            assert!(
                run.stdout.is_empty()
                    || serde_json::from_str::<serde_json::Value>(&run.stdout).is_ok(),
                "{host} {route}: {}",
                run.stdout
            );
        }
    }
}
