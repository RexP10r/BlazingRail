use std::{
    fs::File,
    io::{BufWriter, Error, Write},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use common::{EventInput, PipelineConfig};
use serde_json::to_writer;

use crate::{EventSink, SinkError};

pub struct FileSynk {
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl FileSynk {
    pub fn new(pipeline_config: Arc<PipelineConfig>) -> Result<Self, Error> {
        let file = File::create(pipeline_config.dlq_path.clone())?;
        let buf_writer = BufWriter::with_capacity(pipeline_config.batch_size, file);
        Ok(Self {
            writer: Arc::new(Mutex::new(buf_writer)),
        })
    }
}

#[async_trait]
impl EventSink for FileSynk {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        let mut guard = self.writer.lock().unwrap();
        for input in batch.into_iter() {
            to_writer(&mut *guard, &input)?;
            let _ = guard.write_all(b"\n");
        }
        let _ = guard.flush();
        Ok(())
    }
}
