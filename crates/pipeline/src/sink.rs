use async_trait::async_trait;
use common::EventInput;

#[derive(Debug, thiserror::Error)]
pub enum SinkError {
    #[error("Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Channel is closed")]
    ChannelClosed,

    #[error("Mutex poisoned")]
    MutexPoisoned,

    #[error("Blocking task failed: {0}")]
    TaskFailed(String),
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError>;
}
