use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
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

    tracing::debug!("Event validated");

    Ok(StatusCode::ACCEPTED)
}

pub async fn check_health() -> Response {
    StatusCode::ACCEPTED.into_response()
}
