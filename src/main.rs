use rmcp::ServiceExt;
use sequentialthinking_rs::SequentialThinkingServer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configure structured logging to stderr (critical to preserve stdout for JSON-RPC MCP transport)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing_subscriber::filter::LevelFilter::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Sequential Thinking MCP Server on stdio");

    let server = SequentialThinkingServer::new();
    let transport = rmcp::transport::io::stdio();

    let server_handle = server.serve(transport).await?;
    server_handle.waiting().await?;

    info!("Sequential Thinking MCP Server shut down cleanly");
    Ok(())
}
