use anyhow::Result;
use axum::{Router, routing::get, routing::post};
use common::{AppConfig, EventInput, PipelineConfig};
use dotenvy::dotenv;
use pipeline::{Batcher, FileSink};
use std::env;
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc::channel};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

mod error;

mod state;
use state::AppState;

mod handler;
use handler::handle_create_event;

use crate::handler::check_health;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json_layer = fmt::layer().json();
    tracing_subscriber::registry()
        .with(json_layer)
        .with(filter)
        .init();

    tracing::info!(
        "Telemetry initialized. Log level: {}",
        env::var("RUST_LOG").unwrap_or_else(|_| "default".to_string())
    );

    let app_config = AppConfig::new();
    let pipeline_config = PipelineConfig::new();

    let (tx, rx) = channel::<EventInput>(app_config.channel_capacity);
    let sink = Arc::new(FileSink::new(&pipeline_config)?);

    let _pipeline_handle = tokio::spawn(async move {
        Batcher::new(rx, sink, &pipeline_config)
            .run()
            .await
            .unwrap_or_else(|e| tracing::error!(error=%e, "pipeline task terminated"))
    });

    let state = AppState::new(tx);
    let app: Router = Router::new()
        .route("/v1/events", post(handle_create_event))
        .route("/health", get(check_health))
        .with_state(Arc::new(state));

    let addr = SocketAddr::from((app_config.server_host, app_config.server_port));
    println!("Server launched on {}", &addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
