//! The phase labels the lifecycle prompts print: where each prompt reads a phase's number and name, and every
//! placeholder a label holds is defined in the asset that prints it.

use std::collections::BTreeSet;
use std::path::Path;

use gsd_core::prompts::{asset, render_text};
use regex::Regex;

#[path = "common/files.rs"]
mod files;
#[path = "common/prompt_combos.rs"]
mod prompt_combos;

use prompt_combos::prompts;

/// A phase label as an asset prints it: `Phase` in any case, not after a word character, a `$` or a `-` (so neither a
/// command such as `/gsd:plan-phase {X}` nor a branch template), then a `{placeholder}` (`{Z+1}` reads as `{Z}`), an
/// optional `/{total}`, and a second placeholder after `:`, `—`, `–`, `(` or `⏭`. A `${VARIABLE}` is the shell's: the
/// asset's own code sets it, so it is not read here.
const LABEL: &str =
    r"(?im)(?:^|[^\w$-])phase \{(\w+)(?:\+1)?\}(?:/\{\w+\})?(?:\s*(?::|—|–|\(|⏭)\s*\{(\w+)\})?";

/// A definition: backticked names joined by `, ` or ` and `, then `is` or `are`, wrapped or not (`` `{X}` is ``,
/// `` `{X}`, `{N}` and `PHASE` are ``).
const DEFINITION: &str = r"(?:`[^`\n]+`(?:,\s+|\s+and\s+))*`[^`\n]+`\s+(?:is|are)\b";

/// The placeholders each label of `text` holds, as `(line, name)`.
fn labels(text: &str) -> Vec<(usize, String)> {
    let label = Regex::new(LABEL).expect("the label pattern");
    text.lines()
        .enumerate()
        .flat_map(|(n, line)| {
            label
                .captures_iter(line)
                .flat_map(|c| [c.get(1), c.get(2)])
                .flatten()
                .map(|m| (n + 1, m.as_str().to_string()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The names `text` defines: each backticked name of a definition, its braces and `$` dropped.
fn defined(text: &str) -> BTreeSet<String> {
    let definition = Regex::new(DEFINITION).expect("the definition pattern");
    let name = Regex::new(r"`([^`\n]+)`").expect("a name pattern");
    definition
        .find_iter(text)
        .flat_map(|d| name.captures_iter(d.as_str()).map(|c| c[1].to_string()))
        .map(|n| {
            n.trim_start_matches('$')
                .trim_matches(['{', '}'])
                .to_string()
        })
        .collect()
}

/// Every asset of `@gsd/core` and `@gsd/security` as `(tree, path in it, text)`.
fn assets() -> Vec<(String, String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut all = Vec::new();
    for tree in [root.join("assets"), root.join("../security/assets")] {
        for path in files::under(&tree) {
            if path.extension().is_some_and(|ext| ext == "md") {
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let rel = path.strip_prefix(&tree).expect("under its tree");
                all.push((tree.display().to_string(), rel.display().to_string(), text));
            }
        }
    }
    all
}

#[test]
fn a_label_and_a_definition_read_as_written() {
    let names = |text: &str| -> Vec<String> { labels(text).into_iter().map(|(_, n)| n).collect() };
    assert_eq!(names("**Phase {X}: {Name}** — {N} plan(s)"), ["X", "Name"]);
    assert_eq!(
        names("GSD ► AUTONOMOUS ▸ Phase {N}/{T}: {Name}"),
        ["N", "Name"]
    );
    assert_eq!(
        names("## ✓ Phase {Z} Complete, then Phase {Z+1}"),
        ["Z", "Z"]
    );
    assert_eq!(names("Log \"Phase {N} ⏭ {Name} — Skipped\""), ["N", "Name"]);
    assert_eq!(names("PLANNING PHASE {phase_number}"), ["phase_number"]);
    for none in [
        "/gsd:plan-phase {X}",
        "gsd/phase-{phase}-{slug}",
        "Phase ${PHASE}: ${PHASE_NAME}",
    ] {
        assert!(names(none).is_empty(), "{none}");
    }
    let said = "`{X}`, `{phase_number}` and `{PHASE_NUMBER}` are the phase's number, and `{Name}` is its name";
    assert_eq!(
        defined(said),
        BTreeSet::from(["X", "phase_number", "PHASE_NUMBER", "Name"].map(String::from))
    );
    assert!(
        defined("and `{Name}`\nis its `.fields.name`").contains("Name"),
        "a wrapped one"
    );
    assert!(defined("the `{X}` placeholder").is_empty());
}

/// Every placeholder a phase label holds is defined in the asset that prints it, by a sentence that says what it is
/// (`` `{X}` is the phase's number, the last segment of its `id` ``); a step, mode or template file under a workflow's
/// folder also reads its workflow's. A label with an undefined placeholder leaves the agent to guess a number of
/// another numbering, or a name no step set.
#[test]
fn every_phase_label_defines_its_placeholders() {
    let all = assets();
    let text_of = |tree: &str, path: &str| {
        all.iter()
            .find(|(t, p, _)| t == tree && p == path)
            .map(|(_, _, text)| text.as_str())
    };
    let (mut printing, mut seen, mut undefined) = (0, 0, Vec::new());
    for (tree, path, text) in &all {
        let found = labels(text);
        if found.is_empty() {
            continue;
        }
        printing += 1;
        seen += found.len();
        let mut known = defined(text);
        // gsd-core/workflows/<workflow>/... reads gsd-core/workflows/<workflow>.md
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() > 3 && parts[..2] == ["gsd-core", "workflows"] {
            let parent = format!("gsd-core/workflows/{}.md", parts[2]);
            known.extend(text_of(tree, &parent).map(defined).unwrap_or_default());
        }
        for (line, name) in found {
            if !known.contains(&name) {
                undefined.push(format!("{path}:{line}: {{{name}}}"));
            }
        }
    }
    assert!(
        printing >= 35 && seen >= 150,
        "{printing} assets, {seen} placeholders"
    );
    assert!(
        undefined.is_empty(),
        "undefined in a phase label:\n{}",
        undefined.join("\n")
    );
}

/// A phase's label is `Phase <its id's last segment>: <its name>`: plan-phase's step 3 and autonomous's phase details
/// say which part of the node each value is (a node holds no `phase_number` key), `{X}` is defined, and plan-phase's
/// own body prints no `{Name}` that no step sets.
#[test]
fn the_phase_label_is_read_from_the_id_and_the_name_field() {
    let all = prompts();
    let rendered = |command: &str| {
        let (_, combos) = all.iter().find(|(c, _)| c == command).expect("the prompt");
        render_text(command, "claude-code", &combos[0].0, &combos[0].1).expect("a render")
    };
    // a sentence wrapped over lines reads as one
    let one_line = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let plan = rendered("plan-phase");
    let (_, step) = plan.split_once("## 3. Validate Phase").expect("step 3");
    let step = one_line(step.split_once("\n## ").expect("step 4").0);
    let step = step.as_str();
    let autonomous = one_line(&rendered("autonomous"));
    for (text, said) in [
        (step, "`phase_number` is the last segment of its `id`"),
        (step, "`phase_name` is its `.fields.name`"),
        (step, "`{X}` is `phase_number`"),
        (
            step,
            "a phase's label reads \"Phase {phase_number}: {phase_name}\"",
        ),
        (
            &autonomous,
            "`PHASE_NAME` (`phase_name`) is its `.fields.name`",
        ),
        (&autonomous, "`PHASE_NUM` is the last segment of its `id`"),
        (
            &autonomous,
            "a phase's label reads \"Phase ${PHASE_NUM}: ${PHASE_NAME}\"",
        ),
    ] {
        assert!(text.contains(said), "{said:?} is not said");
    }
    let own = asset("gsd-core/workflows/plan-phase.md").expect("an asset");
    assert!(!own.contains("{Name}"), "plan-phase prints a `{{Name}}`");
}
