mod backend;
mod code_actions;
mod completion;
mod diagnostics;
mod document_symbol;
mod document_sync;
mod goto_definition;
mod hover;
mod position;
mod references;
mod rename;
mod semantic_tokens;
mod state;
mod util;
mod workspace_symbol;

use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(backend::Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
