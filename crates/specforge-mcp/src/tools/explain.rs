use serde_json::{Value, json};
use specforge_diagnostics::{CodeEntry, docs_href, lookup, retired};

use crate::args::Arguments;
use crate::tool::ToolOutcome;

/// `specforge.explain`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// A diagnostic code, any case
    code: String,
}

/// `specforge.explain`: what `specforge explain <code>` prints, as data.
/// A retired code names the entry that replaced it, if any; a code the
/// catalogue doesn't have is invalid input.
pub fn call(args: Args) -> ToolOutcome {
    let code = args.code.trim();
    if let Some(replacement) = retired(code) {
        return ToolOutcome::ok(json!({
            "code": code.to_uppercase(),
            "retired": true,
            "replaced_by": replacement.and_then(lookup).map(entry_json),
        }));
    }
    match lookup(code) {
        Some(entry) => {
            let mut payload = entry_json(entry);
            payload["retired"] = Value::Bool(false);
            ToolOutcome::ok(payload)
        }
        None => ToolOutcome::invalid_input(
            "code",
            format!(
                "Unknown diagnostic code: {code}. Catalogued codes are E###, W###, I###, A###, \
                 R### and R-<AREA>-###; E900-E998, W900-W998 and I900-I998 belong to \
                 third-party extensions"
            ),
        ),
    }
}

fn entry_json(entry: &CodeEntry) -> Value {
    json!({
        "code": entry.code,
        "title": entry.title,
        "owner": entry.owner,
        "level": entry.level.describe(),
        "explanation": entry.explanation,
        "docs": docs_href(entry.code),
    })
}
