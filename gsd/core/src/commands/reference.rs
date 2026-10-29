//! `reference` (D-r42-14): the read-only door to the ported workflows, references and templates a prompt does not
//! inline. Its answer is the embedded file as a model reads it: re-pointed (D-r42-5), without the attribution line,
//! and, for a template, with a first line that says which entities SpecForge keeps instead of the file.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, required, run};
use crate::prompts::{assets, compose, repoint::repoint, setting};
use crate::settings;

/// What a template becomes: `(file name or folder, kinds)`; no kinds means the template has no entity.
const TEMPLATES: &[(&str, &[&str])] = &[
    ("project.md", &["project"]),
    ("requirements.md", &["requirement"]),
    ("roadmap.md", &["milestone", "phase"]),
    ("milestone.md", &["milestone", "phase"]),
    ("context.md", &["context"]),
    ("spec.md", &["phase_spec"]),
    ("summary.md", &["summary"]),
    ("UAT.md", &["uat_session"]),
    ("VALIDATION.md", &["validation"]),
    ("verification-report.md", &["verification"]),
    ("retrospective.md", &["retrospective"]),
    ("user-setup.md", &["user_setup"]),
    ("research-project/", &["research"]),
];

/// The line a template answer starts with, when SpecForge keeps its content somewhere else.
fn kind_line(path: &str) -> Option<String> {
    let rest = path.strip_prefix("gsd-core/templates/")?;
    match rest {
        "state.md" => {
            return Some(
                "SpecForge computes this: there is no entity to write (read it with `specforge query`).".to_string(),
            );
        }
        "milestone-archive.md" => {
            return Some(
                "SpecForge never archives a milestone: it stays a `milestone` entity (`specforge gsd close-milestone` closes it)."
                    .to_string(),
            );
        }
        _ => {}
    }
    let (_, kinds) = TEMPLATES
        .iter()
        .find(|(name, _)| rest == *name || (name.ends_with('/') && rest.starts_with(name)))?;
    let list = kinds
        .iter()
        .map(|k| format!("`{k}`"))
        .collect::<Vec<_>>()
        .join(" and ");
    Some(format!(
        "SpecForge keeps this as {list} entities: create with `specforge new <kind>`, see `specforge schema --kind <kind>`."
    ))
}

/// The edit distance of two paths, to name the nearest one of a mistyped path.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let kept = row[j + 1];
            row[j + 1] = (row[j + 1] + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = kept;
        }
    }
    row[b.len()]
}

/// The embedded path `asked` names: as written, or under `gsd-core/` (`workflows/x.md`).
fn resolve(asked: &str) -> Result<&'static str, String> {
    let asked = asked.trim().trim_start_matches("./");
    for candidate in [asked.to_string(), format!("gsd-core/{asked}")] {
        if let Some((path, _)) = assets::ASSETS.iter().find(|(p, _)| *p == candidate) {
            return Ok(path);
        }
    }
    let nearest = assets::paths()
        .min_by_key(|p| distance(p, asked))
        .unwrap_or_default();
    Err(format!(
        "no embedded file '{asked}'; the nearest is '{nearest}'"
    ))
}

fn reference(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let path = resolve(required(call, "file")?)?;
    let text = assets::get(path).unwrap_or_default();
    // The attribution line is the notice of the file, not part of what a model reads.
    let body = text
        .strip_prefix("<!-- Ported from")
        .and_then(|rest| rest.split_once('\n'))
        .map_or(text, |(_, body)| body);
    let mut answered = String::new();
    if let Some(line) = kind_line(path) {
        answered.push_str(&line);
        answered.push_str("\n\n");
    }
    answered.push_str(&setting::fill(
        &repoint(&compose::composed(path, body)),
        &settings::block(),
    ));
    Ok(answer(
        call,
        json!({ "path": path, "text": answered }),
        answered.clone(),
        None,
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("reference", |cmd| {
        cmd.description("Print one ported GSD workflow, reference or template by its path (gsd-core/references/x.md; the gsd-core/ may be left out)")
            .arg("file", |a| {
                a.required().description("the file, as the text that names it writes it");
            })
            .handler(|call| run(call, reference));
    });
}
