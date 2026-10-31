//! `gsd/E513`: a resolved window is never both waived and fixed (`src/broken-windows.cts`: only an `open` window
//! transitions). `waiver` and `fixed_at` are harness fields, each immutable once recorded, so a ledger that holds both
//! can only come from two commands acting on one window.
//!
//! `gsd/W514`: a refactor proposal recorded as an open `deviation` window while `workflow.windows_enforce` is off
//! blocks nothing at ship. The setting gates it while it is on (`gates_when_on`), so the host drops it then.
//!
//! gsd-core modules this file ports: broken-windows.cjs, complexity-trigger.cjs.

use specforge_extension_sdk::prelude::*;

use super::plans::{link, of_kind, text};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("windows", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = e513(input);
    found.extend(w514(input));
    found
}

fn w514(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "broken_window")
        .filter(|w| text(w, "kind") == Some("deviation") && text(w, "status") == Some("open"))
        .filter_map(|w| {
            let proposal = link(input, w, "refactor")?;
            Some(
                PassDiagnostic::new(
                    "W514",
                    PassSeverity::Warning,
                    format!(
                        "window '{}' tracks refactor proposal '{proposal}' as a deviation, but workflow.windows_enforce is off, so ship will not actually be blocked",
                        w.id
                    ),
                )
                .with_suggestion("specforge settings set @gsd/core workflow.windows_enforce true")
                .with_entity(&w.id),
            )
        })
        .collect()
}

fn e513(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "broken_window")
        .filter(|w| w.fields.contains_key("waiver") && w.fields.contains_key("fixed_at"))
        .map(|w| {
            PassDiagnostic::new(
                "E513",
                PassSeverity::Error,
                format!("window '{}' is both waived and fixed: a resolved window is never resolved again", w.id),
            )
            .with_suggestion("keep the resolution that happened first; the later command should have refused")
            .with_entity(&w.id)
        })
        .collect()
}
