mod config;
mod dto;
mod error;
mod openapi;
mod routes;
mod state;

use std::time::Duration;

use clap::Parser;
use config::Config;
use routes::router;
use state::{init_state, wait_for_scan};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,fff_search=info")),
        )
        .init();

    let cfg = Config::parse();
    tracing::info!(bind = %cfg.bind, "starting fff-server");

    let state = init_state(&cfg)?;

    let ready = wait_for_scan(&state, Duration::from_secs(cfg.wait_scan_secs));
    if ready {
        tracing::info!("initial scan ready");
    } else {
        tracing::warn!(
            "initial scan did not complete within {}s; serving with partial index",
            cfg.wait_scan_secs
        );
    }

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
