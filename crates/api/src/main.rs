use anyhow::Result;
use axum::{Router, routing::get, routing::post};
use axum_prometheus::PrometheusMetricLayer;
use common::{Config, EventInput, KafkaRoutingConfig, PipelineConfig};
use dotenvy::dotenv;
use pipeline::{Batcher, CircuitBreaker, EventSink, FileSink, KafkaSink};
use std::env;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal::ctrl_c;
use tokio::sync::watch;
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

fn init_sink(pipeline_config: &PipelineConfig) -> Result<Arc<dyn EventSink>, InitError> {
    let fallback = Arc::new(FileSink::new(pipeline_config)?);
    if pipeline_config.enable_kafka {
        let routing_conf: KafkaRoutingConfig;
        match KafkaRoutingConfig::new(&pipeline_config.kafka_routing_conf_path) {
            Some(content) => routing_conf = content,
            None => return Err(InitError::WrongKafkaConfig),
        };
        match KafkaSink::new(pipeline_config, &routing_conf) {
            Ok(primary) => {
                let breaker = CircuitBreaker::new(Arc::new(primary), fallback, pipeline_config);
                return Ok(Arc::new(breaker));
            }
            Err(e) => tracing::warn!(error = %e, "Kafka init failed, fallback to file sink"),
        }
    }
    Ok(fallback)
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

    let (tx, rx) = channel::<EventInput>(config.app.channel_capacity);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let sink = init_sink(&config.pipeline)?;
    let batcher = Batcher::new(rx, sink, &config.pipeline, shutdown_rx.clone());

    let pipeline_handle = tokio::spawn(async move {
        batcher
            .run()
            .await
            .inspect_err(|e| tracing::error!(error=%e, "pipeline task terminated"))
    });

    let state = Arc::new(AppState::new(tx, shutdown_rx));
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let app: Router = Router::new()
        .route("/v1/events", post(handle_create_event))
        .route("/health", get(check_health))
        .route("/metrics", get(move || metrics_handler(metric_handle)))
        .route("/ready", get(check_ready))
        .with_state(state.clone())
        .layer(prometheus_layer);

    let addr = SocketAddr::from((config.app.server_host, config.app.server_port));
    tracing::info!("Server launched on {}", &addr);

    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;

    socket.bind(&addr.into())?;
    socket.listen(config.app.socket_max_connections)?;

    let std_listener = std::net::TcpListener::from(socket);
    let listener = TcpListener::from_std(std_listener)?;
    let shutdown_signal = async {
        if let Err(e) = ctrl_c().await {
            tracing::error!(error = %e, "failed to wait for shutdown signal");
        }
    };

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
