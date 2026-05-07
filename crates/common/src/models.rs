use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("event_type cannot be empty")]
    EmptyEventType,

    #[error("event_type must be <= 64 chars")]
    EventTypeTooLong,

    #[error("payload exceeds 4Kb limit")]
    PayloadTooLarge,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventInput {
    pub event_type: String,
    pub payload: Box<RawValue>,
}

impl EventInput {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.payload.get().len() > 4096 {
            return Err(ValidationError::PayloadTooLarge);
        }
        if self.event_type.is_empty() {
            return Err(ValidationError::EmptyEventType);
        }
        if self.event_type.len() > 64 {
            return Err(ValidationError::EventTypeTooLong);
        }
        Ok(())
    }
}
