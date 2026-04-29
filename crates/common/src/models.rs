use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct EventInput {
    pub event_type: String,
    pub payload: Value,
}

impl EventInput {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.event_type.is_empty() {
            return Err(ValidationError::EmptyEventType);
        }
        if self.event_type.len() > 64 {
            return Err(ValidationError::EventTypeTooLong);
        }
        if size_of_val(&self.payload) > 4096 {
            return Err(ValidationError::PayloadTooLarge);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("event_type cannot be empty")]
    EmptyEventType,

    #[error("event_type must be <= 64 chars")]
    EventTypeTooLong,

    #[error("payload exceeds 4Kb limit")]
    PayloadTooLarge,
}
