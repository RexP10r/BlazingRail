use anyhow::Result;
use axum::{Router, routing::get, routing::post};
use axum_prometheus::PrometheusMetricLayer;
use common::{AppConfig, Config, EventInput, KafkaRoutingConfig, PipelineConfig};
use dotenvy::dotenv;
use pipeline::{Batcher, CircuitBreaker, EventSink, FileSink, KafkaSink};
use std::env;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal::ctrl_c;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::{mpsc, watch};
use tokio::{net::TcpListener, sync::mpsc::channel};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

use socket2::{Domain, Socket, Type};

mod error;
use error::InitError;

mod state;
use state::AppState;

mod handler;
use handler::handle_create_event;

use crate::handler::{check_health, check_ready, metrics_handler};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(filter)
        .init();
}

fn load_application_config() -> Result<Config> {
    dotenv().ok();
    Ok(Config::new())
}

fn init_channels(
    capacity: usize,
) -> (
    watch::Sender<bool>,
    watch::Receiver<bool>,
    mpsc::Sender<EventInput>,
    mpsc::Receiver<EventInput>,
) {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (event_tx, event_rx) = channel::<EventInput>(capacity);
    (shutdown_tx, shutdown_rx, event_tx, event_rx)
}

fn wrap_with_circuit_breaker(
    primary: Arc<dyn EventSink>,
    fallback: Arc<dyn EventSink>,
    config: &PipelineConfig,
) -> Arc<dyn EventSink> {
    if config.enable_circuit_breaker {
        Arc::new(CircuitBreaker::new(primary, fallback, config))
    } else {
        primary
    }
}

fn init_sink(pipeline_config: &PipelineConfig) -> Result<Arc<dyn EventSink>, InitError> {
    let fallback: Arc<dyn EventSink> = Arc::new(FileSink::new(pipeline_config)?);

    if !pipeline_config.enable_kafka {
        return Ok(fallback);
    }

    let routing_conf = KafkaRoutingConfig::new(&pipeline_config.kafka_routing_conf_path)
        .ok_or(InitError::WrongKafkaConfig)?;

    match KafkaSink::new(pipeline_config, &routing_conf) {
        Ok(primary) => {
            let primary_arc: Arc<dyn EventSink> = Arc::new(primary);
            Ok(wrap_with_circuit_breaker(
                primary_arc,
                fallback,
                pipeline_config,
            ))
        }
        Err(e) => {
            tracing::warn!(error = %e, "Kafka init failed, fallback to file sink");
            Err(InitError::Kafka(e))
        }
    }
}

fn build_router(state: Arc<AppState>) -> Router {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    Router::new()
        .route("/v1/events", post(handle_create_event))
        .route("/health", get(check_health))
        .route("/metrics", get(move || metrics_handler(metric_handle)))
        .route("/ready", get(check_ready))
        .with_state(state.clone())
        .layer(prometheus_layer)
}

fn configure_socket(config: &AppConfig) -> Result<TcpListener> {
    let addr = SocketAddr::from((config.server_host, config.server_port));
    tracing::info!("Server launched on {}", &addr);

    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;

    socket.bind(&addr.into())?;
    socket.listen(config.socket_max_connections)?;

    let std_listener = std::net::TcpListener::from(socket);
    Ok(TcpListener::from_std(std_listener)?)
}
#[tokio::main]
async fn main() -> Result<()> {
    let config = load_application_config()?;

    init_logging();
    tracing::info!(
        "Telemetry initialized. Log level: {}",
        env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
    );

    let (shutdown_tx, shutdown_rx, tx, rx) = init_channels(config.app.channel_capacity);

    let sink = init_sink(&config.pipeline)?;
    let batcher = Batcher::new(rx, sink, &config.pipeline, shutdown_rx.clone());

    let pipeline_handle = tokio::spawn(async move {
        batcher
            .run()
            .await
            .inspect_err(|e| tracing::error!(error=%e, "pipeline task terminated"))
    });

    let state = Arc::new(AppState::new(tx, shutdown_rx));
    let app = build_router(state);

    let mut sigterm = signal(SignalKind::terminate())?;
    let shutdown_signal = async move {
        tokio::select! {
            _ = ctrl_c() => {},
            _ = sigterm.recv() => {}
        }
    };

    let listener = configure_socket(&config.app)?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    shutdown_tx.send(true).ok();

    let shutdown_timeout = Duration::from_secs(config.app.shutdown_timeout_secs);
    let shutdown_result = tokio::time::timeout(shutdown_timeout, pipeline_handle)
        .await
        .map_err(|_| "shutdown_timeout")
        .and_then(|join| join.map_err(|_| "task_panicked"));

    match shutdown_result {
        Ok(Ok(())) => tracing::info!("pipeline shutdown complete"),
        Ok(Err(e)) => tracing::error!(error=%e, "pipeline error: {}", e),
        Err(e) => {
            tracing::error!(e, "shutdown failed: {} — forcing exit", e);
            std::process::exit(1);
        }
    }
    tracing::info!("Application shutdown complete");

    Ok(())
}
