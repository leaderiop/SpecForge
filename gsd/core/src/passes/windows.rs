//! `gsd/E513`: a resolved window is never both waived and fixed (`src/broken-windows.cts`: only an `open` window
//! transitions). `waiver` and `fixed_at` are harness fields, each immutable once recorded, so a ledger that holds both
//! can only come from two commands acting on one window.

use specforge_extension_sdk::prelude::*;

use super::plans::of_kind;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("windows", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
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
