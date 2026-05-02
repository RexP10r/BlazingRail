use common::{EventInput, PipelineConfig};
use std::{mem::take, pin::Pin, sync::Arc, time::Duration};
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
        pipeline_config: Arc<PipelineConfig>,
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

impl Batcher {
    async fn flush(&self, state: &mut BatcherState) -> Result<(), SinkError> {
        let batch = take(&mut state.buffer);
        self.event_sink.send_batch(batch).await?;
        state.buffer.clear();
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
                  let is_stopping = self.handle_recv(&mut state, msg).await?;
                  if is_stopping {break;}
              },
              _ = &mut state.timer  => self.handle_timeout(&mut state).await?,
            }
        }
        Ok(())
    }
}
