//! `specforge-lsp`: the language server on stdin and stdout.

use tower_lsp::Server;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = specforge_lsp::backend::service();
    Server::new(stdin, stdout, socket).serve(service).await;
}
