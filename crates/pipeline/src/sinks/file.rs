use std::{
    fs::File,
    io::{BufWriter, Error, Write},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use common::{EventInput, PipelineConfig};
use serde_json::to_writer;

use crate::{EventSink, SinkError};

pub struct FileSink {
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl FileSink {
    pub fn new(pipeline_config: &PipelineConfig) -> Result<Self, Error> {
        let file = File::create(pipeline_config.dlq_path.clone())?;
        let buf_writer = BufWriter::with_capacity(pipeline_config.batch_size, file);

        tracing::info!(path = %pipeline_config.dlq_path.display(), "FileSink initialized");
        Ok(Self {
            writer: Arc::new(Mutex::new(buf_writer)),
        })
    }
}

#[async_trait]
impl EventSink for FileSink {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        let writer = Arc::clone(&self.writer);

        let result = tokio::task::spawn_blocking(move || {
          let mut guard = writer.lock().map_err(|_| SinkError::MutexPoisoned)?;
          for input in batch {
              to_writer(&mut *guard, &input)?;
              let _ = guard.write_all(b"\n");
          }
          guard.flush()?;
          Ok::<_, SinkError>(())
        }).await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(join_err) => Err(SinkError::TaskFailed(join_err.to_string()))
        }
    }
}
