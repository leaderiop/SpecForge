//! The two exports a host contribution adds (ADR 0292): the `render__` export that writes an agent's or skill's body,
//! and a hook's `hook__<id>` export, which the host holds to the hook's own timeout.

use specforge_protocol_types::{HookInput, HookOutput, RawGraph, RenderInput, RenderOutput};

use crate::calls::{CallError, ExtensionCalls, Operation, decode, encode};

/// The export an extension answers every render by.
pub const RENDER_EXPORT: &str = "render__";

impl ExtensionCalls<'_> {
    /// The body of the agent or skill `input` names, for one host. The body is never empty: an extension that
    /// declares an agent owes its text.
    pub fn render(&self, extension: &str, input: &RenderInput) -> Result<RenderOutput, CallError> {
        let bytes = encode(Operation::Render, extension, RENDER_EXPORT, input)?;
        let answer = self.call_raw_within(
            Operation::Render,
            extension,
            RENDER_EXPORT,
            &bytes,
            u32::MAX,
        )?;
        decode(
            Operation::Render,
            extension,
            RENDER_EXPORT,
            &answer,
            "RenderOutput",
        )
    }

    /// The hook export `export` on `input`, within `timeout_ms` (never longer than the extension's own limit).
    pub fn run_hook(
        &self,
        extension: &str,
        export: &str,
        input: &HookInput<RawGraph>,
        timeout_ms: u32,
    ) -> Result<HookOutput, CallError> {
        let bytes = encode(Operation::Hook, extension, export, input)?;
        let answer =
            self.call_raw_within(Operation::Hook, extension, export, &bytes, timeout_ms)?;
        decode(Operation::Hook, extension, export, &answer, "HookOutput")
    }
}
