use rmcp::{
    ServerHandler, ServiceExt,
    model::{Implementation, ServerCapabilities, ServerInfo},
    transport::stdio,
};

#[derive(Debug, Clone)]
struct BmadServer;

impl ServerHandler for BmadServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().build())
            .with_server_info(Implementation::new(
                "bmad-method-server",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "BMad Method MCP Server provides guidance for the Build More Architect Dreams \
                 methodology. It helps AI agents determine which step to move to next and which \
                 flow or command to use."
                    .to_string(),
            )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting BMad Method MCP server");

    let server = BmadServer;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
