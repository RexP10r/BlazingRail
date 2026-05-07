use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_prometheus::metrics_exporter_prometheus::PrometheusHandle;
use common::EventInput;
use std::sync::Arc;
use tokio::sync::mpsc::error::TrySendError;

use crate::{AppState, error::AppError};

pub async fn handle_create_event(
    State(state): State<Arc<AppState>>,
    Json(input_event): Json<EventInput>,
) -> Result<impl IntoResponse, AppError> {
    input_event.validate()?;

    state.tx.try_send(input_event).map_err(|err| match err {
        TrySendError::Full(_) => AppError::Backpressure,
        TrySendError::Closed(_) => AppError::Internal,
    })?;

    let depth_value = (state.tx.max_capacity() - state.tx.capacity()) as f64;
    metrics::gauge!("blazingrail_queue_depth").set(depth_value);

    Ok(StatusCode::ACCEPTED)
}

pub async fn check_health() -> Response {
    StatusCode::ACCEPTED.into_response()
}

pub async fn metrics_handler(handle: PrometheusHandle) -> impl IntoResponse {
    let body = handle.render();
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.1.0")],
        body,
    )
}
