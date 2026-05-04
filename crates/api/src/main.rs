use anyhow::Result;
use axum::{Router, routing::get, routing::post};
use common::{Config, EventInput, PipelineConfig};
use dotenvy::dotenv;
use pipeline::{Batcher, EventSink, FileSink, KafkaSink};
use std::env;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal::ctrl_c;
use tokio::{net::TcpListener, sync::mpsc::channel};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

mod error;
use error::InitError;

mod state;
use state::AppState;

mod handler;
use handler::handle_create_event;

use crate::handler::check_health;

fn init_sink(pipeline_config: &PipelineConfig) -> Result<Arc<dyn EventSink>, InitError> {
    let sink: Arc<dyn EventSink> = if pipeline_config.enable_kafka {
        let kafka = KafkaSink::new(pipeline_config)?;
        Arc::new(kafka)
    } else {
        let file = FileSink::new(pipeline_config)?; 
        Arc::new(file)
    };
    Ok(sink)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 12)]
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
        env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
    );

    let config = Config::new();

    let (tx_main, rx) = channel::<EventInput>(config.app.channel_capacity);
    let tx_app = tx_main.clone();
    
    let sink = init_sink(&config.pipeline)?;
    let batcher = Batcher::new(rx, sink, &config.pipeline);

    let pipeline_handle = tokio::spawn(async move {
        batcher
            .run()
            .await
            .inspect_err(|e| tracing::error!(error=%e, "pipeline task terminated"))
    });

    let state = AppState::new(tx_app);
    let app: Router = Router::new()
        .route("/v1/events", post(handle_create_event))
        .route("/health", get(check_health))
        .with_state(Arc::new(state));

    let addr = SocketAddr::from((config.app.server_host, config.app.server_port));
    tracing::info!("Server launched on {}", &addr);

    let listener = TcpListener::bind(addr).await?;
    let shutdown_signal = async {
        if let Err(e) = ctrl_c().await {
            tracing::error!(error = %e, "failed to wait for shutdown signal");
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    drop(tx_main);

    match pipeline_handle.await {
        Ok(Ok(())) => tracing::info!("pipeline shutdown complete"),
        Ok(Err(e)) => tracing::error!(error=%e, "pipeline returned error on shutdown"),
        Err(e) => tracing::error!(error=%e, "pipeline task panicked"),
    }

    tracing::info!("Application shutdown complete");

    Ok(())
}
