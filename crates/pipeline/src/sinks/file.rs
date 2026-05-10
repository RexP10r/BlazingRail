use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Error, Write},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use common::{EventInput, PipelineConfig};
use serde_json::to_writer;

use crate::{EventSink, SinkError};

pub struct FileSink {
    writter: Arc<Mutex<BufWriter<File>>>,
}

impl FileSink {
    pub fn new(pipeline_config: &PipelineConfig) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(pipeline_config.prime_path.clone())?;
        let buf_writer = BufWriter::with_capacity(pipeline_config.batch_capacity, file);

        tracing::info!(path = %pipeline_config.prime_path.display(), "FileSink initialized");
        Ok(Self {
            writter: Arc::new(Mutex::new(buf_writer)),
        })
    }
    pub fn write_batch(
        writter: Arc<Mutex<BufWriter<File>>>,
        batch: Vec<EventInput>,
    ) -> Result<(), SinkError> {
        let mut guard = writter.lock().map_err(|_| SinkError::MutexPoisoned)?;
        for input in batch {
            to_writer(&mut *guard, &input)?;
            guard.write_all(b"\n")?;
        }
        guard.flush()?;
        Ok::<_, SinkError>(())
    }
}

#[async_trait]
impl EventSink for FileSink {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        let writter = Arc::clone(&self.writter);

        let result = tokio::task::spawn_blocking(move || Self::write_batch(writter, batch)).await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(join_err) => Err(SinkError::TaskFailed(join_err.to_string())),
        }
    }
}
