use async_trait::async_trait;
use common::{EventInput, PipelineConfig};
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::time::Instant;

use crate::{EventSink, SinkError};

pub struct CircuitBreaker {
    primary_sink: Arc<dyn EventSink>,
    fallback_sink: Arc<dyn EventSink>,
    timeout: Duration,
    open_until: RwLock<Option<Instant>>,
    threshold: usize,
    error_count: AtomicUsize,
}

impl CircuitBreaker {
    pub fn new(
        primary_sink: Arc<dyn EventSink>,
        fallback_sink: Arc<dyn EventSink>,
        pipeline_config: &PipelineConfig,
    ) -> Self {
        tracing::info!("Circuit breaker initialized");
        Self {
            primary_sink,
            fallback_sink,
            timeout: Duration::from_millis(pipeline_config.circuit_breaker_timeout),
            open_until: RwLock::new(None),
            threshold: pipeline_config.circuit_breaker_threshold,
            error_count: AtomicUsize::new(0),
        }
    }

    fn is_open(&self) -> bool {
        match self.open_until.read().unwrap().as_ref() {
            Some(deadline) => Instant::now() < *deadline,
            None => false,
        }
    }

    fn open_circuit(&self) {
        *self.open_until.write().unwrap() = Some(Instant::now() + self.timeout);
        tracing::error!("Primary sink failed, circuit opened");
    }

    fn reset_circuit(&self) {
        let mut guard = self.open_until.write().unwrap();
        let was_open = guard.is_some();
        *guard = None;
        if was_open {
            tracing::info!("Circuit breaker CLOSED — resumed primary sink");
        }
        self.error_count.store(0, Ordering::Release);
    }

    fn record_failure(&self) {
        let prev = self.error_count.fetch_add(1, Ordering::AcqRel);
        tracing::warn!(
            current_count = prev + 1,
            threshold = self.threshold,
            "circuit breaker failure recorded"
        );
        if prev + 1 >= self.threshold {
            self.open_circuit();
        }
    }
}

#[async_trait]
impl EventSink for CircuitBreaker {
    async fn send_batch(&self, batch: Vec<EventInput>) -> Result<(), SinkError> {
        if self.is_open() {
            return self
                .fallback_sink
                .send_batch(batch)
                .await
                .inspect_err(|e| tracing::error!(error=%e, "fallback sink also failed"));
        }

        match self.primary_sink.send_batch(batch).await {
            Ok(()) => {
                if self.error_count.load(Ordering::Relaxed) != 0 {
                    self.reset_circuit();
                }
                Ok(())
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }
}
