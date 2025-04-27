use bazel_lsp::server::Backend;
use tokio::runtime::Runtime;
use tower_lsp::{LspService, Server};

fn main() {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| Backend::new(client));
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}
