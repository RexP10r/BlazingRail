use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use common::{EventInput, models::RawEventInput};
use std::sync::Arc;
use tokio::sync::mpsc::error::TrySendError;

use crate::{AppState, error::AppError};

pub async fn handle_create_event(
    State(state): State<Arc<AppState>>,
    Json(raw_event): Json<RawEventInput>,
) -> Result<impl IntoResponse, AppError> {
    raw_event.validate()?;

    let event_input = EventInput::from_raw(raw_event)?;

    event_input.validate()?;

    state.tx.try_send(event_input).map_err(|err| match err {
        TrySendError::Full(_) => AppError::Backpressure,
        TrySendError::Closed(_) => AppError::Internal(anyhow::anyhow!("Channel closed")),
    })?;

    tracing::info!("Received event");

    Ok(StatusCode::ACCEPTED)
}
