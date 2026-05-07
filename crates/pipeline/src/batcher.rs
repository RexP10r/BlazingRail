use common::{EventInput, PipelineConfig};
use std::{mem::replace, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    sync::mpsc::Receiver,
    time::{Instant, Sleep, sleep},
};

use crate::{EventSink, SinkError};

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
        pipeline_config: &PipelineConfig,
    ) -> Self {
        Self {
            receiver,
            event_sink,
            capacity: pipeline_config.batch_size,
            timeout: Duration::from_millis(pipeline_config.flush_timeout_ms),
        }
    }
}

struct BatcherState {
    buffer: Vec<EventInput>,
    timer: Pin<Box<Sleep>>,
}

impl BatcherState {
    pub fn new(batcher: &Batcher) -> Self {
        Self {
            buffer: Vec::with_capacity(batcher.capacity),
            timer: Box::pin(sleep(batcher.timeout)),
        }
    }
}

fn record_batch_metrics(batch_size: usize, flush_duration: Duration) {
    metrics::histogram!("blazingrail_batch_size").record(batch_size as f64);
    metrics::histogram!("blazingrail_batch_flush_duration_seconds").record(flush_duration.as_secs_f64());
}

impl Batcher {
    async fn flush(&self, state: &mut BatcherState) -> Result<(), SinkError> {
        let start = Instant::now();

        let batch = replace(&mut state.buffer, Vec::with_capacity(self.capacity));

        let batch_size = batch.len();

        self.event_sink
            .send_batch(batch)
            .await
            .inspect_err(|e| tracing::error!(error = %e, "sink write failed"))?;

        let duration = start.elapsed();
        record_batch_metrics(batch_size, duration);
        Ok(())
    }
    async fn handle_recv(
        &self,
        state: &mut BatcherState,
        msg: Option<EventInput>,
    ) -> Result<bool, SinkError> {
        match msg {
            Some(input) => {
                state.buffer.push(input);
                if state.buffer.len() >= self.capacity {
                    self.flush(state).await?;
                }
                state.timer.as_mut().reset(Instant::now() + self.timeout);
                Ok(false)
            }
            None => {
                if !state.buffer.is_empty() {
                    self.flush(state).await?;
                }
                Ok(true)
            }
        }
    }
    async fn handle_timeout(&self, state: &mut BatcherState) -> Result<(), SinkError> {
        if !state.buffer.is_empty() {
            self.flush(state).await?;
        }
        state.timer.as_mut().reset(Instant::now() + self.timeout);
        Ok(())
    }
    pub async fn run(mut self) -> Result<(), SinkError> {
        let mut state = BatcherState::new(&self);
        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    match self.handle_recv(&mut state, msg).await {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => {
                            tracing::error!(error = %e, "sink error, dropping batch and continuing");
                            state.buffer.clear();
                            state.timer.as_mut().reset(Instant::now() + self.timeout);
                        }
                    }
                },
                _ = &mut state.timer => {
                    if let Err(e) = self.handle_timeout(&mut state).await {
                        tracing::error!(error = %e, "timeout flush failed");
                        state.buffer.clear();
                    }
                }
            }
        }
        Ok(())
    }
}
