use axum::{Json, extract::State};
use serde_json::Value;
use std::sync::Arc;

use crate::AppState;

pub async fn handle_create_event(
    State(_app): State<Arc<AppState>>,
    Json(_payload): Json<Value>,
) -> (axum::http::StatusCode, &'static str) {
    (axum::http::StatusCode::ACCEPTED, "queued")
}
