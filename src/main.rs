use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::time::{SystemTime, UNIX_EPOCH};

use rmcp::ServiceExt;
use sequentialthinking_rs::SequentialThinkingServer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Direct stderr to a unique file in /tmp/ by default (preserving clean stdout/stderr on stdio)
    let pid = std::process::id();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let log_path = std::env::var("SEQ_LOG_FILE")
        .or_else(|_| std::env::var("SEQUENTIALTHINKING_LOG_FILE"))
        .unwrap_or_else(|_| {
            format!(
                "{}/seq-{}-{}.log",
                std::env::temp_dir().display(),
                pid,
                timestamp
            )
        });

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    // Duplicate fd 2 (stderr) to the unique log file
    unsafe {
        libc::dup2(log_file.as_raw_fd(), 2);
    }

    // Configure structured logging to stderr (now pointing to the unique log file)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing_subscriber::filter::LevelFilter::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!(
        log_file = %log_path,
        "Starting Seq MCP Server on stdio with stderr redirected to log file"
    );

    let server = SequentialThinkingServer::new();
    let transport = rmcp::transport::io::stdio();

    let server_handle = server.serve(transport).await?;
    server_handle.waiting().await?;

    info!("Seq MCP Server shut down cleanly");
    Ok(())
}
