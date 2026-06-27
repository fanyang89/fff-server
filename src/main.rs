mod config;
mod dto;
mod error;
mod limits;
mod mcp;
mod openapi;
mod routes;
mod state;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use clap::Parser;
use config::Config;
use routes::router;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::parse();
    tracing::info!(bind = %cfg.bind, "starting plocate-server (plocate backend)");

    let state = AppState::new(&cfg)?;
    tracing::info!(
        base_path = %state.base_path.display(),
        db_path = %state.db_path.display(),
        "indexed root configured"
    );

    // If the plocate database doesn't exist yet, build it in the background so
    // the server starts serving immediately (searches return empty until ready).
    if !state.db_exists() {
        tracing::info!("plocate database missing — starting initial build in background");
        state.clone().trigger_reindex();
    } else {
        tracing::info!("existing plocate database found — ready immediately");
    }

    // Periodic reindex loop (no-op if interval is 0).
    let _reindex_handle = state
        .clone()
        .spawn_reindex_interval(cfg.reindex_interval_secs);

    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    let addr = listener.local_addr()?;
    tracing::info!(%addr, swagger = format!("http://{addr}/swagger-ui"), "listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
