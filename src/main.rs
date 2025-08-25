use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("uatu_watcher=debug".parse()?),
        )
        .init();

    info!("Starting Uatu Watcher...");
    
    // TODO: Implement main application logic
    // For now, just keep running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    
    Ok(())
}