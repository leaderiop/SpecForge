//! `settings` and `config` (T6): a value is written by the host's own `settings set`, never by the prompt.

use super::commands_support::twin;
use super::prompts_support::{command, payload, prompt, render_three_hosts, text_of};

fn text(p: &super::harness::Project, id: &str, args: &[&str]) -> String {
    text_of(&payload(&prompt(p, id, args, "claude-code")))
}

#[test]
fn settings_prompt_names_settings_set() {
    let p = twin();
    for id in ["settings", "config"] {
        let text = text(&p, id, &[]);
        assert!(text.contains("specforge settings set @gsd/core"), "{id}");
        assert!(!text.contains("settings set gsd "), "{id}");
    }
    // P6: the line the text gives for a toggle, run with `false`.
    let text = text(&p, "settings", &[]);
    let line = command(
        &text,
        "  specforge settings set @gsd/core <key> \"<value>\"",
        "",
        &[("<key>", "workflow.research"), ("<value>", "false")],
    );
    let refs: Vec<&str> = line.iter().map(String::as_str).collect();
    let run = p.run(&refs);
    assert_eq!(run.code, 0, "{line:?}: {}", run.all());
    let shown = p.run(&["settings", "--format", "json", "--extension", "@gsd/core"]);
    assert_eq!(shown.code, 0, "{}", shown.all());
    let shown: serde_json::Value = serde_json::from_str(&shown.stdout).expect("JSON");
    assert!(shown.to_string().contains("workflow.research"), "{shown}");
    let block =
        super::prompts_support::text_of(&payload(&prompt(&p, "settings", &[], "claude-code")));
    assert!(
        block
            .lines()
            .any(|l| l.contains("workflow.research") && l.contains("false")),
        "the settings block does not show workflow.research false"
    );
}

#[test]
fn config_profile_names_model_profile() {
    let p = twin();
    let text = text(&p, "config", &["--profile", "budget"]);
    assert!(text.contains("--profile \"budget\""), "{text}");
    assert!(
        text.contains("specforge settings set @gsd/core model_profile budget"),
        "no model_profile line"
    );
}

#[test]
fn a_setting_of_another_extension_is_refused() {
    let p = twin();
    let run = p.run(&["settings", "set", "@gsd/core", "workflow.ui_phase", "false"]);
    assert_ne!(run.code, 0, "{}", run.all());
    let text = text(&p, "config", &[]);
    assert!(
        text.contains("specforge settings set @gsd/ui workflow.ui_phase false"),
        "the text does not name @gsd/ui"
    );
}

#[test]
fn config_integrations_never_sets_a_secret() {
    let p = twin();
    let text = text(&p, "config", &["--integrations"]);
    for word in ["BRAVE_API_KEY", "FIRECRAWL_API_KEY", "EXA_API_KEY"] {
        assert!(text.contains(word), "no {word}");
    }
    for line in text.lines().filter(|l| l.contains("settings set")) {
        for secret in ["brave_search \"", "firecrawl \"", "exa_search \""] {
            assert!(!line.contains(secret), "sets a secret: {line}");
        }
    }
}

#[test]
fn settings_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(&p, "settings", &[], "", "Interactive configuration");
}

#[test]
fn config_renders_for_three_hosts() {
    let p = twin();
    for args in [
        vec![],
        vec!["--advanced"],
        vec!["--integrations"],
        vec!["--profile", "quality"],
    ] {
        let argument_text = args.join(" ").replace("--profile ", "--profile \"")
            + if args.len() > 1 { "\"" } else { "" };
        render_three_hosts(
            &p,
            "config",
            &args,
            &argument_text,
            "Configure GSD settings",
        );
    }
}
