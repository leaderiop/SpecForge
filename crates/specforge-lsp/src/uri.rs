//! The two conversions between the filesystem and the protocol's document
//! URIs.

use tower_lsp::lsp_types::Url;

/// The `file://` URI of `path`.
pub(crate) fn file_path_to_uri(path: &str) -> Url {
    Url::from_file_path(path).unwrap_or_else(|_| {
        Url::parse(&format!("file://{path}")).unwrap_or_else(|_| Url::parse("file:///").unwrap())
    })
}

/// The path of a `file://` URI; any other URI as written.
pub(crate) fn uri_to_file_path(uri: &Url) -> String {
    uri.to_file_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| uri.to_string())
}
