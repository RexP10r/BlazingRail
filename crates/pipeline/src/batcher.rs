use std::{
    sync::{Arc, mpsc::Receiver},
    time::Duration,
};

use common::{EventInput, PipelineConfig};

use crate::{EventSink, SinkError};

#[allow(dead_code)]
pub struct Batcher {
    receiver: Receiver<EventInput>,
    event_sink: Arc<dyn EventSink>,
    capacity: usize,
    timeout: Duration,
}

impl Batcher {
    pub fn new(
        receiver: Receiver<EventInput>,
        event_sink: Arc<dyn EventSink>,
        pipeline_config: Arc<PipelineConfig>,
    ) -> Self {
        Self {
            receiver,
            event_sink,
            capacity: pipeline_config.batch_size,
            timeout: Duration::from_millis(pipeline_config.flush_timeout_ms),
        }
    }
    pub async fn run (&mut self) -> Result<(), SinkError> {
        Ok(())
    }
}
