use anyhow::Result;
use axum::{Router, routing::post};
use common::{AppConfig, EventInput};
use dotenvy::dotenv;
use std::env;
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc::channel};
use tracing_subscriber::EnvFilter;

mod error;

mod state;
use state::AppState;

mod handler;
use handler::handle_create_event;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!(
        "Telemetry initialized. Log level: {}",
        env::var("RUST_LOG").unwrap_or_else(|_| "default".to_string())
    );

    let config = Arc::new(AppConfig::new());

    let (tx, mut rx) = channel::<EventInput>(config.channel_capacity);

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            tracing::debug!("Event received: {}", event.event_type);
        }
    });

    let state = AppState::new(config.clone(), tx);
    let app: Router = Router::new()
        .route("/v1/events", post(handle_create_event))
        .with_state(Arc::new(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], config.server_port));
    println!("Server launched on {}", &addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
