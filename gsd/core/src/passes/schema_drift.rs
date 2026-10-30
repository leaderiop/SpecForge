//! The schema-drift pass: `gsd/E405`, a phase that changed the schema files of an ORM and recorded no push of its
//! database. A port of `src/schema-detect.cts:20-73` (gsd-core v1.18.0, MIT, `LICENSE-GSD`): `SCHEMA_PATTERNS`, and
//! `ORM_INFO`'s `envHint` and `evidencePatterns`, hand-written because the crate has no regex dependency.
//!
//! GSD runs the gate at `execute:wave:post`, after the work, so a phase is judged once it has a summary (the nearest
//! stand-in: a commit alone may be the first of several, and the collector's write that records it must not be
//! refused): the changed files are its plans' `files_modified`, and the execution log is the text of its summaries
//! and deviations and the subjects of its commits. `GSD_SKIP_SCHEMA_CHECK` has no counterpart: `workflow.schema_drift_gate` is it.
//!
//! Also ports gsd-core bin/lib modules: gate-schema-drift.cjs.
//!
//! gsd-core modules this file ports: schema-detect.cjs.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::commits::{commits, target};
use super::plans::{of_kind, phase_of, strings, texts};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("schema_drift", |p| {
        p.phase("check").run(check);
    });
}

/// How a `SCHEMA_PATTERNS` regex reads: `^exact$`, or `^prefix.*suffix$`.
enum Shape {
    Exact(&'static str),
    Glob(&'static str, &'static str),
}

use Shape::{Exact, Glob};

const SCHEMA_PATTERNS: &[(Shape, &str)] = &[
    (Glob("src/collections/", ".ts"), "payload"),
    (Glob("src/globals/", ".ts"), "payload"),
    (Exact("prisma/schema.prisma"), "prisma"),
    (Glob("prisma/schema/", ".prisma"), "prisma"),
    (Exact("drizzle/schema.ts"), "drizzle"),
    (Exact("src/db/schema.ts"), "drizzle"),
    (Glob("drizzle/", ".ts"), "drizzle"),
    (Glob("supabase/migrations/", ".sql"), "supabase"),
    (Glob("src/entities/", ".ts"), "typeorm"),
    (Glob("src/migrations/", ".ts"), "typeorm"),
];

/// An ORM's `envHint`, its case-insensitive evidence (`\s+` between the words) and its case-sensitive evidence.
struct Orm {
    name: &'static str,
    env_hint: &'static str,
    runs: &'static [&'static [&'static str]],
    exact: &'static [&'static str],
}

const ORMS: &[Orm] = &[
    Orm {
        name: "payload",
        env_hint: "CI=true PAYLOAD_MIGRATING=true npx payload migrate",
        runs: &[&["payload", "migrate"]],
        exact: &["PAYLOAD_MIGRATING"],
    },
    Orm {
        name: "prisma",
        env_hint: "npx prisma db push --accept-data-loss (if destructive changes are intended)",
        runs: &[
            &["prisma", "db", "push"],
            &["prisma", "migrate", "deploy"],
            &["prisma", "migrate", "dev"],
        ],
        exact: &[],
    },
    Orm {
        name: "drizzle",
        env_hint: "npx drizzle-kit push",
        runs: &[&["drizzle-kit", "push"], &["drizzle-kit", "migrate"]],
        exact: &[],
    },
    Orm {
        name: "supabase",
        env_hint: "supabase db push",
        runs: &[
            &["supabase", "db", "push"],
            &["supabase", "migration", "up"],
        ],
        exact: &[],
    },
    Orm {
        name: "typeorm",
        env_hint: "npx typeorm migration:run -d src/data-source.ts",
        runs: &[&["typeorm", "migration:run"], &["typeorm", "schema:sync"]],
        exact: &[],
    },
];

/// The ORM whose schema files `file` is one of; the path is read with `/` separators (`posixNormalize`).
fn orm_of(file: &str) -> Option<&'static str> {
    let file = file.replace('\\', "/");
    SCHEMA_PATTERNS.iter().find_map(|(shape, orm)| {
        let fits = match shape {
            Exact(path) => file == *path,
            // `.*` does not cross a newline, and may be empty.
            Glob(prefix, suffix) => {
                file.len() >= prefix.len() + suffix.len()
                    && file.starts_with(prefix)
                    && file.ends_with(suffix)
                    && !file[prefix.len()..file.len() - suffix.len()].contains('\n')
            }
        };
        fits.then_some(*orm)
    })
}

/// Whether `rest` starts with `words`, each ASCII case-insensitively and apart by whitespace (`a\s+b`).
fn starts_run(mut rest: &str, words: &[&str]) -> bool {
    for (n, word) in words.iter().enumerate() {
        if n > 0 {
            let trimmed = rest.trim_start_matches(char::is_whitespace);
            if trimmed.len() == rest.len() {
                return false;
            }
            rest = trimmed;
        }
        match rest.get(..word.len()) {
            Some(head) if head.eq_ignore_ascii_case(word) => rest = &rest[word.len()..],
            _ => return false,
        }
    }
    true
}

fn has_evidence(orm: &Orm, log: &str) -> bool {
    orm.exact.iter().any(|e| log.contains(e))
        || log
            .char_indices()
            .any(|(at, _)| orm.runs.iter().any(|run| starts_run(&log[at..], run)))
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let commits = commits(input);
    let mut phases: BTreeMap<&str, Vec<&PassEntity>> = BTreeMap::new();
    for plan in of_kind(input, "plan") {
        if let Some(phase) = phase_of(input, &plan.id) {
            phases.entry(phase).or_default().push(plan);
        }
    }
    let mut found = Vec::new();
    for (phase, plans) in phases {
        let ids: Vec<&str> = plans.iter().map(|p| p.id.as_str()).collect();
        let summaries: Vec<&PassEntity> = of_kind(input, "summary")
            .filter(|s| ids.contains(&crate::digest::scope_of(&s.id)))
            .collect();
        let subjects: Vec<&str> = commits
            .iter()
            .filter(|c| c.plan.as_deref().is_some_and(|p| ids.contains(&p)))
            .map(|c| c.subject.as_str())
            .collect();
        // Until a summary closes a plan there is no wave to judge: the push may be a commit still to come.
        if summaries.is_empty() {
            continue;
        }
        let deviations = of_kind(input, "deviation").filter(|d| {
            target(input, &d.id, "summary").is_some_and(|s| summaries.iter().any(|x| x.id == s))
        });
        let log: String = summaries
            .iter()
            .copied()
            .chain(deviations)
            .flat_map(texts)
            .chain(subjects.iter().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        let files: Vec<String> = plans
            .iter()
            .flat_map(|p| strings(p, "files_modified"))
            .map(|f| f.trim().to_string())
            .filter(|f| orm_of(f).is_some())
            .collect();
        let mut orms: Vec<&Orm> = Vec::new();
        for orm in files.iter().filter_map(|f| orm_of(f)) {
            let known = ORMS.iter().find(|o| o.name == orm);
            if let Some(known) = known.filter(|k| !orms.iter().any(|o| o.name == k.name)) {
                orms.push(known);
            }
        }
        let unpushed: Vec<&&Orm> = orms.iter().filter(|o| !has_evidence(o, &log)).collect();
        if unpushed.is_empty() {
            continue;
        }
        let commands: Vec<String> = unpushed
            .iter()
            .map(|o| format!("{}: {}", o.name, o.env_hint))
            .collect();
        found.push(
            PassDiagnostic::new(
                "E405",
                PassSeverity::Error,
                format!(
                    "phase '{phase}' changed schema files ({}) but records no database push for {}",
                    files.join(", "),
                    unpushed
                        .iter()
                        .map(|o| o.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .with_suggestion(format!(
                "run the push command and name it in the plan's summary: {}",
                commands.join("; ")
            ))
            .with_entity(phase),
        );
    }
    found
}

#[cfg(test)]
mod tests;
