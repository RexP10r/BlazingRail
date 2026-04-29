use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::ValidationError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed {0}")]
    Validation(#[from] ValidationError),

    #[error("queue is full, backpressure applied")]
    Backpressure,

    #[error("internal server error {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct AppErrorResponse {
    #[serde(rename = "type")]
    type_: String,
    title: String,
    status: u16,
    details: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, title, detail) = match &self {
            AppError::Validation(e) => (StatusCode::BAD_REQUEST, "Validation Error", e.to_string()),
            AppError::Backpressure => (
                StatusCode::SERVICE_UNAVAILABLE,
                "System Overloaded",
                "Event queue is full. Retry later".into(),
            ),
            AppError::Internal(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Error",
                e.to_string(),
            ),
        };
        let body = Json(AppErrorResponse {
            type_: format!("https://api.blazingrail.dev/errors{}", status.as_str()),
            title: title.into(),
            status: status.as_u16(),
            details: detail,
        });

        (status, body).into_response()
    }
}
