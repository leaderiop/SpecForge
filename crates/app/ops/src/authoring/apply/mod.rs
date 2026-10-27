//! The one place a planned change is written (ADR 0289): [`apply`] writes it under the project's write lock and names who
//! asked. A typed edit, a rename and `harness gc` write through it, and the surfaces and tests do not call
//! [`PlannedChange::apply`] themselves.

use crate::authoring::planned::PlannedChange;
use crate::{Caller, OpError, Writes};
use specforge_project::CompiledProject;
use specforge_project::harness::LedgerChange;

/// The surface a core operation was asked on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Mcp,
    /// `interactive`: stdin and stdout are a terminal, so a person is at the keyboard.
    Cli {
        interactive: bool,
    },
    Lsp,
}

/// Who asked for a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A command of `extension` answered the plan.
    Command { extension: String },
    /// A collector of `extension` answered the plan.
    Collector { extension: String },
    /// A hook of `extension` answered the plan.
    Hook { extension: String },
    /// The host's own operation, asked on `surface`.
    Core { surface: Surface },
}

impl Origin {
    /// A core operation asked on the command line.
    pub fn cli(interactive: bool) -> Self {
        Origin::Core {
            surface: Surface::Cli { interactive },
        }
    }

    /// A core operation asked over MCP.
    pub fn mcp() -> Self {
        Origin::Core {
            surface: Surface::Mcp,
        }
    }
}

/// Write `change` as one project change (ADR 0128) under the write lock, each file resolved through the caller's
/// containment (ADR 0104): checked against the text it was planned on, then replaced or created; on any failure,
/// every file already written is put back. Returns the files it wrote.
pub fn apply(
    change: &PlannedChange,
    _origin: &Origin,
    caller: Caller<'_>,
) -> Result<Writes, OpError> {
    change.apply(caller)
}

/// Write the ledger change of a rename the editor applied to its buffers and has saved (ADR 0289): the ledger as the
/// rename leaves it, `after` being the project the buffers show, under the write lock, over what the ledger holds
/// now. The renamed files are the editor's to save, so this writes nothing else.
pub fn land_ledger(
    before: &CompiledProject,
    after: &CompiledProject,
    change: LedgerChange,
    origin: &Origin,
    caller: Caller<'_>,
) -> Result<Writes, OpError> {
    let planned = PlannedChange::new(
        after.environment().sources().clone(),
        "cannot write the ledger of a rename",
        [],
    )
    .landing_ledger(before, after, change);
    apply(&planned, origin, caller)
}

/// What tests write through (behind the `testing` feature): a core operation on the command line, nobody at a terminal.
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::{Origin, apply as write};
    use crate::authoring::planned::PlannedChange;
    use crate::{Caller, OpError, Writes};

    /// The origin most tests write as.
    pub fn origin() -> Origin {
        Origin::cli(false)
    }

    /// Write `change` as [`origin`]: the one way a test applies a planned change.
    pub fn apply(change: &PlannedChange, caller: Caller<'_>) -> Result<Writes, OpError> {
        write(change, &origin(), caller)
    }
}
