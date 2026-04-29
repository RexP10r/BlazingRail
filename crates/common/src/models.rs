use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, value::RawValue};
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

#[derive(Serialize, Deserialize, Debug)]
pub struct RawEventInput<'a> {
    pub event_type: String,
    #[serde(borrow)]
    pub payload: &'a RawValue,
}

impl RawEventInput<'_> {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.payload.get().len() > 4096 {
            return Err(ValidationError::PayloadTooLarge);
        }
        Ok(())
    }
}

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
        Ok(())
    }
    pub fn from_raw(input: RawEventInput) -> Result<Self, serde_json::Error> {
        Ok(Self {
            event_type: input.event_type,
            payload: from_str(input.payload.get())?,
        })
    }
}
