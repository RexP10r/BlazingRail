#![allow(dead_code)]

use async_trait::async_trait;
use common::EventInput;

#[derive(Debug, thiserror::Error)]
pub enum SynkError {
    #[error("Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Channel is closed")]
    ChannelClosed,
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SynkError>;
}
