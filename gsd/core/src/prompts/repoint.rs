//! Re-pointing (D-r42-5): upstream's text reads its references through `~/.claude/gsd-core/<path>`, a directory this
//! port does not install. At render time every such path becomes the command that serves the embedded file, so the
//! assets stay byte-equal to upstream's. The pipeline is `gsd_kit::prompt::repoint`; this is `@gsd/core`'s call of it.

use super::assets;

/// `text` with each `[@]<prefix>.claude/gsd-core/<path>` replaced by `` `specforge gsd reference gsd-core/<path>` ``.
pub fn repoint(text: &str) -> String {
    repoint_for(text, "claude-code")
}

/// [`repoint`] for the text of `host`.
pub fn repoint_for(text: &str, host: &str) -> String {
    gsd_kit::prompt::repoint::repoint(text, host, &|path| assets::get(path).is_some())
}

#[cfg(test)]
mod tests;
