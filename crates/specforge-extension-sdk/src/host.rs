//! Typed access to the three host functions the runtime injects. The externs
//! are declared here, inside the SDK, so they link into the plugin module
//! without the author writing `extern "ExtismHost"` blocks.

/// Safe wrappers over the host functions the runtime injects into every
/// extension module.
impl super::HostApi {
    /// Ask the host a structured question about the graph under compilation.
    /// `query` is a JSON payload; the response is the host's JSON answer.
    pub fn query_graph(&self, query: &str) -> String {
        let resp = unsafe { host_query_graph(query.as_bytes().to_vec()) }
            .expect("host_query_graph failed");
        String::from_utf8_lossy(&resp).into_owned()
    }

    /// Report a diagnostic back to the host (JSON matching the Diagnostic
    /// shape: code/severity/message/span/suggestion).
    pub fn emit_diagnostic(&self, diagnostic_json: &str) {
        let _ = unsafe { host_emit_diagnostic(diagnostic_json.as_bytes().to_vec()) };
    }

    /// Read a file inside the spec root (subject to the extension's sandbox
    /// policy). Returns `None` when the file does not exist or is denied.
    pub fn read_file(&self, rel_path: &str) -> Option<String> {
        let resp = unsafe { host_read_file(rel_path.as_bytes().to_vec()) }.ok()?;
        if resp.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(&resp).into_owned())
        }
    }
}

#[extism_pdk::host_fn]
extern "ExtismHost" {
    fn host_emit_diagnostic(input: Vec<u8>) -> Vec<u8>;
    fn host_read_file(input: Vec<u8>) -> Vec<u8>;
    fn host_query_graph(input: Vec<u8>) -> Vec<u8>;
}
