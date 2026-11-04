//! `specforge-lsp`: the language server on stdin and stdout.

use std::sync::Arc;

use tower_lsp::Server;

#[tokio::main]
async fn main() {
    // The composition root (ADR 0335): the effect plane's adapters, built once.
    let process = Arc::new(specforge_process::Runner);
    let ports = specforge_effects::Ports {
        git: Arc::new(specforge_git::Gix::default()),
        process,
        http: Arc::new(specforge_http::Client::new()),
        fs: Arc::new(specforge_fs::LocalFs),
    };
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = specforge_lsp::backend::service(ports);
    Server::new(stdin, stdout, socket).serve(service).await;
}
